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

#[derive(Debug, Clone)]
pub struct DemoChannel;
impl bevy_simplenet::ChannelPack for DemoChannel
{
    type ConnectMsg = DemoConnectMsg;
    type ClientMsg = DemoClientMsg;
    type ClientRequest = ();
    type ServerMsg = DemoServerMsg;
    type ServerResponse = ();
}

type _DemoServer = bevy_simplenet::Server<DemoChannel>;
type _DemoClient = bevy_simplenet::Client<DemoChannel>;
type DemoClientEvent = bevy_simplenet::ClientEventFrom<DemoChannel>;
type DemoServerEvent = bevy_simplenet::ServerEventFrom<DemoChannel>;
type DemoServerReport = bevy_simplenet::ServerReport<<DemoChannel as bevy_simplenet::ChannelPack>::ConnectMsg>;

fn server_demo_factory() -> bevy_simplenet::ServerFactory<DemoChannel>
{
    bevy_simplenet::ServerFactory::<DemoChannel>::new("test")
}

fn client_demo_factory() -> bevy_simplenet::ClientFactory<DemoChannel>
{
    bevy_simplenet::ClientFactory::<DemoChannel>::new("test")
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn rate_limit_test(max_count_per_period: u32)
{
    // prepare tokio runtimes for server and client
    let server_runtime = enfync::builtin::native::TokioHandle::default();
    let client_runtime = enfync::builtin::Handle::default();

    // launch websocket server
    let mut websocket_server = server_demo_factory().new_server(
            server_runtime,
            "127.0.0.1:0",
            bevy_simplenet::AcceptorConfig::Default,
            bevy_simplenet::Authenticator::None,
            bevy_simplenet::ServerConfig{
                max_connections   : 10,
                max_msg_size      : 1_000,
                rate_limit_config : bevy_simplenet::RateLimitConfig{
                        period    : std::time::Duration::from_millis(15),  //15ms to coordinate with async waits
                        max_count : max_count_per_period
                    },
                heartbeat_interval : std::time::Duration::from_secs(5),
                keepalive_timeout  : std::time::Duration::from_secs(10),
            }
        );

    let websocket_url = websocket_server.url();


    // make client
    let connect_msg = DemoConnectMsg(String::from("hello!"));
    let mut websocket_client = client_demo_factory().new_client(
            client_runtime,
            websocket_url,
            bevy_simplenet::AuthRequest::None{ client_id: 3578762u128 },
            bevy_simplenet::ClientConfig::default(),
            connect_msg.clone()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((client_id, DemoServerEvent::Report(DemoServerReport::Connected(_, connect_msg)))) = websocket_server.next()
    else { panic!("server should be connected once client is connected"); };
    let Some(DemoClientEvent::Report(bevy_simplenet::ClientReport::Connected)) = websocket_client.next()
    else { panic!("client should be connected to server"); };
    assert_eq!(connect_msg.0, connect_msg.0);
    assert_eq!(websocket_server.num_connections(), 1u64);


    // send message: client -> server
    let client_val = 42;
    let signal = websocket_client.send(DemoClientMsg(client_val));
    assert_eq!(signal.status(), ezsockets::MessageStatus::Sending);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((msg_client_id, DemoServerEvent::Msg(DemoClientMsg(msg_client_val)))) = websocket_server.next()
    else { panic!("server did not receive client msg"); };
    assert_eq!(client_id, msg_client_id);
    assert_eq!(client_val, msg_client_val);
    assert_eq!(signal.status(), ezsockets::MessageStatus::Sent);
    assert_eq!(websocket_server.num_connections(), 1u64);



    // send messages to fill up server rate limiter to the brim
    let mut signals = Vec::new();
    for _ in 0..max_count_per_period
    {
        signals.push(websocket_client.send(DemoClientMsg(client_val)));
    }

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    // expect all messages received
    for i in 0..max_count_per_period
    {
        let Some((msg_client_id, DemoServerEvent::Msg(DemoClientMsg(msg_client_val)))) = websocket_server.next()
        else { panic!("server did not receive client msg"); };
        assert_eq!(client_id, msg_client_id);
        assert_eq!(client_val, msg_client_val);
        assert_eq!(signals[i as usize].status(), ezsockets::MessageStatus::Sent);
    }

    // server should still be alive
    assert!(!websocket_server.is_dead());
    assert_eq!(websocket_server.num_connections(), 1u64);

    // expect no more messages
    let None = websocket_server.next()
    else { panic!("server received more client msgs than expected"); };


    // send messages to fill up server rate limiter past the brim
    let mut signals = Vec::new();
    for _ in 0..(max_count_per_period + 1)
    {
        signals.push(websocket_client.send(DemoClientMsg(client_val)));
    }

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    // expect all message except last received
    for i in 0..max_count_per_period
    {
        let Some((msg_client_id, DemoServerEvent::Msg(DemoClientMsg(msg_client_val)))) = websocket_server.next()
        else { panic!("server did not receive client msg"); };
        assert_eq!(client_id, msg_client_id);
        assert_eq!(client_val, msg_client_val);
        assert_eq!(signals[i as usize].status(), ezsockets::MessageStatus::Sent);
    }

    // server should still be alive
    assert!(!websocket_server.is_dead());

    // expect client was disconnected (message sent and then server shut us down)
    // - expect no more messages (last message was dropped)
    assert_eq!(signals[max_count_per_period as usize].status(), ezsockets::MessageStatus::Sent);
    assert!(websocket_client.is_dead());

    let Some((dc_client_id, DemoServerEvent::Report(DemoServerReport::Disconnected))) = websocket_server.next()
    else { panic!("client should be disconnected"); };
    let Some(DemoClientEvent::Report(bevy_simplenet::ClientReport::ClosedByServer(_))) = websocket_client.next()
    else { panic!("client should be closed by server"); };
    let Some(DemoClientEvent::Report(bevy_simplenet::ClientReport::IsDead(_))) = websocket_client.next()
    else { panic!("client should be closed by server"); };
    assert_eq!(client_id, dc_client_id);


    // no more connection reports
    let None = websocket_server.next()
    else { panic!("server should receive no more connection reports"); };
    let None = websocket_client.next()
    else { panic!("client should receive no more connection reports"); };
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
fn rate_limiter()
{
    rate_limit_test(1);
    rate_limit_test(2);
    rate_limit_test(20);
}

//-------------------------------------------------------------------------------------------------------------------
