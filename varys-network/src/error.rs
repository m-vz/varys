use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    InvalidMacAddress(#[from] pnet::datalink::ParseMacAddrErr),
    #[error("No default network device was found")]
    DefaultDeviceNotFound,
    #[error("Could not find device {0}")]
    NetworkDeviceNotFound(String),
    #[error("Tried to stop sniffer that was not running")]
    CannotStop,
    #[error("Did not receive sniffer stats")]
    NoStatsReceived,
    #[error("Pcap error: {0}")]
    Pcap(String),
}

impl From<pcap::Error> for Error {
    fn from(value: pcap::Error) -> Self {
        match value {
            pcap::Error::IoError(err) => std::io::Error::from(err).into(),
            _ => Error::Pcap(value.to_string()),
        }
    }
}
