use crate::{
    protocol::{
        Packet, PacketError, PacketParse, ParseResult,
        types::{BinaryData, EncodedStr, FixedHeader, QoS},
        v5::{Property, property::Properties},
    },
    traits::Writable,
    utils::{Cursor, write_many},
};

#[derive(Debug, Clone, Copy)]
pub struct Connect<'a> {
    pub client_id: &'a str,
    pub keep_alive: u16,
    pub clean_start: bool,
    pub will: Option<Will<'a>>,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
    pub properties: &'a [ConnectProperty<'a>],
}

impl Packet for Connect<'_> {
    const TYPE: u8 = 0b0001;
}

impl Writable for Connect<'_> {
    type Error<E> = E;

    fn size(&self) -> usize {
        10 + Properties(self.properties).size()
            + EncodedStr(self.client_id).size()
            + self.will.size()
            + self.username.map(EncodedStr).size()
            + self.password.map(EncodedStr).size()
    }

    async fn write_to<T>(&self, mut sink: T) -> Result<(), T::Error>
    where
        T: embedded_io_async::Write,
    {
        // Protocol Name:
        EncodedStr("MQTT").write_to(&mut sink).await?;

        // Protocol Version:
        5u8.write_to(&mut sink).await?;

        // Connect Flags:
        let connect_flags = {
            let username = u8::from(self.username.is_some());
            let password = u8::from(self.username.is_some());
            let will_retain = u8::from(self.will.is_some_and(|w| w.retain));
            let will_qos = u8::from(self.will.map(|w| w.qos).unwrap_or(QoS::AtMostOnce));
            let will = u8::from(self.will.is_some());
            let clean_start = u8::from(self.clean_start);

            username << 7
                | password << 6
                | will_retain << 5
                | will_qos << 3
                | will << 2
                | clean_start << 1
        };
        connect_flags.write_to(&mut sink).await?;

        // Keep Alive:
        self.keep_alive.write_to(&mut sink).await?;

        // Properties:
        Properties(self.properties).write_to(&mut sink).await?;

        // Payload:
        EncodedStr(self.client_id).write_to(&mut sink).await?;
        self.will.write_to(&mut sink).await?;
        self.username.map(EncodedStr).write_to(&mut sink).await?;
        self.password.map(EncodedStr).write_to(&mut sink).await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Will<'a> {
    pub retain: bool,
    pub qos: QoS,
    pub properties: &'a [WillProperty<'a>],
    pub topic: &'a str,
    pub payload: &'a [u8],
}

impl Writable for Will<'_> {
    type Error<E> = E;

    fn size(&self) -> usize {
        Properties(self.properties).size()
            + EncodedStr(self.topic).size()
            + BinaryData(self.payload).size()
    }

    async fn write_to<S>(&self, mut sink: S) -> Result<(), Self::Error<S::Error>>
    where
        S: embedded_io_async::Write,
    {
        Properties(self.properties).write_to(&mut sink).await?;
        EncodedStr(self.topic).write_to(&mut sink).await?;
        BinaryData(self.payload).write_to(&mut sink).await?;

        Ok(())
    }
}

/// Properties accepted in a [`Connect`] request.
#[derive(Debug, Clone, Copy)]
pub enum ConnectProperty<'a> {
    /// The Session Expiry Interval in seconds.
    ///
    /// Spec: [3.1.2.11.2](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901048)
    SessionExpiryInterval(u32),
    /// The Receive Maximum value.
    ///
    /// Spec: [3.1.2.11.3](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901049)
    ReceiveMaximum(u16),
    /// The Maximum Packet Size the Client is willing to accept.
    ///
    /// Spec: [3.1.2.11.4](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901050)
    MaximumPacketSize(u32),
    /// The Topic Alias Maximum value.
    ///
    /// Spec: [3.1.2.11.5](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901051)
    TopicAliasMaximum(u16),
    /// The Client uses this value to request the Server to return Response Information in the
    /// [`ConnAck`].
    ///
    /// Spec: [3.1.2.11.6](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901052)
    RequestResponseInformation(u8),
    /// The Client uses this value to indicate whether the Reason String or User Properties are
    /// sent in the case of failures.
    ///
    /// Spec: [3.1.2.11.7](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901053)
    RequestProblemInformation(bool),
    /// A custom connection related property.
    ///
    /// Spec: [3.1.2.11.8](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901054)
    UserProperty { key: &'a str, value: &'a str },
    /// Contains the name of the authentication method used for extended authentication.
    ///
    /// Spec: [3.1.2.11.9](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901055)
    AuthenticationMethod(&'a str),
    /// Binary Data containing authentication data.
    ///
    /// Spec: [3.1.2.11.9](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901056)
    AuthenticationData(&'a [u8]),
}

impl Writable for ConnectProperty<'_> {
    type Error<E> = E;

    fn size(&self) -> usize {
        let payload = match self {
            Self::SessionExpiryInterval(v) => v.size(),
            Self::ReceiveMaximum(v) => v.size(),
            Self::MaximumPacketSize(v) => v.size(),
            Self::TopicAliasMaximum(v) => v.size(),
            Self::RequestResponseInformation(v) => v.size(),
            Self::RequestProblemInformation(v) => u8::from(*v).size(),
            Self::UserProperty { key, value } => EncodedStr(key).size() + EncodedStr(value).size(),
            Self::AuthenticationMethod(v) => EncodedStr(v).size(),
            Self::AuthenticationData(v) => BinaryData(v).size(),
        };
        1 + payload
    }

    async fn write_to<S>(&self, mut sink: S) -> Result<(), Self::Error<S::Error>>
    where
        S: embedded_io_async::Write,
    {
        match self {
            Self::SessionExpiryInterval(v) => write_many!(sink, 0x11u8, v),
            Self::ReceiveMaximum(v) => write_many!(sink, 0x21u8, v),
            Self::MaximumPacketSize(v) => write_many!(sink, 0x27u8, *v),
            Self::TopicAliasMaximum(v) => write_many!(sink, 0x22u8, *v),
            Self::RequestResponseInformation(v) => write_many!(sink, 0x19u8, *v),
            Self::RequestProblemInformation(v) => write_many!(sink, 0x17u8, u8::from(*v)),
            Self::UserProperty { key, value } => {
                write_many!(sink, 0x26u8, EncodedStr(key), EncodedStr(value))
            }
            Self::AuthenticationMethod(v) => write_many!(sink, 0x15u8, EncodedStr(v)),
            Self::AuthenticationData(v) => write_many!(sink, 0x16u8, BinaryData(v)),
        }

        Ok(())
    }
}

