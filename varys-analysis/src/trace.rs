use std::fmt::{Debug, Display, Formatter};

use chrono::{DateTime, Duration, Utc};

use varys_network::address::MacAddress;
use varys_network::packet::Packet;

use crate::error::Error;

pub struct TrafficTrace {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub packets: Vec<Packet>,
}

impl TrafficTrace {
    pub fn duration(&self) -> Duration {
        self.end_time - self.start_time
    }

    pub fn as_binary_trace(&self, relative_to: MacAddress) -> BinaryTrafficTrace {
        BinaryTrafficTrace(
            self.packets
                .iter()
                .filter_map(|packet| packet.direction(relative_to))
                .map(|direction| direction.into())
                .collect(),
        )
    }

    pub fn as_numeric_trace(&self, relative_to: MacAddress) -> NumericTrafficTrace {
        NumericTrafficTrace(
            self.packets
                .iter()
                .filter_map(|packet| {
                    packet
                        .direction(relative_to)
                        .map(|direction| i32::from(direction) * packet.len as i32)
                })
                .collect(),
        )
    }
}

impl TryFrom<Vec<Packet>> for TrafficTrace {
    type Error = Error;

    fn try_from(mut packets: Vec<Packet>) -> Result<Self, Self::Error> {
        packets.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let start_time = packets.first().ok_or(Error::EmptyTrace)?.timestamp;
        let end_time = packets.last().ok_or(Error::EmptyTrace)?.timestamp;

        Ok(Self {
            start_time,
            end_time,
            packets,
        })
    }
}

impl Display for TrafficTrace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Trace from {} to {} ({:.2} seconds, {} packets)",
            self.start_time.format("%d.%m.%Y %H:%M:%S"),
            self.end_time.format("%d.%m.%Y %H:%M:%S"),
            self.duration().num_milliseconds() as f32 / 1000.,
            self.packets.len()
        )
    }
}

#[derive(Debug)]
pub struct BinaryTrafficTrace(pub Vec<bool>);

impl BinaryTrafficTrace {
    pub fn resized(&mut self, len: usize) -> &mut Self {
        self.0.resize(len, false);

        self
    }
}

impl Display for BinaryTrafficTrace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Binary trace of {} packets ({}, ...)",
            self.0.len(),
            self.0
                .iter()
                .take(6)
                .map(|&packet| if packet { "1" } else { "-1" })
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Debug)]
pub struct NumericTrafficTrace(pub Vec<i32>);

impl NumericTrafficTrace {
    pub fn resized(&mut self, len: usize) -> &mut Self {
        self.0.resize(len, 0);

        self
    }
}

impl Display for NumericTrafficTrace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Numeric trace of {} packets ({}, ...)",
            self.0.len(),
            self.0
                .iter()
                .take(6)
                .map(|packet| packet.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}
