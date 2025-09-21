use crate::{
    protocol::{
        Packet, PacketError, PacketWrite, Parse, ParseError,
        types::{FixedHeader, VariableByteInteger},
    },
    utils::Cursor,
};

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

impl Parse for ConnAck {
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
