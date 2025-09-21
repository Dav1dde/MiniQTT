use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:1883").await?;
    let stream = embedded_io_adapters::tokio_1::FromTokio::new(&mut stream);

    let mut rx_buffer = [0; 12];
    let connection = miniqtt::Connection::new(stream, &mut rx_buffer);

    let mut client = miniqtt::Client::new(connection);
    client.connect().await?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    client.disconnect().await?;

    Ok(())
}
