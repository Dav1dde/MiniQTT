use crate::{
    protocol::{
        Packet, PacketError, PacketWrite, Parse, ParseError,
        types::{EncodedStr, FixedHeader, VariableByteInteger},
    },
    utils::Cursor,
};

#[derive(Debug)]
pub struct Connect {}

impl Packet for Connect {
    const TYPE: u8 = 0b0001;
}

impl PacketWrite for Connect {
    async fn write<T>(&self, mut sink: T) -> Result<(), T::Error>
    where
        T: embedded_io_async::Write,
    {
        // Fixed Header:
        sink.write_all(&[Self::TYPE << 4]).await?;
        sink.write_all(VariableByteInteger::from(20u8).as_slice())
            .await?;

        // Protocol Name:
        sink.write_all(&[0, 4, b'M', b'Q', b'T', b'T']).await?;

        // Protocol Version:
        sink.write_all(&[5]).await?;

        // Connect Flags:
        sink.write_all(&[0]).await?;

        // Keep Alive:
        sink.write_all(&[0, 0]).await?;

        // Properties:
        sink.write_all(VariableByteInteger::from(0u8).as_slice())
            .await?;

        // Payload:
        sink.write_all(&7u16.to_be_bytes()).await?;
        sink.write_all(b"miniqtt").await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ConnAck {}

impl Packet for ConnAck {
    const TYPE: u8 = 0b0010;
}

impl<'a> Parse<'a> for ConnAck {
    type Error = PacketError;

    fn parse(data: &[u8]) -> Result<(usize, Self), ParseError<PacketError>> {
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

#[derive(Debug)]
pub struct Disconnect {}

impl Packet for Disconnect {
    const TYPE: u8 = 0b1110;
}

impl PacketWrite for Disconnect {
    async fn write<T>(&self, mut sink: T) -> Result<(), T::Error>
    where
        T: embedded_io_async::Write,
    {
        // Fixed Header:
        sink.write_all(&[Self::TYPE << 4]).await?;
        sink.write_all(VariableByteInteger::from(1u8).as_slice())
            .await?;

        // Reason Code:
        sink.write_all(&[0x04]).await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Subscribe<'a> {
    pub identifier: u16,
    pub topics: &'a [(&'a str, u8, bool)],
}

impl Packet for Subscribe<'_> {
    const TYPE: u8 = 0b1000;
}

impl PacketWrite for Subscribe<'_> {
    async fn write<T>(&self, mut sink: T) -> Result<(), T::Error>
    where
        T: embedded_io_async::Write,
    {
        // Fixed Header:
        sink.write_all(&[Self::TYPE << 4 | 0b0010]).await?;

        let mut length = 2 + 1;
        for (topic, _, _) in self.topics {
            length += 2 + topic.len() as u16 + 1;
        }

        sink.write_all(VariableByteInteger::from(length).as_slice())
            .await?;

        // Identifier:
        sink.write_all(&self.identifier.to_be_bytes()).await?;

        // Properties:
        sink.write_all(&[0]).await?;

        // Payload:
        for (topic, qos, no_local) in self.topics {
            EncodedStr(topic).write_to(&mut sink).await?;
            // TODO: there are more options, like retain etc.
            let options = (qos & 0b11) | u8::from(*no_local) << 2;
            sink.write_all(&[options]).await?;
        }

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

#[derive(Debug)]
pub struct Publish<'a> {
    pub dup: bool,
    pub qos: u8,
    pub retain: bool,
    pub identifier: Option<u16>,
    pub topic: &'a str,
    pub body: &'a [u8],
}

impl Packet for Publish<'_> {
    const TYPE: u8 = 0b0011;
}

impl<'a> Parse<'a> for Publish<'a> {
    type Error = PacketError;

    fn parse(data: &'a [u8]) -> Result<(usize, Self), ParseError<Self::Error>> {
        let mut cursor = Cursor::new(data);

        let fixed_header = cursor.read::<FixedHeader>()?;
        if fixed_header.ty() != Self::TYPE {
            return Err(PacketError::InvalidType {
                expected: Self::TYPE,
                actual: fixed_header.ty(),
            }
            .into());
        }

        let dup = fixed_header.flags() & 0b1000 > 0;
        // TODO: QoS malformed packet
        let qos = (fixed_header.flags() >> 1) & 0b11;
        let retain = fixed_header.flags() & 0b0001 > 0;

        let packet_length = fixed_header.length().as_u32() as usize;
        let start_length = cursor.position();

        let EncodedStr(topic) = cursor.read()?;

        let identifier = match qos {
            0 => None,
            _ => Some(cursor.read_u16_be()?),
        };

        let properties = cursor
            .read::<VariableByteInteger>()
            .map_err(|err| err.map(|_| PacketError::ProtocolError))?;
        let _ = cursor.read_slice(properties.as_u32() as usize)?;

        // TODO: we might want some length validations here.
        let body_len = packet_length - (cursor.position() - start_length);
        let body = cursor.read_slice(body_len)?;

        Ok((
            cursor.position(),
            Self {
                dup,
                qos,
                identifier,
                retain,
                topic,
                body,
            },
        ))
    }
}
