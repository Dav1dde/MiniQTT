mod common;

macro_rules! miniqtt_test {
    () => {
        
    };
}


#[tokio::test]
async fn mock() {
    let mut server = common::MockServer::new();
    let mut client = server.client().await;

    client.respond_with("20 09 00 00 06 22 00 0a  21 00 14");

    client
        .connect("miniqtt")
        .with_username("aaa")
        .with_password("bbb")
        .keep_alive(0x42)
        .await
        .unwrap();

    client.assert(
        "
        10 1e 00 04 4d 51 54 54  05 c2 00 42 00 00 07 6d
        69 6e 69 71 74 74 00 03  61 61 61 00 03 62 62 62
    ",
    );
}

#[tokio::test]
async fn foobar() {
    let server = common::TestServer::auto().await;
    let mut client = server.client().await;

    client.respond_with("20 09 00 00 06 22 00 0a  21 00 14");

    client
        .connect("miniqtt")
        .with_username("aaa")
        .with_password("bbb")
        .keep_alive(0x42)
        .await
        .unwrap();

    client.assert(
        "
        10 1e 00 04 4d 51 54 54  05 c2 00 42 00 00 07 6d
        69 6e 69 71 74 74 00 03  61 61 61 00 03 62 62 62
    ",
    );
}
