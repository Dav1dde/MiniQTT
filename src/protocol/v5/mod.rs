use core::fmt;

use crate::{
    protocol::{
        Packet, PacketError, Parse, ParseError,
        types::{EncodedStr, FixedHeader, VariableByteInteger},
    },
    traits::Writable,
    utils::Cursor,
};

pub mod connect;
pub mod property;

pub use self::connect::{ConnAck, Connect};
pub use self::property::Property;

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
    // TODO: probably should make the inner tuple a type
    pub topics: &'a [(&'a str, u8, bool)],
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
        self.identifier.size()
            + 1
            + self
                .topics
                .iter()
                .map(|(topic, _, _)| EncodedStr(topic).size() + 1)
                .sum::<usize>()
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
        for (topic, qos, no_local) in self.topics {
            EncodedStr(topic).write_to(&mut sink).await?;
            // TODO: there are more options, like retain etc.
            let options = (qos & 0b11) | u8::from(*no_local) << 2;
            options.write_to(&mut sink).await?;
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

pub struct Publish<'a> {
    pub dup: bool,
    pub qos: u8,
    pub retain: bool,
    pub identifier: Option<u16>,
    pub topic: &'a str,
    pub payload: &'a [u8],
}

impl fmt::Debug for Publish<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Publish {{ ")?;
        write!(f, "Q{} ", self.qos)?;
        write!(f, "D{} ", self.dup as u8)?;
        write!(f, "R{} ", self.retain as u8)?;
        match self.identifier {
            Some(id) => write!(f, "Id:{id} ")?,
            None => write!(f, "Id:- ")?,
        };
        write!(f, "| {:?}: ", self.topic)?;
        match str::from_utf8(self.payload) {
            Ok(payload) => write!(f, "{payload:?} ")?,
            Err(_) => write!(f, "{:?} ", self.payload)?,
        }
        write!(f, "}}")?;

        Ok(())
    }
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
                payload: body,
            },
        ))
    }
}
