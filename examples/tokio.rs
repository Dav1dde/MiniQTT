use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder().format_timestamp(None).init();

    let mut stream = TcpStream::connect("127.0.0.1:1883").await?;
    let stream = embedded_io_adapters::tokio_1::FromTokio::new(&mut stream);

    let connection = miniqtt::Connection::new(stream, [0; 128]);
    let mut client = miniqtt::Client::new(connection);

    client
        .connect("miniqtt")
        .with_username("foo")
        .with_password("bar")
        .keep_alive(10)
        .await?;

    client.send("miniqtt", b"hello world").await?;

    client.subscribe("$SYS/#").await?;
    loop {
        client.receive().await?;
    }
}
