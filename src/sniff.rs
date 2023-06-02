use pcap::Device;
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
