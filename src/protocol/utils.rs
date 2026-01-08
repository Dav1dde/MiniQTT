use crate::protocol::types::FixedHeader;
use crate::protocol::{Packet, PacketError, ParseResult};
use crate::utils::Cursor;

pub trait CursorExt {
    fn read_fixed_header<T>(&mut self) -> ParseResult<FixedHeader>
    where
        T: Packet;
}

impl CursorExt for Cursor<'_> {
    /// Reads a fixed header for a specific packet `T`.
    ///
    /// This utility also validates the read fixed header to match the expected packet.
    fn read_fixed_header<T>(&mut self) -> ParseResult<FixedHeader>
    where
        T: Packet,
    {
        let header = self.read::<FixedHeader>()?;

        if header.ty() != T::TYPE {
            return Err(PacketError::InvalidPacketType {
                expected: T::TYPE,
                actual: header.ty(),
            }
            .into());
        }

        Ok(header)
    }
}
