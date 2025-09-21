pub mod types;
pub mod v5;

pub trait Packet {
    const TYPE: u8;
}

pub trait PacketWrite: Packet {
    // TODO: Send + Sync fun etc.
    fn write<T>(&self, write: T) -> impl Future<Output = Result<(), T::Error>>
    where
        T: embedded_io_async::Write;
}

pub trait Parse: Sized {
    type Error;

    // TODO: Send + Sync fun etc.
    fn parse(data: &[u8]) -> Result<(usize, Self), ParseError<Self::Error>>;
}

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
    ProtocolError,
}