impl Property for ConnectProperty<'_> {}

/// [`Will`] specific properties accepted in a [`Connect`] request.
#[derive(Debug)]
pub enum WillProperty<'a> {
    /// The Will Delay Interval in seconds.
    ///
    /// Spec: [3.1.3.2.2](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901062)
    WillDelay(u32),
    /// The Payload Format Indicator.
    ///
    /// Spec: [3.1.3.2.3](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901063)
    PayloadFormatIndicator(u8),
    /// The Message Expiry Interval in seconds.
    ///
    /// Spec: [3.1.3.2.4](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901064)
    MessageExpiryInterval(u32),
    /// String describing the content of the Will Message.
    ///
    /// Spec: [3.1.3.2.5](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901065)
    ContentType(&'a str),
    /// Used as the Topic Name for a response message.
    ///
    /// Spec: [3.1.3.2.6](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901066)
    ResponseTopic(&'a str),
    /// The Correlation Data is used by the sender of the Request Message to identify
    /// which request the Response Message is for when it is received.
    ///
    /// Spec: [3.1.3.2.7](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901067)
    CorrelationData(&'a [u8]),
    /// A custom connection related property.
    ///
    /// Spec: [3.1.3.2.8](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901068)
    UserProperty { key: &'a str, value: &'a str },
}

impl Writable for WillProperty<'_> {
    type Error<E> = E;

    fn size(&self) -> usize {
        let payload = match self {
            Self::WillDelay(v) => v.size(),
            Self::PayloadFormatIndicator(v) => v.size(),
            Self::MessageExpiryInterval(v) => v.size(),
            Self::ContentType(v) => EncodedStr(v).size(),
            Self::ResponseTopic(v) => EncodedStr(v).size(),
            Self::CorrelationData(v) => v.size(),
            Self::UserProperty { key, value } => EncodedStr(key).size() + EncodedStr(value).size(),
        };
        1 + payload
    }

    async fn write_to<S>(&self, mut sink: S) -> Result<(), Self::Error<S::Error>>
    where
        S: embedded_io_async::Write,
    {
        match self {
            Self::WillDelay(v) => write_many!(sink, 0x18u8, v),
            Self::PayloadFormatIndicator(v) => write_many!(sink, 0x01u8, v),
            Self::MessageExpiryInterval(v) => write_many!(sink, 0x02u8, *v),
            Self::ContentType(v) => write_many!(sink, 0x03u8, EncodedStr(v)),
            Self::ResponseTopic(v) => write_many!(sink, 0x08u8, EncodedStr(v)),
            Self::CorrelationData(v) => write_many!(sink, 0x09u8, BinaryData(v)),
            Self::UserProperty { key, value } => {
                write_many!(sink, 0x26u8, EncodedStr(key), EncodedStr(value))
            }
        }

        Ok(())
    }
}

impl Property for WillProperty<'_> {}

#[derive(Debug)]
pub struct ConnAck {}

impl Packet for ConnAck {
    const TYPE: u8 = 0b0010;
}

impl<'a> PacketParse<'a> for ConnAck {
    fn parse(data: &[u8]) -> ParseResult<(usize, Self), PacketError> {
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
