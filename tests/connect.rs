mod common;

#[tokio::test]
async fn test_client_connect_no_credentials() {
    let mosquitto = common::Mosquitto::builder().start();
    let mut client = mosquitto.client().await;

    client.connect("miniqtt").await.unwrap();
}

#[tokio::test]
async fn test_client_connect_username_anon() {
    let mosquitto = common::Mosquitto::builder().start();
    let mut client = mosquitto.client().await;

    client
        .connect("miniqtt")
        .with_username("foo")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_client_connect_credentials_anon() {
    let mosquitto = common::Mosquitto::builder().start();
    let mut client = mosquitto.client().await;

    client
        .connect("miniqtt")
        .with_username("foo")
        .with_password("bar")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_client_connect_credentials_valid() {
    let mosquitto = common::Mosquitto::builder()
        .credentials("foo", "bar")
        .start();
    let mut client = mosquitto.client().await;

    client
        .connect("miniqtt")
        .with_username("foo")
        .with_password("bar")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_client_connect_credentials_invalid() {
    let mosquitto = common::Mosquitto::builder()
        .credentials("xxx", "yyy")
        .start();
    let mut client = mosquitto.client().await;

    client
        .connect("miniqtt")
        .with_username("foo")
        .with_password("bar")
        .await
        .unwrap();

    // TODO: this test is broken, the connection is rejected in the `ConAck` but that is not yet
    // validated
}
