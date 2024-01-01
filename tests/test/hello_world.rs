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

#[test]
fn hello_world()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    tracing::info!("ws hello world test: start");
    */

    // prepare tokio runtimes for server and client
    let server_runtime = enfync::builtin::native::TokioHandle::default();
    let client_runtime = enfync::builtin::Handle::default();

    // launch websocket server
    tracing::info!("ws hello world test: launching server...");
    let websocket_server = server_demo_factory().new_server(
            server_runtime,
            "127.0.0.1:0",
            bevy_simplenet::AcceptorConfig::Default,
            bevy_simplenet::Authenticator::None,
            bevy_simplenet::ServerConfig{
                max_connections   : 10,
                max_msg_size      : 1_000,
                rate_limit_config : bevy_simplenet::RateLimitConfig{
                        period    : std::time::Duration::from_secs(1),
                        max_count : 20
                    },
                heartbeat_interval : std::time::Duration::from_secs(5),
                keepalive_timeout  : std::time::Duration::from_secs(10),
            }
        );

    let websocket_url = websocket_server.url();
    assert_eq!(websocket_server.num_connections(), 0u64);



    // make client
    tracing::info!("ws hello world test: launching client...");
    let connect_msg1 = DemoConnectMsg(String::from("hello!"));
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 44718u128 },
            bevy_simplenet::ClientConfig::default(),
            connect_msg1.clone()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((client_id, DemoServerEvent::Report(DemoServerReport::Connected(_, connect_msg)))) = websocket_server.next()
    else { panic!("server should be connected once client is connected"); };
    let Some(DemoClientEvent::Report(bevy_simplenet::ClientReport::Connected)) = websocket_client.next()
    else { panic!("client should be connected to server"); };
    assert_eq!(connect_msg.0, connect_msg1.0);
    assert_eq!(websocket_server.num_connections(), 1u64);


    // send message: client -> server
    tracing::info!("ws hello world test: client sending msg...");
    let client_val = 42;
    let signal = websocket_client.send(DemoClientMsg(client_val)).unwrap();
    assert_eq!(signal.status(), ezsockets::MessageStatus::Sending);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((msg_client_id, DemoServerEvent::Msg(DemoClientMsg(msg_client_val)))) = websocket_server.next()
    else { panic!("server did not receive client msg"); };
    assert_eq!(client_id, msg_client_id);
    assert_eq!(client_val, msg_client_val);
    assert_eq!(signal.status(), ezsockets::MessageStatus::Sent);


    // send message: server -> client
    tracing::info!("ws hello world test: server sending msg...");
    let server_val = 24;
    websocket_server.send(client_id, DemoServerMsg(server_val)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(DemoClientEvent::Msg(DemoServerMsg(msg_server_val))) = websocket_client.next()
    else { panic!("client did not receive server msg"); };
    assert_eq!(server_val, msg_server_val);


    // server closes client
    tracing::info!("ws hello world test: server closing client...");
    let closure_frame =
        ezsockets::CloseFrame{
            code   : ezsockets::CloseCode::Normal,
            reason : String::from("test")
        };
    websocket_server.close_session(client_id, Some(closure_frame)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    assert!(!websocket_server.is_dead());
    assert!(websocket_client.is_dead());
    assert_eq!(websocket_server.num_connections(), 0u64);

    let Some(DemoClientEvent::Report(bevy_simplenet::ClientReport::ClosedByServer(_))) = websocket_client.next()
    else { panic!("client should be closed by server"); };
    let Some((dc_client_id, DemoServerEvent::Report(DemoServerReport::Disconnected))) = websocket_server.next()
    else { panic!("server should be disconnected after client is disconnected (by server)"); };
    assert_eq!(client_id, dc_client_id);



    // new client
    tracing::info!("ws hello world test: launching client 2...");
    let connect_msg2 = DemoConnectMsg(String::from("hello 2!"));
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url,
            bevy_simplenet::AuthRequest::None{ client_id: 872657u128 },
            bevy_simplenet::ClientConfig::default(),
            connect_msg2.clone()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((client_id, DemoServerEvent::Report(DemoServerReport::Connected(_, connect_msg)))) = websocket_server.next()
    else { panic!("server should be connected once client is connected"); };
    let Some(DemoClientEvent::Report(bevy_simplenet::ClientReport::Connected)) = websocket_client.next()
    else { panic!("client should be connected to server"); };
    assert_eq!(connect_msg.0, connect_msg2.0);
    assert_eq!(websocket_server.num_connections(), 1u64);


    // client closes client
    tracing::info!("ws hello world test: client closing client...");
    websocket_client.close();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    assert!(!websocket_server.is_dead());
    assert!(websocket_client.is_dead());
    assert_eq!(websocket_server.num_connections(), 0u64);

    let Some((dc_client_id, DemoServerEvent::Report(DemoServerReport::Disconnected))) = websocket_server.next()
    else { panic!("server should be disconnected after client is disconnected (by client)"); };
    let Some(DemoClientEvent::Report(bevy_simplenet::ClientReport::ClosedBySelf)) = websocket_client.next()
    else { panic!("client should have closed itself"); };
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
