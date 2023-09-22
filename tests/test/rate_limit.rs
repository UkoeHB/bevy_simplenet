//local shortcuts

//third-party shortcuts
use serde::{Serialize, Deserialize};

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// message from server
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DemoServerMsg(pub u64);

/// message from client
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DemoClientMsg(pub u64);

/// client connect message
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DemoConnectMsg(pub String);

type ServerDemo = bevy_simplenet::Server::<DemoServerMsg, DemoClientMsg, DemoConnectMsg>;
type ClientDemo = bevy_simplenet::Client::<DemoServerMsg, DemoClientMsg, DemoConnectMsg>;

fn server_demo_factory() -> ServerDemo::Factory
{
    ServerDemo::Factory::new("test")
}

fn client_demo_factory() -> ClientDemo::Factory
{
    ClientDemo::Factory::new("test")
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn rate_limit_test(max_count_per_period: u32)
{
    // prepare tokio runtimes for server and client
    let server_runtime = enfync::builtin::Handle::default();
    let client_runtime = enfync::builtin::Handle::default();

    // prepare connection acceptor
    let plain_acceptor = ezsockets::tungstenite::Acceptor::Plain;

    // launch websocket server
    let websocket_server = enfync::blocking::extract(server_demo_factory().new_server(
            server_runtime,
            "127.0.0.1:0",
            plain_acceptor,
            bevy_simplenet::Authenticator::None,
            bevy_simplenet::ServerConfig{
                max_connections   : 10,
                max_msg_size      : 1_000,
                rate_limit_config : bevy_simplenet::RateLimitConfig{
                        period    : std::time::Duration::from_millis(15),  //15ms to coordinate with async waits
                        max_count : max_count_per_period
                    }
            }
        )).unwrap();

    let websocket_url = websocket_server.url();


    // make client
    let connect_msg = DemoConnectMsg(String::from("hello!"));
    let websocket_client = enfync::blocking::extract(client_demo_factory().new_client(
            client_runtime,
            websocket_url,
            bevy_simplenet::AuthRequest::None{ client_id: 3578762u128 },
            bevy_simplenet::ClientConfig::default(),
            connect_msg.clone()
        )).unwrap();
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(bevy_simplenet::ServerReport::Connected(client_id, connect_msg)) = websocket_server.next_report()
    else { panic!("server should be connected once client is connected"); };
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    assert_eq!(connect_msg.0, connect_msg.0);


    // send message: client -> server
    let client_val = 42;
    websocket_client.send(&DemoClientMsg(client_val)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((msg_client_id, DemoClientMsg(msg_client_val))) = websocket_server.next_msg()
    else { panic!("server did not receive client msg"); };
    assert_eq!(client_id, msg_client_id);
    assert_eq!(client_val, msg_client_val);



    // send messages to fill up server rate limiter to the brim
    for _ in 0..max_count_per_period
    {
        websocket_client.send(&DemoClientMsg(client_val)).unwrap();
    }

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    // expect all messages received
    for _ in 0..max_count_per_period
    {
        let Some((msg_client_id, DemoClientMsg(msg_client_val))) = websocket_server.next_msg()
        else { panic!("server did not receive client msg"); };
        assert_eq!(client_id, msg_client_id);
        assert_eq!(client_val, msg_client_val);
    }

    // server should still be alive
    assert!(!websocket_server.is_dead());

    // expect no more messages
    let None = websocket_server.next_msg()
    else { panic!("server received more client msgs than expected"); };


    // send messages to fill up server rate limiter past the brim
    for _ in 0..(max_count_per_period + 1)
    {
        websocket_client.send(&DemoClientMsg(client_val)).unwrap();
    }

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    // expect all message except last received
    for _ in 0..max_count_per_period
    {
        let Some((msg_client_id, DemoClientMsg(msg_client_val))) = websocket_server.next_msg()
        else { panic!("server did not receive client msg"); };
        assert_eq!(client_id, msg_client_id);
        assert_eq!(client_val, msg_client_val);
    }

    // server should still be alive
    assert!(!websocket_server.is_dead());

    // expect no more messages (last message was dropped)
    let None = websocket_server.next_msg()
    else { panic!("server received more client msgs than expected"); };

    // expect client was disconnected
    assert!(websocket_client.is_dead());

    let Some(bevy_simplenet::ServerReport::Disconnected(dc_client_id)) = websocket_server.next_report()
    else { panic!("client should be disconnected"); };
    let Some(bevy_simplenet::ClientReport::ClosedByServer(_)) = websocket_client.next_report()
    else { panic!("client should be closed by server"); };
    assert_eq!(client_id, dc_client_id);


    // no more connection reports
    let None = websocket_server.next_report()
    else { panic!("server should receive no more connection reports"); };
    let None = websocket_client.next_report()
    else { panic!("client should receive no more connection reports"); };
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
fn bevy_simplenet_rate_limiter()
{
    rate_limit_test(1);
    rate_limit_test(2);
    rate_limit_test(20);
}

//-------------------------------------------------------------------------------------------------------------------
