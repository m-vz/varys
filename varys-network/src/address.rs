use std::fmt::{Display, Formatter};
use std::str::FromStr;

use pnet::util::MacAddr;

use crate::error::Error;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MacAddress(pub u8, pub u8, pub u8, pub u8, pub u8, pub u8);

impl From<MacAddress> for MacAddr {
    fn from(value: MacAddress) -> Self {
        MacAddr::new(value.0, value.1, value.2, value.3, value.4, value.5)
    }
}

impl From<MacAddr> for MacAddress {
    fn from(value: MacAddr) -> Self {
        MacAddress(value.0, value.1, value.2, value.3, value.4, value.5)
    }
}

impl FromStr for MacAddress {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(MacAddr::from_str(s)?.into())
    }
}

impl Display for MacAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0, self.1, self.2, self.3, self.4, self.5
        )
    }
}
