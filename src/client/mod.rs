use core::sync::atomic::{AtomicU16, Ordering};

use crate::log;
use crate::protocol::types::FixedHeader;
use crate::protocol::{Packet, PacketError, Parse, ParseError, v5};
use crate::traits::Writable;

mod connect;

#[doc(inline)]
pub use self::connect::Connect;

pub struct Client<'a, C> {
    // TODO: connection should possibly a trait to make dealing with it easier, or make the Client
    // a trait.
    connection: Connection<'a, C>,
    identifier: AtomicU16, // TODO: maybe we don't need the atomic here
}

impl<'a, C> Client<'a, C> {
    pub fn new(connection: Connection<'a, C>) -> Self {
        Self {
            connection,
            identifier: AtomicU16::new(20_000),
        }
    }
}

impl<'c, C> Client<'c, C>
where
    C: embedded_io_async::Read,
    C: embedded_io_async::Write,
{
    // TODO: maybe only connected clients should be able to be created via a builder.
    // TODO: sending methods could send the payload, then return a future which simply awaits
    // the correct response (based on the packet identifier), this allows for concurrent messages
    // being sent and concurrently being awaited. The problem is, the mutable borrow for the shared
    // buffer though, so that might not be possible.

    /// Initiates a connection with the MQTT broker.
    ///
    /// # Cancel safety
    ///
    /// The returned future is *not* cancel safe.
    pub fn connect<'a>(
        &mut self,
        client_id: &'a str,
    ) -> Connect<'a, impl connect::MakeFuture<'a, Output = Result<(), C::Error>>> {
        Connect::new(client_id, |packet| async move {
            self.connection.send(&packet).await?;

            let _ack = self.connection.receive::<v5::ConnAck>().await?;

            Ok(())
        })
    }

    pub async fn subscribe(&mut self, topic: &str) -> Result<(), C::Error> {
        let packet = v5::Subscribe {
            identifier: self.next_identifier(),
            topics: &[(topic, 0, true)],
        };
        self.connection.send(&packet).await?;

        let _ack = self.connection.receive::<v5::SubAck>().await?;

        Ok(())
    }

    /// Receives a message from the MQTT server.
    ///
    /// # Cancel safety
    ///
    /// This method *is* cancel safe.
    pub async fn receive(&mut self) -> Result<(), C::Error> {
        // TODO: while this is cancel safe, it may be interleaved with with different send
        // calls which expect different packages from the server.
        // These in-between publish messages may need to be dropped (so we can get to the ACK)
        // or temporarily buffered and skipped (if the buffer size is big enough).
        // This should follow the QoS of the package.
        let _message = self.connection.receive::<v5::Publish>().await?;

        Ok(())
    }

    /// Disconnects from the server by sending a disconnect message and dropping the connection.
    ///
    /// # Cancel safety
    ///
    /// This method is *not* cancel safe.
    pub async fn disconnect(&mut self) -> Result<(), C::Error> {
        // TODO: should probably keep track of connection state
        // and also drop the connection here.
        self.connection.send(&v5::Disconnect {}).await?;

        Ok(())
    }

    fn next_identifier(&self) -> u16 {
        self.identifier.fetch_add(1, Ordering::Relaxed)
    }
}

pub struct Connection<'a, C> {
    inner: C,
    // TODO: buffer should be generic and possibly be resizable, to allow for dynamic and growing buffers
    // TODO: should be some kind of cursor
    rx_buffer: &'a mut [u8],
    size: usize,
    position: Option<usize>,
}

impl<'a, C> Connection<'a, C> {
    pub fn new(inner: C, rx_buffer: &'a mut [u8]) -> Self {
        Self {
            inner,
            rx_buffer,
            size: 0,
            position: None,
        }
    }
}

impl<C> Connection<'_, C>
where
    C: embedded_io_async::Write,
{
    async fn send<T>(&mut self, packet: &T) -> Result<(), T::Error<C::Error>>
    where
        T: Packet,
        T: Writable,
        T: core::fmt::Debug,
    {
        log::debug!("-> {packet:?}");

        // TODO: proper PacketError
        FixedHeader::new(T::TYPE, packet.flags(), packet.size())
            .write_to(&mut self.inner)
            .await
            .unwrap();

        packet.write_to(&mut self.inner).await?;

        Ok(())
    }
}

impl<C> Connection<'_, C>
where
    C: embedded_io_async::Read,
{
    async fn receive<'a, T>(&'a mut self) -> Result<T, C::Error>
    where
        T: Parse<'a, Error = PacketError>,
        T: core::fmt::Debug,
    {
        // Move all the remaining data which is left in the buffer to the beginning,
        // to make sure the next package is properly aligned.
        // We need to do this at the beginning of reading a new packet, instead of
        // at the end, because the just read packet may point into the buffer.
        //
        // There are two possible optimization we can do:
        //  1) Make the buffer wrap, which requires support in all packets to parse from
        //     a non continuous slice.
        //  2) Read in two iterations. The first read only reads enough for the fixed header, 2-5
        //     bytes, from that we know how long the total length of the packet is and we can
        //     target read just enough for the packet, minimizing the amount of data we have to
        //     copy.
        if let Some(position) = self.position.take() {
            log::trace!("{:?} -{}", &self.rx_buffer[..self.size], position);
            self.rx_buffer.copy_within(position..self.size, 0);
            self.size -= position;
            log::trace!("{:?} ={}", &self.rx_buffer[..self.size], self.size);
        }

        loop {
            let (data, remaining) = self.rx_buffer.split_at_mut(self.size);

            // TODO: confirm the details written down here.
            //
            // This seems really like a borrow checker limitation. On each iteration of the loop,
            // we split the buffer into two separate mutable borrows.
            //  - The first one is *only* used to parse the package.
            //  - THe second one is *only* used to read more data.
            // None of the two halves escapes the loop iteration, unless we exit the function,
            // on the next iteration, there are no more open references to `rx_buffer` and we can
            // make a fresh split.
            //
            // The transmute _should_ be safe, as we still tie the packet to 'self, combined with
            // the usage of the buffers described before..
            match T::parse(unsafe { core::mem::transmute::<&[u8], &[u8]>(&*data) }) {
                Ok((position, packet)) => {
                    self.position = Some(position);
                    log::debug!("<- {packet:?}");
                    return Ok(packet);
                }
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
                log::trace!("{:?} +{r}", &self.rx_buffer[..self.size]);
            }
        }
    }
}
