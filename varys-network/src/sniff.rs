use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::time::Duration;
use std::{thread, thread::JoinHandle};

use log::{info, trace};
pub use pcap::ConnectionStatus;
use pcap::{Capture, Device, Stat};

use crate::error::Error;
use crate::packet::Packet;

/// A sniffer is used to capture network packets on a specific network device.
pub struct Sniffer {
    device: Device,
}

impl Sniffer {
    /// Start sniffing on this device.
    ///
    /// This requires root privileges to access the network devices, otherwise an error is returned.
    /// This also returns an error if a `file_path` was provided which could not be written to.
    ///
    /// # Arguments
    ///
    /// * `file_path`: The path to which the captured traffic is written. The extension `.pcap` will
    /// be added if it isn't already in the path.
    ///
    /// Returns a [`SnifferInstance`], on which [`SnifferInstance::stop`] can be called to stop
    /// capturing the traffic.
    ///
    /// # Examples
    ///
    /// Start sniffer, writing to `capture.pcap`:
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use varys_network::sniff;
    /// # use varys_network::sniff::Sniffer;
    /// let sniffer = Sniffer::from(sniff::default_device().unwrap());
    ///
    /// let instance = sniffer.start(Path::new("/path/to/capture.pcap")).unwrap();
    /// # instance.stop().unwrap();
    /// ```
    pub fn start(&self, file_path: &Path) -> Result<SnifferInstance, Error> {
        let mut file_path = file_path.to_owned();
        file_path.set_extension("pcap");

        info!("{} starting (writing to {:?})...", self, file_path);

        let mut capture = Capture::from_device(self.device.clone())?
            .promisc(true)
            .immediate_mode(true)
            .buffer_size(100_000_000)
            .open()?
            .setnonblock()?;
        let mut file = capture.savefile(file_path)?;
        let (shutdown_channel, receiver) = channel();

        let join_handle = thread::spawn(move || {
            while receiver.try_recv() == Err(TryRecvError::Empty) {
                match capture.next_packet() {
                    Ok(packet) => {
                        file.write(&packet);
                        trace!("{}", Packet::from(packet));
                    }
                    Err(_) => thread::sleep(Duration::from_millis(10)),
                }
            }

            capture.stats().map_err(Error::from)
        });

        Ok(SnifferInstance {
            shutdown_channel,
            join_handle,
        })
    }

    /// Run a sniffer for a specified amount of seconds and stop it automatically afterwards. The
    /// current thread is blocked until the sniffer is done.
    ///
    /// This requires root privileges to access the network devices, otherwise an error is returned.
    /// This also returns an error if a `file_path` was provided which could not be written to.
    ///
    /// # Arguments
    ///
    /// * `seconds`: How many seconds to capture traffic for.
    /// * `file_path`: The path to which the captured traffic is written. The extension `.pcap` will
    /// be added if it isn't already in the path.
    ///
    /// Returns [`SnifferStats`] with statistics about the capture.
    ///
    /// # Examples
    ///
    /// Capture traffic for 5 seconds, writing to `capture.pcap`:
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use varys_network::sniff;
    /// # use varys_network::sniff::Sniffer;
    /// let sniffer = Sniffer::from(sniff::default_device().unwrap());
    ///
    /// let stats = sniffer.run_for(5, Path::new("capture.pcap")).unwrap();
    /// ```
    pub fn run_for(&self, seconds: u64, file_path: &Path) -> Result<SnifferStats, Error> {
        info!("Running sniffer for {seconds} seconds...");

        let instance = self.start(file_path)?;

        thread::sleep(Duration::from_secs(seconds));

        instance.stop()
    }
}

impl From<Device> for Sniffer {
    fn from(device: Device) -> Self {
        Sniffer { device }
    }
}

impl Display for Sniffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sniffer on {} ({:?} | {:?})",
            self.device.name, self.device.flags.connection_status, self.device.flags.if_flags
        )
    }
}

/// A handle to a running sniffer instance. It can be stopped with [`SnifferInstance::stop`].
pub struct SnifferInstance {
    shutdown_channel: Sender<()>,
    join_handle: JoinHandle<Result<Stat, Error>>,
}

