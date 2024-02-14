use std::fmt::{Debug, Display, Formatter};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

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

    pub fn as_binary_trace(&self, relative_to: &MacAddress) -> BinaryTrafficTrace {
        BinaryTrafficTrace(
            self.packets
                .iter()
                .filter_map(|packet| packet.direction(relative_to))
                .map(|direction| direction.into())
                .collect(),
        )
    }

    pub fn as_numeric_trace(&self, relative_to: &MacAddress) -> NumericTrafficTrace {
        NumericTrafficTrace(
            self.packets
                .iter()
                .filter_map(|packet| {
                    packet
                        .direction(relative_to)
                        .map(|direction| f32::from(direction) * packet.len as f32)
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct BinaryTrafficTrace(pub Vec<bool>);

impl BinaryTrafficTrace {
    /// Resize the trace, truncating if it is longer than `len` and adding zeroes if it is shorter.
    ///
    /// # Arguments
    ///
    /// * `len`: The new length of the trace.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_analysis::trace::BinaryTrafficTrace;
    /// let mut trace = BinaryTrafficTrace(vec![true, false, true]);
    ///
    /// trace.resize(2);
    /// assert_eq!(trace, BinaryTrafficTrace(vec![true, false]));
    /// trace.resize(4);
    /// assert_eq!(trace, BinaryTrafficTrace(vec![true, false, false, false]));
    /// ```
    pub fn resize(&mut self, len: usize) {
        self.0.resize(len, false);
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct NumericTrafficTrace(pub Vec<f32>);

impl NumericTrafficTrace {
    /// Resize the trace, truncating if it is longer than `len` and adding zeroes if it is shorter.
    ///
    /// # Arguments
    ///
    /// * `len`: The new length of the trace.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_analysis::trace::NumericTrafficTrace;
    /// let mut trace = NumericTrafficTrace(vec![1., 2., 3.]);
    ///
    /// trace.resize(2);
    /// assert_eq!(trace, NumericTrafficTrace(vec![1., 2.]));
    /// trace.resize(4);
    /// assert_eq!(trace, NumericTrafficTrace(vec![1., 2., 0., 0.]));
    /// ```
    pub fn resize(&mut self, len: usize) {
        self.0.resize(len, 0.);
    }

    /// Get the minimum and maximum value of this trace.
    pub fn min_max(&self) -> (f32, f32) {
        self.0
            .iter()
            .fold((f32::MAX, f32::MIN), |(min, max), &value| {
                (min.min(value), max.max(value))
            })
    }

    /// Scale the whole trace by the given factor.
    ///
    /// # Arguments
    ///
    /// * `scale`: The factor to scale the trace by.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_analysis::trace::NumericTrafficTrace;
    /// let mut trace = NumericTrafficTrace(vec![1., 2., 3.]);
    ///
    /// trace.resize(2);
    /// assert_eq!(trace, NumericTrafficTrace(vec![1., 2.]));
    /// ```
    pub fn scale(&mut self, scale: f32) {
        self.0.iter_mut().for_each(|value| *value *= scale);
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
