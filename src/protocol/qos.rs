use core::fmt;

/// Quality of Service levels.
///
/// Spec: [4.3](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901234)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum QoS {
    /// QoS 0: At most once delivery.
    ///
    /// The message is delivered according to the capabilities of the underlying network.
    /// No response is sent by the receiver and no retry is performed by the sender.
    /// The message arrives at the receiver either once or not at all.
    ///
    /// Spec: [4.3.1](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901235)
    AtMostOnce = 0,
    /// QoS 1: At least once delivery.
    ///
    /// This Quality of Service level ensures that the message arrives at the receiver at least once.
    ///
    /// Spec: [4.3.1](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901236)
    AtLeastOnce = 1,
    /// QoS 2: Exactly once delivery.
    ///
    /// This is the highest Quality of Service level, for use when neither loss nor duplication
    /// of messages are acceptable. There is an increased overhead associated with QoS 2.
    ///
    /// Spec: [4.3.1](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901237)
    ExactlyOnce = 2,
}

impl From<QoS> for u8 {
    fn from(value: QoS) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for QoS {
    type Error = InvalidQoS;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::AtMostOnce),
            1 => Ok(Self::AtLeastOnce),
            2 => Ok(Self::ExactlyOnce),
            v => Err(InvalidQoS(v)),
        }
    }
}

/// Error when attempting to create an invalid [`QoS`] level.
#[derive(Debug)]
pub struct InvalidQoS(u8);

impl fmt::Display for InvalidQoS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid QoS, expected 0, 1, 2 got '{}'", self.0)
    }
}

impl core::error::Error for InvalidQoS {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qos_convert() {
        assert_eq!(u8::from(QoS::AtMostOnce), 0);
        assert_eq!(QoS::try_from(0).unwrap(), QoS::AtMostOnce);
        assert_eq!(u8::from(QoS::AtLeastOnce), 1);
        assert_eq!(QoS::try_from(1).unwrap(), QoS::AtLeastOnce);
        assert_eq!(u8::from(QoS::ExactlyOnce), 2);
        assert_eq!(QoS::try_from(2).unwrap(), QoS::ExactlyOnce);

        for i in 3..u8::MAX {
            assert!(
                QoS::try_from(i).is_err(),
                "'{i}' should not be convertable to a QoS"
            );
        }
    }
}
