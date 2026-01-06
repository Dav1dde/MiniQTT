use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicU32;

use tokio::net::{TcpStream, ToSocketAddrs};

use crate::common::Wiretap;
use crate::common::utils::{HexBlock, parse_hex_block};

type Connection = embedded_io_adapters::tokio_1::FromTokio<Wiretap<TcpStream>>;

/// A test handle for a real MQTT server.
pub struct TestServer {
    client: miniqtt::Client<'static, Connection>,
    prefix: String,
}

impl TestServer {
    pub async fn connect<S>(addr: S) -> Self
    where
        S: ToSocketAddrs,
    {
        let client = connect_to(addr).await;

        Self {
            client,
            prefix: new_prefix(),
        }
    }

    pub async fn auto() -> Self {
        Self::connect("127.0.0.1:1883").await
    }

    /// Returns a to this test namespaced topic name.
    pub fn topic(&self, name: &str) -> String {
        format!("{prefix}/{name}", prefix = self.prefix)
    }

    /// Creates and returns a newly to the server connected [`TestClient`].
    pub async fn client(&self) -> TestClient {
        let addr = self.client.inner().inner().inner().peer_addr().unwrap();
        let client = connect_to(addr).await;

        TestClient {
            client,
            expected_server_response: Vec::new(),
        }
    }
}

pub struct TestClient {
    client: miniqtt::Client<'static, Connection>,
    expected_server_response: Vec<u8>,
}

impl TestClient {
    pub fn respond_with(&mut self, response: &str) {
        self.expected_server_response = parse_hex_block(response);
    }

    #[track_caller]
    pub fn assert(&mut self, expected: &str) {
        let expected = HexBlock::new(&parse_hex_block(expected)).to_string();

        let sent = std::mem::take(self.wiretap_mut().write_mut());
        let sent = HexBlock::new(&sent).to_string();

        assert_eq!(expected, sent);

        let expected =
            HexBlock::new(&std::mem::take(&mut self.expected_server_response)).to_string();

        let received = std::mem::take(self.wiretap_mut().read_mut());
        let received = HexBlock::new(&received).to_string();

        assert_eq!(expected, received);
    }

    fn wiretap_mut(&mut self) -> &mut Wiretap<TcpStream> {
        self.client.inner_mut().inner_mut()
    }
}

impl Deref for TestClient {
    type Target = miniqtt::Client<'static, Connection>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for TestClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

async fn connect_to<S>(addr: S) -> miniqtt::Client<'static, Connection>
where
    S: ToSocketAddrs,
{
    let stream = TcpStream::connect(addr).await.unwrap();
    let stream = Wiretap::new(stream);
    let stream = embedded_io_adapters::tokio_1::FromTokio::new(stream);

    // TODO: connection needs to be generic to support owned buffers, leak for now.
    let rx_buffer = vec![0; 128].leak();
    let connection = miniqtt::Connection::new(stream, rx_buffer);

    miniqtt::Client::new(connection)
}

fn new_prefix() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);

    let i = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    // TODO: maybe need a pid in here
    format!("miniqtt/test/{i}")
}
