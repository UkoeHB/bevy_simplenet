//local shortcuts

//third-party shortcuts
use serde::{Serialize, Deserialize};

//standard shortcuts
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// message from server
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DemoServerMsg(pub u64);

/// message from client
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DemoClientMsg(pub String);

/// client connect message
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DemoConnectMsg(pub String);

#[derive(Debug, Clone)]
pub struct DemoMsgPack;
impl bevy_simplenet::MsgPack for DemoMsgPack
{
    type ConnectMsg = DemoConnectMsg;
    type ClientMsg = DemoClientMsg;
    type ClientRequest = ();
    type ServerMsg = DemoServerMsg;
    type ServerResponse = ();
}

type _DemoServer = bevy_simplenet::Server::<DemoMsgPack>;
type _DemoClient = bevy_simplenet::Client::<DemoMsgPack>;
type _DemoServerVal = bevy_simplenet::ServerValFromPack<DemoMsgPack>;
type _DemoClientVal = bevy_simplenet::ClientValFromPack<DemoMsgPack>;

fn server_demo_factory() -> bevy_simplenet::ServerFactory<DemoMsgPack>
{
    bevy_simplenet::ServerFactory::<DemoMsgPack>::new("test")
}

fn client_demo_factory() -> bevy_simplenet::ClientFactory<DemoMsgPack>
{
    bevy_simplenet::ClientFactory::<DemoMsgPack>::new("test")
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn message_size_limit_test(max_msg_size: u32)
{
    // prepare tokio runtimes for server and client
    let server_runtime = enfync::builtin::native::TokioHandle::default();
    let client_runtime = enfync::builtin::Handle::default();

    // launch websocket server
    let websocket_server = server_demo_factory().new_server(
            server_runtime,
            "127.0.0.1:0",
            bevy_simplenet::AcceptorConfig::Default,
            bevy_simplenet::Authenticator::None,
            bevy_simplenet::ServerConfig{
                max_connections   : 10,
                max_msg_size,
                rate_limit_config : bevy_simplenet::RateLimitConfig{
                        period    : std::time::Duration::from_millis(15),
                        max_count : 25
                    },
                heartbeat_interval : std::time::Duration::from_secs(5),
                keepalive_timeout  : std::time::Duration::from_secs(10),
            }
        );

    let websocket_url = websocket_server.url();


    // 1. prepare message that is too large
    let mut msg_vec = Vec::<u8>::new();
    msg_vec.resize((max_msg_size + 1) as usize, 1u8);
    let large_msg = String::from_utf8(msg_vec).unwrap();


    // 2. connect message size limit

    // make client with invalid connect message size
    let large_connect_msg = DemoConnectMsg(large_msg.clone());
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 67891u128 },
            bevy_simplenet::ClientConfig{
                max_initial_connect_attempts: 1usize,
                ..Default::default()
            },
            large_connect_msg
        );

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    assert!(websocket_client.is_dead());  //failed to connect
    let Some(bevy_simplenet::ClientReport::IsDead) = websocket_client.next_report()
    else { panic!("client should be closed by server"); };
    assert_eq!(websocket_server.num_connections(), 0u64);


    // 3. client message size limit

    // make client
    let connect_msg = DemoConnectMsg(String::from(""));
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url,
            bevy_simplenet::AuthRequest::None{ client_id: 4678587u128 },
            bevy_simplenet::ClientConfig::default(),
            connect_msg.clone()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(bevy_simplenet::ServerReport::Connected(client_id, connect_msg)) = websocket_server.next_report()
    else { panic!("server should be connected once client is connected"); };
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    assert_eq!(connect_msg.0, connect_msg.0);
    assert_eq!(websocket_server.num_connections(), 1u64);

    // send message with invalid size: client -> server
    let signal = websocket_client.send(DemoClientMsg(large_msg)).unwrap();
    assert_eq!(signal.status(), ezsockets::MessageStatus::Sending);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    // expect no message acquired by server
    let None = websocket_server.next_val() else { panic!("server received client msg"); };

    // expect client was disconnected
    assert_eq!(signal.status(), ezsockets::MessageStatus::Sent);  //sent and then server shut us down
    assert!(websocket_client.is_dead());

    let Some(bevy_simplenet::ServerReport::Disconnected(dc_client_id)) = websocket_server.next_report()
    else { panic!("client should be disconnected"); };
    let Some(bevy_simplenet::ClientReport::ClosedByServer(_)) = websocket_client.next_report()
    else { panic!("client should be closed by server"); };
    let Some(bevy_simplenet::ClientReport::IsDead) = websocket_client.next_report()
    else { panic!("client should be closed by server"); };
    assert_eq!(client_id, dc_client_id);
    assert_eq!(websocket_server.num_connections(), 0u64);


    // no more connection reports
    let None = websocket_server.next_report()
    else { panic!("server should receive no more connection reports"); };
    let None = websocket_client.next_report()
    else { panic!("client should receive no more connection reports"); };
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
fn bevy_simplenet_message_size_limit()
{
    message_size_limit_test(25);
    message_size_limit_test(40);
    message_size_limit_test(100);
}

//-------------------------------------------------------------------------------------------------------------------
