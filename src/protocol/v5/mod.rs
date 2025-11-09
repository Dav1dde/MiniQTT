use crate::protocol::types::{EncodedStr, FixedHeader, VariableByteInteger};
use crate::protocol::{Packet, PacketError, Parse, ParseError, QoS};
use crate::traits::Writable;
use crate::utils::Cursor;

pub mod connect;
pub mod property;
pub mod publish;

pub use self::connect::{ConnAck, Connect};
pub use self::property::Property;
pub use self::publish::Publish;

#[derive(Debug)]
pub struct Disconnect {}

impl Packet for Disconnect {
    const TYPE: u8 = 0b1110;
}

impl Writable for Disconnect {
    type Error<E> = E;

    fn size(&self) -> usize {
        1
    }

    async fn write_to<T>(&self, mut sink: T) -> Result<(), T::Error>
    where
        T: embedded_io_async::Write,
    {
        // Reason Code:
        sink.write_all(&[0x04]).await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Subscribe<'a> {
    pub identifier: u16,
    // TODO: properties
    pub topics: &'a [TopicFilter<'a>],
}

impl Packet for Subscribe<'_> {
    const TYPE: u8 = 0b1000;

    fn flags(&self) -> u8 {
        0b0010
    }
}

impl Writable for Subscribe<'_> {
    type Error<E> = E;

    fn size(&self) -> usize {
        self.identifier.size() + 1 + self.topics.size()
    }

    async fn write_to<T>(&self, mut sink: T) -> Result<(), T::Error>
    where
        T: embedded_io_async::Write,
    {
        // Identifier:
        self.identifier.write_to(&mut sink).await?;

        // Properties:
        VariableByteInteger::from(0u8).write_to(&mut sink).await?;

        // Payload:
        self.topics.write_to(&mut sink).await?;

        Ok(())
    }
}

/// A topic filter, used to [subscribe](Subscribe) to topics.
///
/// Spec: [3.8.3](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901168).
#[derive(Debug)]
pub struct TopicFilter<'a> {
    /// The name of the topic.
    pub name: &'a str,
    /// The Quality of Service level.
    pub qos: QoS,
    /// No Local option.
    ///
    /// If the value is true, Application Messages MUST NOT be forwarded to a connection with a
    /// client id equal to the client id of the publishing connection.
    pub no_local: bool,
    /// Retain as Published option.
    ///
    /// If `true`, Application Messages forwarded using this subscription keep the RETAIN flag they
    /// were published with. If `false`, Application Messages forwarded using this subscription have
    /// the RETAIN flag set to `false`. Retained messages sent when the subscription is established have
    /// the RETAIN flag set to `true.
    pub retain_as_published: bool,
    /// Retain handling.
    ///
    /// This option specifies whether retained messages are sent when the subscription is established.
    /// This does not affect the sending of retained messages at any point after the subscribe.
    /// If there are no retained messages matching the Topic Filter, all of these values act the same.
    pub retain: RetainHandling,
}

/// [Topic filter](TopicFilter::retain) retain handling.
#[derive(Debug, Default)]
#[repr(u8)]
pub enum RetainHandling {
    /// Send retained messages at the time of the subscribe.
    #[default]
    SendRetained = 0,
    /// Send retained messages at subscribe only if the subscription does not currently exist.
    SendRetainedOnNewSubscription = 1,
    /// Do not send retained messages at the time of the subscribe.
    DoNotSendRetained = 3,
}

impl Writable for TopicFilter<'_> {
    type Error<E> = E;

    fn size(&self) -> usize {
        EncodedStr(self.name).size() + 1
    }

    async fn write_to<S>(&self, mut sink: S) -> Result<(), Self::Error<S::Error>>
    where
        S: embedded_io_async::Write,
    {
        EncodedStr(self.name).write_to(&mut sink).await?;

        let options = (self.retain_as_published as u8) << 4
            | u8::from(self.retain_as_published) << 3
            | u8::from(self.no_local) << 2
            | u8::from(self.qos);
        options.write_to(&mut sink).await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct SubAck {}

impl Packet for SubAck {
    const TYPE: u8 = 0b1001;
}

impl<'a> Parse<'a> for SubAck {
    type Error = PacketError;

    fn parse(data: &[u8]) -> Result<(usize, Self), ParseError<Self::Error>> {
        let mut cursor = Cursor::new(data);

        let fixed_header = cursor.read::<FixedHeader>()?;
        if fixed_header.ty() != Self::TYPE {
            return Err(PacketError::InvalidType {
                expected: Self::TYPE,
                actual: fixed_header.ty(),
            }
            .into());
        }

        let _ = cursor.read_slice(fixed_header.length().as_u32() as usize)?;

        Ok((cursor.position(), Self {}))
    }
}
