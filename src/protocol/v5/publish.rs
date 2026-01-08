use core::fmt;

use crate::protocol::types::{EncodedStr, VariableByteInteger};
use crate::protocol::utils::CursorExt;
use crate::protocol::{Packet, PacketError, Parse, ParseError, QoS};
use crate::traits::Writable;
use crate::utils::Cursor;

pub struct Publish<'a> {
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
    pub identifier: Option<u16>,
    pub topic: &'a str,
    pub payload: &'a [u8],
}

impl fmt::Debug for Publish<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Publish {{ ")?;
        write!(f, "Q{} ", self.qos as u8)?;
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

    fn flags(&self) -> u8 {
        (self.dup as u8) << 3 | (self.qos as u8) << 1 | self.retain as u8
    }
}

impl<'a> Parse<'a> for Publish<'a> {
    type Error = PacketError;

    fn parse(data: &'a [u8]) -> Result<(usize, Self), ParseError<Self::Error>> {
        let mut cursor = Cursor::new(data);

        let fixed_header = cursor.read_fixed_header::<Self>()?;

        let dup = fixed_header.flags() & 0b1000 > 0;
        let qos = QoS::try_from((fixed_header.flags() >> 1) & 0b11)
            .map_err(|_| PacketError::ProtocolError)?;
        let retain = fixed_header.flags() & 0b0001 > 0;

        let packet_length = fixed_header.length().as_u32() as usize;
        let start_length = cursor.position();

        let EncodedStr(topic) = cursor.read()?;

        let identifier = match qos {
            QoS::AtMostOnce => None,
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

impl Writable for Publish<'_> {
    type Error<E> = E;

    fn size(&self) -> usize {
        EncodedStr(self.topic).size()
            + self.identifier.size()
            + VariableByteInteger::from(0u8).size()
            + self.payload.len()
    }

    async fn write_to<S>(&self, mut sink: S) -> Result<(), Self::Error<S::Error>>
    where
        S: embedded_io_async::Write,
    {
        EncodedStr(self.topic).write_to(&mut sink).await?;
        self.identifier.write_to(&mut sink).await?;

        // TODO: properties
        VariableByteInteger::from(0u8).write_to(&mut sink).await?;

        sink.write_all(self.payload).await?;

        Ok(())
    }
}
