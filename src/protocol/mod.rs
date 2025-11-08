mod qos;

pub mod types;
pub mod v5;

pub use qos::*;

pub trait Packet {
    const TYPE: u8;

    fn flags(&self) -> u8 {
        0
    }
}

pub trait Parse<'a>: Sized {
    type Error;

    // TODO: Send + Sync fun etc.
    // TODO: Writable expects no header to be written, but this requires to manually parse the header.
    fn parse(data: &'a [u8]) -> ParseResult<(usize, Self), Self::Error>;
}

pub trait PacketParse<'a>: Sized {
    fn parse(data: &'a [u8]) -> ParseResult<(usize, Self), PacketError>;
}

impl<'a, T> Parse<'a> for T
where
    T: PacketParse<'a>,
{
    type Error = PacketError;

    fn parse(data: &'a [u8]) -> ParseResult<(usize, Self), Self::Error> {
        <T as PacketParse>::parse(data)
    }
}

pub type ParseResult<T, E> = Result<T, ParseError<E>>;

#[derive(Debug, PartialEq, Eq)] // TODO: impl error
pub enum ParseError<T> {
    NotEnoughData,
    Error(T),
}

impl<T> ParseError<T> {
    pub fn map<F, S>(self, f: F) -> ParseError<S>
    where
        F: FnOnce(T) -> S,
    {
        match self {
            Self::NotEnoughData => ParseError::NotEnoughData,
            Self::Error(err) => ParseError::Error(f(err)),
        }
    }
}

impl<T> From<T> for ParseError<T> {
    fn from(value: T) -> Self {
        Self::Error(value)
    }
}

#[derive(Debug)]
pub enum PacketError {
    InvalidType { expected: u8, actual: u8 },
    // TODO: this should be more descriptive
    ProtocolError,
}
