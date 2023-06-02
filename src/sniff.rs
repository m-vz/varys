use chrono::{DateTime, Utc};
use log::{info, trace};
use pcap::{Capture, ConnectionStatus, Device, Packet, PacketCodec, TimestampType};
use std::fmt::{Display, Formatter};
use std::time::{Duration, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No default network device was found.")]
    DefaultDeviceNotFound,
    #[error("Could not find device {0}.")]
    DeviceNotFound(String),
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
            "{} bytes captured on {}: {:?}",
            length,
            self.timestamp.format("%d.%m.%Y %H:%M:%S"),
            self.data
        )
    }
}

pub struct Codec;

impl PacketCodec for Codec {
    type Item = SnifferPacket;

    fn decode(&mut self, packet: Packet) -> Self::Item {
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
    pub fn start(&self) -> Result<(), Error> {
        info!("{} starting...", self);

        let capture = Capture::from_device(self.device.clone())?
            .promisc(true)
            .immediate_mode(true)
            .buffer_size(100_000_000)
            .tstamp_type(TimestampType::Adapter)
            .open()?;

        for packet in capture.iter(Codec) {
            trace!("{}", packet?);
        }

        Ok(())
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
