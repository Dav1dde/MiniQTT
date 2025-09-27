use core::fmt;

/// A MQTT Client error.
#[derive(Debug)]
pub enum Error<E> {
    /// The client is not connected to the server.
    ///
    /// The connection may have been closed by the server or the client
    /// disconnected.
    Disconnected,
    /// A protocol error.
    ///
    /// Protocol errors may happen when invalid data is received, a protocol error cannot be
    /// recovered.
    Protocol,
    /// The connection buffer is not big enough to receive a package.
    InsufficientBufferSize,
    /// An underlying error occurred on the connection.
    Connection(E),
}

impl<E> From<E> for Error<E> {
    fn from(value: E) -> Self {
        Self::Connection(value)
    }
}

impl<E> fmt::Display for Error<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "The connection is closed!"),
            Self::Protocol => write!(f, "A protocol error occured!"),
            Self::InsufficientBufferSize => {
                write!(f, "Buffer is not big enough to parse a received packet!")
            }
            Self::Connection(err) => write!(f, "A connection error occured: {err}"),
        }
    }
}

impl<E> core::error::Error for Error<E> where E: core::error::Error {}

/// A MQTT Client result.
pub type Result<T, E> = core::result::Result<T, Error<E>>;
