use pcap::{ConnectionStatus, Device};
use std::fmt::{Display, Formatter};
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

pub struct Sniffer {
    device: Device,
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
