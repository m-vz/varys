use std::fmt::{Display, Formatter};

use chrono::{DateTime, Duration, Utc};

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
