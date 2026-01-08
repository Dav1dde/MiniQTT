use miniqtt::protocol::v5::ConnectProperty;

mod common;

#[tokio::test]
async fn test_client_connect_no_credentials() {
    let mosquitto = common::Mosquitto::builder().start();
    let mut client = mosquitto.client().await;

    let res = client.connect("miniqtt").await.unwrap();
    assert!(res.successful());
}

#[tokio::test]
async fn test_client_connect_username_anon() {
    let mosquitto = common::Mosquitto::builder().start();
    let mut client = mosquitto.client().await;

    let res = client
        .connect("miniqtt")
        .with_username("foo")
        .await
        .unwrap();

    assert!(res.successful());
}

#[tokio::test]
async fn test_client_connect_credentials_anon() {
    let mosquitto = common::Mosquitto::builder().start();
    let mut client = mosquitto.client().await;

    let res = client
        .connect("miniqtt")
        .with_username("foo")
        .with_password("bar")
        .await
        .unwrap();

    assert!(res.successful());
}

#[tokio::test]
async fn test_client_connect_credentials_valid() {
    let mosquitto = common::Mosquitto::builder()
        .credentials("foo", "bar")
        .start();
    let mut client = mosquitto.client().await;

    let res = client
        .connect("miniqtt")
        .with_username("foo")
        .with_password("bar")
        .await
        .unwrap();

    assert!(res.successful());
}

#[tokio::test]
async fn test_client_connect_credentials_invalid() {
    let mosquitto = common::Mosquitto::builder()
        .credentials("xxx", "yyy")
        .start();
    let mut client = mosquitto.client().await;

    let res = client
        .connect("miniqtt")
        .with_username("foo")
        .with_password("bar")
        .await
        .unwrap();

    assert!(!res.successful());
}

#[tokio::test]
async fn test_client_connect_resume_session() {
    let mosquitto = common::Mosquitto::builder().start();
    let mut client = mosquitto.client().await;

    let res = client
        .connect("miniqtt")
        // Indicate the client wants to resume sessions.
        .resume_session(true)
        // How long the server should keep the session around.
        .with_properties(&[ConnectProperty::SessionExpiryInterval(99)])
        .await
        .unwrap();

    assert!(res.successful());

    let _ = client.disconnect().await;

    // Re-Connect, with a new client, resuming the old session.
    let mut client = mosquitto.client().await;

    let res = client
        .connect("miniqtt")
        .resume_session(true)
        .await
        .unwrap();

    assert!(res.successful());
    assert!(res.session_present());

    // Note: Just because the client sets the correct flags and asserts the resumption, it does not
    // mean it implements proper session resumptions.
}
