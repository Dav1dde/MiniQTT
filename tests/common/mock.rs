use std::{
    io,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::common::utils::{HexBlock, parse_hex_block};

type Connection<'a> = embedded_io_adapters::tokio_1::FromTokio<MockStream<'a>>;

pub struct MockServer {
    read: Vec<u8>,
    write: Vec<u8>,
}

impl MockServer {
    pub fn new() -> Self {
        Self {
            read: Vec::new(),
            write: Vec::new(),
        }
    }

    pub async fn client(&mut self) -> TestClient<'_> {
        let stream = MockStream {
            reader: &mut self.read,
            writer: &mut self.write,
        };
        let stream = embedded_io_adapters::tokio_1::FromTokio::new(stream);

        let connection = miniqtt::Connection::new(stream, vec![0; 128].leak());
        TestClient {
            client: miniqtt::Client::new(connection),
        }
    }
}

pub struct TestClient<'a> {
    client: miniqtt::Client<'a, Connection<'a>>,
}

impl TestClient<'_> {
    pub fn respond_with(&mut self, response: &str) {
        let data = parse_hex_block(response);
        *self.client.inner_mut().inner_mut().reader = data;
    }

    #[track_caller]
    pub fn assert(&mut self, expected: &str) {
        let expected = HexBlock::new(&parse_hex_block(expected)).to_string();

        let sent = std::mem::take(self.client.inner_mut().inner_mut().writer);
        let sent = HexBlock::new(&sent).to_string();

        assert_eq!(expected, sent);
        assert!(
            self.client.inner().inner().reader.is_empty(),
            "leftover data"
        );
    }
}

impl<'a> Deref for TestClient<'a> {
    type Target = miniqtt::Client<'a, Connection<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl<'a> DerefMut for TestClient<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

pub struct MockStream<'a> {
    reader: &'a mut Vec<u8>,
    writer: &'a mut Vec<u8>,
}

impl AsyncRead for MockStream<'_> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let reader = &mut *self.get_mut().reader;

        let amt = std::cmp::min(reader.len(), buf.remaining());
        buf.put_slice(&reader[..amt]);

        reader.copy_within(amt.., 0);
        reader.truncate(reader.len() - amt);

        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MockStream<'_> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut self.get_mut().writer).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.get_mut().writer).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.get_mut().writer).poll_shutdown(cx)
    }
}
