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

#[test]
fn bevy_simplenet_hello_world()
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
    let server_runtime = enfync::builtin::IOHandle::default();
    let client_runtime = enfync::builtin::IOHandle::default();

    // prepare connection acceptor
    let plain_acceptor = ezsockets::tungstenite::Acceptor::Plain;

    // launch websocket server
    tracing::info!("ws hello world test: launching server...");
    let websocket_server = server_demo_factory().new_server(
            server_runtime,
            "127.0.0.1:0",
            plain_acceptor,
            bevy_simplenet::Authenticator::None,
            bevy_simplenet::ServerConfig{
                max_connections   : 10,
                max_msg_size      : 1_000,
                rate_limit_config : bevy_simplenet::RateLimitConfig{
                        period    : std::time::Duration::from_secs(1),
                        max_count : 20
                    }
            }
        );

    let websocket_url = websocket_server.url();



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

    let Some(bevy_simplenet::ServerReport::Connected(client_id, connect_msg)) = websocket_server.next_report()
    else { panic!("server should be connected once client is connected"); };
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    assert_eq!(connect_msg.0, connect_msg1.0);


    // send message: client -> server
    tracing::info!("ws hello world test: client sending msg...");
    let client_val = 42;
    websocket_client.send(&DemoClientMsg(client_val)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((msg_client_id, DemoClientMsg(msg_client_val))) = websocket_server.next_msg()
    else { panic!("server did not receive client msg"); };
    assert_eq!(client_id, msg_client_id);
    assert_eq!(client_val, msg_client_val);


    // send message: server -> client
    tracing::info!("ws hello world test: server sending msg...");
    let server_val = 24;
    websocket_server.send(client_id, DemoServerMsg(server_val)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(DemoServerMsg(msg_server_val)) = websocket_client.next_msg()
    else { panic!("client did not receive server msg"); };
    assert_eq!(server_val, msg_server_val);


    // server closes client
    tracing::info!("ws hello world test: server closing client...");
    let closure_frame =
        ezsockets::CloseFrame{
            code   : ezsockets::CloseCode::Normal,
            reason : String::from("test")
        };
    websocket_server.close_session(client_id, closure_frame).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    assert!(!websocket_server.is_dead());
    assert!(websocket_client.is_dead());

    let Some(bevy_simplenet::ClientReport::ClosedByServer(_)) = websocket_client.next_report()
    else { panic!("client should be closed by server"); };
    let Some(bevy_simplenet::ServerReport::Disconnected(dc_client_id)) = websocket_server.next_report()
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

    let Some(bevy_simplenet::ServerReport::Connected(client_id, connect_msg)) = websocket_server.next_report()
    else { panic!("server should be connected once client is connected"); };
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    assert_eq!(connect_msg.0, connect_msg2.0);


    // client closes client
    tracing::info!("ws hello world test: client closing client...");
    websocket_client.close();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    assert!(!websocket_server.is_dead());
    assert!(websocket_client.is_dead());

    let Some(bevy_simplenet::ServerReport::Disconnected(dc_client_id)) = websocket_server.next_report()
    else { panic!("server should be disconnected after client is disconnected (by client)"); };
    let Some(bevy_simplenet::ClientReport::ClosedBySelf) = websocket_client.next_report()
    else { panic!("client should have closed itself"); };
    assert_eq!(client_id, dc_client_id);


    // no more connection reports
    let None = websocket_server.next_report()
    else { panic!("server should receive no more connection reports"); };
    let None = websocket_client.next_report()
    else { panic!("client should receive no more connection reports"); };
}

//-------------------------------------------------------------------------------------------------------------------
