use pcap::{ConnectionStatus, Device};
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
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

pub fn default_device() -> Result<Option<Device>, Error> {
    Ok(Device::lookup()?)
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
