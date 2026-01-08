use core::sync::atomic::{AtomicU16, Ordering};

use crate::log;
use crate::protocol::types::FixedHeader;
use crate::protocol::v5::TopicFilter;
use crate::protocol::{Packet, PacketError, Parse, ParseError, QoS, v5};
use crate::traits::Writable;

mod connect;
mod error;
mod utils;

pub use self::connect::{Connect, ConnectResponse};
pub use self::error::{Error, Result};
pub use self::utils::MakeFuture;

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
    ) -> Connect<'a, impl MakeFuture<v5::Connect<'a>, Output = Result<ConnectResponse, C::Error>>>
    {
        Connect::new(client_id, |packet| async move {
            self.connection.send(&packet).await?;

            let ack = self.connection.receive::<v5::ConnAck>().await?;

            // TODO: according to the protocol, if the reason is not successful, the client must
            // terminate the connection. Currently the connection trait just asks for Read/Write,
            // there is no way to force close the connection.
            //
            // Maybe that is okay, maybe we should keep internal state on the client/connection and
            // reject all further interactions, or just do nothing.

            Ok(ConnectResponse { ack })
        })
    }

    pub async fn subscribe(&mut self, topic: &str) -> Result<(), C::Error> {
        let packet = v5::Subscribe {
            identifier: self.next_identifier(),
            topics: &[TopicFilter {
                name: topic,
                qos: QoS::AtMostOnce,
                no_local: Default::default(),
                retain_as_published: Default::default(),
                retain: Default::default(),
            }],
        };
        self.connection.send(&packet).await?;

        let _ack = self.connection.receive::<v5::SubAck>().await?;

        Ok(())
    }

    // TODO: Make a builder like for `connect` which supports:
    //  - QoS
    //  - Topic Alias (send(..).with_alias(&mut my_alias)), where the alias tracks its internal
    //  register state (including id). Not sure how you'd free an alias again, maybe there is just
    //  no API for that and you just re-use different topic ids?
    pub async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<(), C::Error> {
        let packet = v5::Publish {
            dup: false,
            qos: QoS::AtMostOnce,
            retain: false,
            identifier: None,
            topic,
            payload,
        };

        self.connection.send(&packet).await?;

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
    async fn send<T>(&mut self, packet: &T) -> Result<(), C::Error>
    where
        T: Packet,
        T: Writable,
        T: core::fmt::Debug,
        T::Error<C::Error>: Into<C::Error>,
    {
        log::debug!("-> {packet:?}");

        FixedHeader::new(T::TYPE, packet.flags(), packet.size())
            .write_to(&mut self.inner)
            .await?;

        packet.write_to(&mut self.inner).await.map_err(Into::into)?;

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
                Err(ParseError::Error(_err)) => {
                    // TODO: once we end up here, we will never make progress
                    //  1) Maybe just close the connection/disconnect, check the spec!
                    //  2) Try to recover:
                    //     - Throw away all data and start from scratch.
                    //     - Throw away exactly one packet, we should know based on the fixed
                    //     header.
                    // Not trying to recover and just disconnecting is probably the better idea.
                    // Also need to consider QoS levels without disconnect.
                    log::debug!("protocol error: {_err:?}");
                    return Err(Error::Protocol);
                }
            }

            if remaining.is_empty() {
                // TODO: maybe can recover here by just skipping the current packet,
                // assuming the buffer is big enough to parse the fixed header.
                //
                // This allows recovery from oversized `PUBLISH` packets while still
                // handling other packets gracefully.
                // Need to consider QoS levels here possibly.
                //
                // In any case, we should return an error here at least once to inform the user,
                // something was dropped.
                return Err(Error::InsufficientBufferSize);
            }

            let r = self.inner.read(remaining).await?;
            if r == 0 {
                match data.is_empty() {
                    true => log::debug!("Clean Exit"),
                    false => log::debug!("Connection Reset by Peer"),
                };
                return Err(Error::Disconnected);
            } else {
                self.size += r;
                log::trace!("{:?} +{r}", &self.rx_buffer[..self.size]);
            }
        }
    }
}