impl SnifferInstance {
    /// Stop the running sniffer consuming the instance and get the statistics from the run.
    ///
    /// Returns [`SnifferStats`] with statistics about the capture.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use varys_network::sniff;
    /// # use varys_network::sniff::Sniffer;
    /// let sniffer = Sniffer::from(sniff::default_device().unwrap());
    /// let instance = sniffer.start(Path::new("capture.pcap")).unwrap();
    ///
    /// let stats = instance.stop().unwrap();
    /// ```
    pub fn stop(self) -> Result<SnifferStats, Error> {
        info!("Sniffer stopping");

        self.shutdown_channel
            .send(())
            .map_err(|_| Error::CannotStop)?;
        self.join_handle
            .join()
            .map_err(|_| Error::NoStatsReceived)?
            .map(SnifferStats::from)
    }
}

/// Statistics about a finished capture.
///
/// `received` is the number of packets received in total.
///
/// `buffer_dropped` is the number of packets dropped because the buffer for incoming packets was
/// too small or packets were not processed quickly enough.
///
/// `interface_dropped` is the number of packets dropped by the network interface.
#[derive(Debug)]
pub struct SnifferStats {
    pub received: u32,
    pub buffer_dropped: u32,
    pub interface_dropped: u32,
}

impl From<Stat> for SnifferStats {
    fn from(stats: Stat) -> Self {
        SnifferStats {
            received: stats.received,
            buffer_dropped: stats.dropped,
            interface_dropped: stats.if_dropped,
        }
    }
}

impl Display for SnifferStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Captured {} packets (buffer dropped: {}, interface dropped: {})",
            self.received, self.buffer_dropped, self.interface_dropped
        )
    }
}

/// Get all network devices.
///
/// Returns an error if device information could not be retrieved.
///
/// # Examples
///
/// ```
/// # use pcap::ConnectionStatus;
/// # use varys_network::sniff;
/// let devices = sniff::all_devices().unwrap();
/// ```
pub fn all_devices() -> Result<Vec<Device>, Error> {
    Ok(Device::list()?)
}

/// Get all network devices with a certain connection status.
///
/// Returns an error if device information could not be retrieved.
///
/// # Arguments
///
/// * `status`: The status to filter the devices by.
///
/// # Examples
///
/// ```
/// # use pcap::ConnectionStatus;
/// # use varys_network::sniff;
/// let connected_devices = sniff::devices_with_status(&ConnectionStatus::Connected).unwrap();
/// ```
pub fn devices_with_status(status: &ConnectionStatus) -> Result<Vec<Device>, Error> {
    Ok(all_devices()?
        .into_iter()
        .filter(|device| device.flags.connection_status == *status)
        .collect())
}

/// Get the system default network device suitable for network capture.
///
/// Returns an error if no default device was found or device information could not be retrieved.
///
/// # Examples
///
/// ```
/// # use pcap::ConnectionStatus;
/// # use varys_network::sniff;
/// let default_device = sniff::default_device().unwrap();
/// ```
pub fn default_device() -> Result<Device, Error> {
    Device::lookup()?.ok_or(Error::DefaultDeviceNotFound)
}

/// Get the network device with a specific name
///
/// Returns an error if no device with the given name was found or if device information could not
/// be retrieved.
///
/// # Arguments
///
/// * `name`: The name of the device to find.
///
/// # Examples
///
/// ```
/// # use pcap::ConnectionStatus;
/// # use varys_network::error::Error;
/// # use varys_network::sniff;
/// let connected_devices = sniff::device_by_name("en0").unwrap();
/// let invalid_device = sniff::device_by_name("Invalid device name");
///
/// if let Err(Error::NetworkDeviceNotFound(name)) = invalid_device {
///     if name.as_str() == "Invalid device name" {
///         return;
///     } else {
///         panic!("Wrong error format");
///     }
/// } else {
///     panic!("Error expected");
/// }
/// ```
pub fn device_by_name(name: &str) -> Result<Device, Error> {
    all_devices()?
        .into_iter()
        .find(|device| device.name == name)
        .ok_or(Error::NetworkDeviceNotFound(name.to_string()))
}
