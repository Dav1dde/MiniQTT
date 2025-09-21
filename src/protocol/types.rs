use crate::{
    protocol::{PacketError, Parse, ParseError},
    utils::Cursor,
};

/// The fixed header is a basic building block of the MQTT protocol, it is the beginning of each
/// packet, containing its type, flags and a variable length.
pub struct FixedHeader {
    start: u8,
    length: VariableByteInteger,
}

impl FixedHeader {
    /// They type of the packet.
    pub fn ty(&self) -> u8 {
        self.start >> 4
    }

    /// The associated flags with this packet.
    pub fn flags(&self) -> u8 {
        self.start & 0b1111
    }

    /// The variable length in bytes of the packet, following the fixed header.
    pub fn length(&self) -> VariableByteInteger {
        self.length
    }
}

impl Parse for FixedHeader {
    type Error = PacketError;

    fn parse(data: &[u8]) -> Result<(usize, Self), ParseError<Self::Error>> {
        let mut cursor = Cursor::new(data);

        let start = cursor.read_u8()?;
        let length = cursor
            .read()
            .map_err(|err| err.map(|_| PacketError::ProtocolError))?;

        Ok((cursor.position(), Self { start, length }))
    }
}

/// A variable byte integer.
///
/// Specification: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>.
///
/// # Examples:
///
/// ```
/// # use miniqtt::protocol::types::VariableByteInteger;
/// let a: VariableByteInteger = 17u8.into();
/// assert_eq!(a.as_u32(), 17);
/// assert_eq!(a.as_slice(), &[0x11])
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VariableByteInteger(
    /// Variable byte integer in its encoded form.
    ///
    /// Invariant: unused bytes *must* be set to 0.
    [u8; 4],
);

impl VariableByteInteger {
    fn encode(num: u32) -> Result<Self, VariableByteIntegerOverflow> {
        if num > 0xfffffff {
            return Err(VariableByteIntegerOverflow { _private: () });
        }

        let mut num = num;
        let mut result = [0; 4];

        for result in &mut result {
            let mut byte = (num % 0x80) as u8;
            num /= 0x80;
            if num > 0 {
                byte |= 0x80;
            }
            *result = byte;
        }

        Ok(Self(result))
    }

    /// Parses a variable encoded integer from a slice.
    ///
    /// Returns [`ParseError::NotEnoughData`] if the slice does not contain
    /// enough data to parse the integer. Parsing can be re-attempted with more data.
    ///
    /// When parsing succeeds the parsed [`VariableByteInteger`] and its length are returned.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use miniqtt::protocol::types::VariableByteInteger;
    /// # use miniqtt::protocol::ParseError;
    /// assert_eq!(VariableByteInteger::parse(&[0x7f]), Ok((1, 127u8.into())));
    /// assert_eq!(VariableByteInteger::parse(&[0x80]), Err(ParseError::NotEnoughData));
    /// assert_eq!(VariableByteInteger::parse(&[0x80, 0x01]), Ok((2, 128u8.into())));
    /// assert_eq!(VariableByteInteger::parse(&[0x80, 0x01, 0xff]), Ok((2, 128u8.into())));
    /// assert!(matches!(
    ///     VariableByteInteger::parse(&[0xff, 0xff, 0xff, 0xff]),
    ///     Err(ParseError::Error(_))
    /// ));
    /// ```
    pub fn parse(data: &[u8]) -> Result<(usize, Self), ParseError<VariableByteIntegerInvalid>> {
        let mut result = [0; 4];
        let mut len = 0;

        for (i, result) in result.iter_mut().enumerate() {
            len = i + 1;
            let b = *data.get(i).ok_or(ParseError::NotEnoughData)?;

            *result = b;
            if b < 0x80 {
                break;
            }
        }

        if result[3] > 0x7f {
            return Err(VariableByteIntegerInvalid { _private: () }.into());
        }

        Ok((len, Self(result)))
    }

    /// Returns the encoded value as an `u8`, if it can be represented as such.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use miniqtt::protocol::types::VariableByteInteger;
    /// let a: VariableByteInteger = 127u8.into();
    /// assert_eq!(a.as_u8(), Some(127));
    ///
    /// let b: VariableByteInteger = 128u8.into();
    /// assert_eq!(b.as_u8(), None);
    /// ```
    pub fn as_u8(&self) -> Option<u8> {
        match self.0[0] {
            v @ 0..=0x7f => Some(v),
            _ => None,
        }
    }

