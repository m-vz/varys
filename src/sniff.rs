use chrono::{DateTime, Utc};
use log::{info, trace};
use pcap::{Capture, ConnectionStatus, Device, Packet, Stat};
use std::fmt::{Display, Formatter};
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No default network device was found.")]
    DefaultDeviceNotFound,
    #[error("Could not find device {0}.")]
    DeviceNotFound(String),
    #[error("Tried to stop sniffer that was not running.")]
    CannotStop,
    #[error("Did not receive sniffer stats.")]
    NoStatsReceived,
    #[error(transparent)]
    Pcap(#[from] pcap::Error),
}

pub struct SnifferPacket {
    pub timestamp: DateTime<Utc>,
    pub len: u32,
    pub captured_len: u32,
    pub data: Box<[u8]>,
}

impl Display for SnifferPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let length = if self.captured_len == self.len {
            self.len.to_string()
        } else {
            format!("{}/{}", self.captured_len, self.len)
        };

        write!(
            f,
            "{} bytes captured on {}: {}",
            length,
            self.timestamp.format("%d.%m.%Y %H:%M:%S"),
            self.data
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<String>>()
                .join(" ")
        )
    }
}

impl From<Packet<'_>> for SnifferPacket {
    fn from(packet: Packet) -> Self {
        let timestamp = packet.header.ts;
        let s = timestamp.tv_sec as u64;
        let ms = u64::try_from(timestamp.tv_usec as i64).unwrap_or(0); // tv_usec might be negative for dates before 1970, ignore those
        let timestamp =
            DateTime::from(UNIX_EPOCH + Duration::from_secs(s) + Duration::from_micros(ms));
        SnifferPacket {
            timestamp,
            len: packet.header.len,
            captured_len: packet.header.caplen,
            data: packet.data.into(),
        }
    }
}

pub struct Sniffer {
    device: Device,
}

impl Sniffer {
    pub fn start(&self) -> Result<SnifferInstance, Error> {
        info!("{} starting...", self);

        let mut capture = Capture::from_device(self.device.clone())?
            .promisc(true)
            .immediate_mode(true)
            .buffer_size(100_000_000)
            .open()?
            .setnonblock()?;
        let (sender, receiver) = channel();

        let join_handle = thread::spawn(move || {
            while receiver.try_recv() == Err(TryRecvError::Empty) {
                match capture.next_packet() {
                    Ok(packet) => {
                        trace!("{}", SnifferPacket::from(packet));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }

            capture.stats().map_err(Error::Pcap)
        });

        Ok(SnifferInstance {
            sender,
            join_handle,
        })
    }

    pub fn run_for(&self, seconds: u64) -> Result<SnifferStats, Error> {
        let instance = self.start()?;
        thread::sleep(Duration::from_secs(seconds));
        let stats = instance.stop()?;

        Ok(stats)
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

pub struct SnifferInstance {
    sender: Sender<()>,
    join_handle: JoinHandle<Result<Stat, Error>>,
}

impl SnifferInstance {
    pub fn stop(self) -> Result<SnifferStats, Error> {
        self.sender.send(()).map_err(|_| Error::CannotStop)?;
        self.join_handle
            .join()
            .map_err(|_| Error::NoStatsReceived)?
            .map(SnifferStats::from)
    }
}

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

pub fn default_device() -> Result<Device, Error> {
    Device::lookup()?.ok_or(Error::DefaultDeviceNotFound)
}

pub fn all_devices() -> Result<Vec<Device>, Error> {
    Ok(Device::list()?)
}

pub fn devices_with_status(status: &ConnectionStatus) -> Result<Vec<Device>, Error> {
    Ok(all_devices()?
        .into_iter()
        .filter(|device| device.flags.connection_status == *status)
        .collect())
}

pub fn device_by_name(name: &str) -> Result<Device, Error> {
    all_devices()?
        .into_iter()
        .find(|device| device.name == name)
        .ok_or(Error::DeviceNotFound(name.to_string()))
}
