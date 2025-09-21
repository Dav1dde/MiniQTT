use crate::{
    protocol::{PacketError, PacketWrite, Parse, ParseError, v5},
    utils::Cursor,
};

pub struct Client<'a, C> {
    // TODO: connection should possibly a trait to make dealing with it easier, or make the Client
    // a trait.
    connection: Connection<'a, C>,
}

impl<'a, C> Client<'a, C> {
    pub fn new(connection: Connection<'a, C>) -> Self {
        Self { connection }
    }
}

impl<C> Client<'_, C>
where
    C: embedded_io_async::Read,
    C: embedded_io_async::Write,
{
    // TODO: maybe only connected clients should be able to be created via a builder.
    pub async fn connect(&mut self) -> Result<(), C::Error> {
        self.connection.send(&v5::Connect {}).await?;

        let ack = self.connection.receive::<v5::ConnAck>().await?;

        dbg!(ack);

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), C::Error> {
        self.connection.send(&v5::Disconnect {}).await?;

        Ok(())
    }
}

pub struct Connection<'a, C> {
    inner: C,
    // TODO: buffer should be generic and possibly be resizable, to allow for dynamic and growing buffers
    // TODO: should be some kind of cursor
    rx_buffer: &'a mut [u8],
    size: usize,
}

impl<'a, C> Connection<'a, C> {
    pub fn new(inner: C, rx_buffer: &'a mut [u8]) -> Self {
        Self {
            inner,
            rx_buffer,
            size: 0,
        }
    }
}

impl<C> Connection<'_, C>
where
    C: embedded_io_async::Write,
{
    async fn send<T: PacketWrite>(&mut self, packet: &T) -> Result<(), C::Error> {
        packet.write(&mut self.inner).await?;

        Ok(())
    }
}

impl<C> Connection<'_, C>
where
    C: embedded_io_async::Read,
{
    async fn receive<T: Parse<Error = PacketError>>(&mut self) -> Result<T, C::Error> {
        loop {
            let (data, remaining) = self.rx_buffer.split_at_mut(self.size);

            let mut cursor = Cursor::new(data);
            match cursor.read::<T>() {
                Ok(packet) => return Ok(packet),
                Err(ParseError::NotEnoughData) => {}
                Err(ParseError::Error(err)) => panic!("failed to parse packet {err:?}"),
            }

            if remaining.is_empty() {
                panic!("Buffer to small");
            }

            let r = self.inner.read(remaining).await?;
            if r == 0 {
                if data.is_empty() {
                    panic!("Clean Exit");
                } else {
                    panic!("Connection Reset by Peer");
                }
            } else {
                self.size += r;
            }
        }
    }
}