    /// Returns the encoded value as an `u16`, if it can be represented as such.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use miniqtt::protocol::types::VariableByteInteger;
    /// let a: VariableByteInteger = 127u16.into();
    /// assert_eq!(a.as_u16(), Some(127));
    ///
    /// let b: VariableByteInteger = 16382u16.into();
    /// assert_eq!(b.as_u16(), Some(16382));
    ///
    /// let b: VariableByteInteger = 16384u16.into();
    /// assert_eq!(b.as_u16(), None);
    /// ```
    pub fn as_u16(&self) -> Option<u16> {
        match self.0[0] {
            v @ 0..=0x7f => Some(u16::from(v)),
            v1 => match self.0[1] {
                v2 @ 0..=0x7f => {
                    let a = (v1 & 0x7f) as u16;
                    let b = (v2 & 0x7f) as u16;
                    Some(a | b << 7)
                }
                _ => None,
            },
        }
    }

    /// Returns the encoded value as an `u32`.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use miniqtt::protocol::types::VariableByteInteger;
    /// let a: VariableByteInteger = 127u16.into();
    /// assert_eq!(a.as_u32(), 127);
    ///
    /// let b: VariableByteInteger = 17000u16.into();
    /// assert_eq!(b.as_u32(), 17000);
    /// ```
    pub fn as_u32(&self) -> u32 {
        let mut result = 0;
        let mut mult = 1;

        for byte in self.0 {
            result += u32::from(byte & 0x7f) * mult;
            mult *= 0x80;
        }

        result
    }

    /// Size in bytes of the variable byte encoded integer.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use miniqtt::protocol::types::VariableByteInteger;
    /// let a: VariableByteInteger = 0u16.into();
    /// assert_eq!(a.size(), 1);
    ///
    /// let b: VariableByteInteger = 127u16.into();
    /// assert_eq!(b.size(), 1);
    ///
    /// let c: VariableByteInteger = 200_000_000u32.try_into().unwrap();
    /// assert_eq!(c.size(), 4);
    /// ```
    pub fn size(&self) -> usize {
        let end = self.0[1..]
            .iter()
            .position(|x| *x == 0)
            .unwrap_or(self.0.len() - 1);

        end + 1
    }

    /// Returns the raw encoded data of the encoded integer.
    ///
    /// # Examples:
    ///
    /// ```
    /// # use miniqtt::protocol::types::VariableByteInteger;
    /// let a: VariableByteInteger = 0u16.into();
    /// assert_eq!(a.as_slice(), &[0x00]);
    ///
    /// let b: VariableByteInteger = 127u16.into();
    /// assert_eq!(b.as_slice(), &[0x7f]);
    ///
    /// let c: VariableByteInteger = 200_000_000u32.try_into().unwrap();
    /// assert_eq!(c.as_slice(), &[0x80, 0x84, 0xaf, 0x5f]);
    /// ```
    pub fn as_slice(&self) -> &[u8] {
        &self.0[..self.size()]
    }
}

#[derive(Debug)] // TODO: implement `Error`
pub struct VariableByteIntegerOverflow {
    _private: (),
}

#[derive(Debug, PartialEq, Eq)] // TODO: implement `Error`
pub struct VariableByteIntegerInvalid {
    _private: (),
}

impl From<u8> for VariableByteInteger {
    fn from(value: u8) -> Self {
        Self::encode(value.into()).unwrap()
    }
}

impl From<u16> for VariableByteInteger {
    fn from(value: u16) -> Self {
        Self::encode(value.into()).unwrap()
    }
}

impl TryFrom<u32> for VariableByteInteger {
    type Error = VariableByteIntegerOverflow;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::encode(value)
    }
}

impl From<VariableByteInteger> for u32 {
    fn from(value: VariableByteInteger) -> Self {
        value.as_u32()
    }
}

impl Parse for VariableByteInteger {
    type Error = VariableByteIntegerInvalid;

    fn parse(data: &[u8]) -> Result<(usize, Self), ParseError<Self::Error>> {
        Self::parse(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_var_byte_int {
        ($value:expr, $repr:expr) => {
            match $value {
                value => {
                    let encoded = VariableByteInteger::try_from(value as u32).unwrap();
                    assert_eq!(encoded.as_slice(), $repr);
                    assert_eq!(encoded.as_u32(), value as u32);
                    assert_eq!(u32::from(encoded), value as u32);
                }
            }
        };
    }

    #[test]
    fn test_var_byte_int() {
        for byte in 0..128 {
            assert_var_byte_int!(byte, &[byte]);
        }

        assert_var_byte_int!(128, &[0x80, 0x01]);
        assert_var_byte_int!(16383, &[0xff, 0x7f]);

        assert_var_byte_int!(16384, &[0x80, 0x80, 0x01]);
        assert_var_byte_int!(2097151, &[0xff, 0xff, 0x7f]);

        assert_var_byte_int!(2097152, &[0x80, 0x80, 0x80, 0x01]);
        assert_var_byte_int!(268435455, &[0xff, 0xff, 0xff, 0x7f]);
    }

    #[test]
    fn test_var_byte_int_large() {}
}
