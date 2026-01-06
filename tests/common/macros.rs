#[macro_export]
macro_rules! assert_exchange {
    ($client:ident ==> @$foo:literal) => {{
        let client = &mut $client;

        let x = client.take_from_client();
        insta::assert_snapshot!($crate::common::utils::HexBlock::new(&x), @$foo);
        // let x = client.take_from_server();
        // insta::assert_snapshot!($crate::common::utils::HexBlock::new(&x), "bar lol", @$bar);
    }};
    ($client:ident <== @$foo:literal) => {{
        let client = &mut $client;

        let x = client.take_from_server();
        insta::assert_snapshot!($crate::common::utils::HexBlock::new(&x), @$foo);
    }};
}
