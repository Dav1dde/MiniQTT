use crate::protocol::{Parse, ParseError};

pub struct Cursor<'a> {
    buf: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, position: 0 }
    }

    pub fn read_u8<T>(&mut self) -> Result<u8, ParseError<T>> {
        let result = *self.rem().first().ok_or(ParseError::NotEnoughData)?;
        self.position += 1;
        Ok(result)
    }

    pub fn read<T: Parse>(&mut self) -> Result<T, ParseError<T::Error>> {
        let (len, packet) = T::parse(self.rem())?;
        self.position += len;
        Ok(packet)
    }

    pub fn read_slice<T>(&mut self, amount: usize) -> Result<&'a [u8], ParseError<T>> {
        let result = self.rem().get(..amount).ok_or(ParseError::NotEnoughData)?;
        self.position += amount;
        Ok(result)
    }

    pub fn position(&self) -> usize {
        self.position
    }

    fn rem(&self) -> &'a [u8] {
        &self.buf[self.position..]
    }
}
