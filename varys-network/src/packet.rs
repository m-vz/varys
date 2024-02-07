use std::fmt::{Display, Formatter};
use std::path::Path;
use std::time;
use std::time::Duration;

use chrono::{DateTime, Utc};
use pcap::Capture;

use crate::error::Error;

/// A sniffer packet contains all packet information for one captured pcap packet.
pub struct Packet {
    pub timestamp: DateTime<Utc>,
    /// The length of the packet, in bytes.
    ///
    /// In rare cases, this might be more than the amount of data captured.
    pub len: usize,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn captured_len(&self) -> usize {
        self.data.len()
    }
}

impl Display for Packet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let length = if self.captured_len() == self.len {
            self.len.to_string()
        } else {
            format!("{}/{}", self.captured_len(), self.len)
        };

        write!(
            f,
            "{length} bytes captured on {}",
            self.timestamp.format("%d.%m.%Y %H:%M:%S")
        )
    }
}

impl From<pcap::Packet<'_>> for Packet {
    fn from(packet: pcap::Packet) -> Self {
        let timestamp = packet.header.ts;
        let s = timestamp.tv_sec as u64;
        let us = u64::try_from(timestamp.tv_usec as i64).unwrap_or(0); // tv_usec might be negative for dates before 1970, ignore those
        let timestamp =
            DateTime::from(time::UNIX_EPOCH + Duration::from_secs(s) + Duration::from_micros(us));

        Packet {
            timestamp,
            len: packet.header.len as usize,
            data: packet.data.into(),
        }
    }
}

/// Load all packets from a pcap file.
///
/// # Arguments
///
/// * `path`: The path to the pcap file.
pub fn load_packets<P: AsRef<Path>>(path: P) -> Result<Vec<Packet>, Error> {
    let mut capture = Capture::from_file(path)?;
    let mut packets = Vec::new();

    loop {
        match capture.next_packet() {
            Ok(pcap_packet) => packets.push(Packet::from(pcap_packet)),
            Err(pcap::Error::NoMorePackets) => break,
            Err(error) => return Err(Error::from(error)),
        }
    }

    Ok(packets)
}
