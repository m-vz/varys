use pcap::{ConnectionStatus, Device};
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

pub fn available_devices() -> Result<Vec<Device>, Error> {
    Ok(Device::list()?)
}

pub fn connected_devices() -> Result<Vec<Device>, Error> {
    Ok(available_devices()?
        .into_iter()
        .filter(|device| device.flags.connection_status == ConnectionStatus::Connected)
        .collect())
}
