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
type _DemoServerVal = bevy_simplenet::ServerValFrom<DemoChannel>;
type _DemoClientVal = bevy_simplenet::ClientValFrom<DemoChannel>;

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

fn connections_limit_test(max_connections: u32)
{
    assert!(max_connections > 0);

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
                max_connections,
                max_msg_size      : 10_000,
                rate_limit_config : bevy_simplenet::RateLimitConfig{
                        period    : std::time::Duration::from_millis(15),
                        max_count : 25
                    },
                heartbeat_interval : std::time::Duration::from_secs(5),
                keepalive_timeout  : std::time::Duration::from_secs(10),
            }
        );

    let websocket_url = websocket_server.url();
    assert_eq!(websocket_server.num_connections(), 0u64);


    // 1. connect 'max connections' clients
    let mut clients = Vec::new();
    let connect_msg = DemoConnectMsg(String::from("hello"));

    for client_num in 0..max_connections
    {
        // make client
        let websocket_client = client_demo_factory().new_client(
                client_runtime.clone(),
                websocket_url.clone(),
                bevy_simplenet::AuthRequest::None{ client_id: client_num as u128 },
                bevy_simplenet::ClientConfig::default(),
                connect_msg.clone()
            );

        std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

        // client should connect
        assert!(!websocket_client.is_dead());
        let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
        else { panic!("client should be connected to server"); };
        let Some(bevy_simplenet::ServerReport::Connected(_, _)) = websocket_server.next_report()
        else { panic!("server should be connected to client: {}", client_num); };
        assert_eq!(websocket_server.num_connections(), 1u64 + client_num as u64);

        clients.push(websocket_client);
    }

    // 2. connecting one more client should fail
    // make client
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 92748u128 },
            bevy_simplenet::ClientConfig{
                max_initial_connect_attempts: 1usize,
                ..Default::default()
            },
            connect_msg.clone()
        );

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    // client should fail to connect
    assert!(websocket_client.is_dead());
    let Some(bevy_simplenet::ClientReport::IsDead) = websocket_client.next_report()
    else { panic!("client should have failed to connect"); };
    let None = websocket_server.next_report()
    else { panic!("server should not connect to another client"); };
    assert_eq!(websocket_server.num_connections(), max_connections as u64);

    // 3. disconnect one client
    let client_to_disconnect = clients.pop().expect("there should be at least one connected client");
    client_to_disconnect.close();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(bevy_simplenet::ClientReport::ClosedBySelf) = client_to_disconnect.next_report()
    else { panic!("client should be closed by self"); };
    let Some(bevy_simplenet::ServerReport::Disconnected(_)) = websocket_server.next_report()
    else { panic!("server should see a disconnected client"); };
    assert_eq!(websocket_server.num_connections(), (max_connections - 1) as u64);

    // 4. adding a client should now succeed
    // make client
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 64819u128 },
            bevy_simplenet::ClientConfig::default(),
            connect_msg.clone()
        );

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    // client should connect
    assert!(!websocket_client.is_dead());
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    let Some(bevy_simplenet::ServerReport::Connected(_, _)) = websocket_server.next_report()
    else { panic!("server should be connected to client"); };
    assert_eq!(websocket_server.num_connections(), max_connections as u64);

    clients.push(websocket_client);  //save client so it doesn't get dropped

    // 5. connecting one more client should fail
    // make client
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 15364898u128 },
            bevy_simplenet::ClientConfig{
                max_initial_connect_attempts: 1usize,
                ..Default::default()
            },
            connect_msg.clone()
        );

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    // client should not connect
    assert!(websocket_client.is_dead());
    let Some(bevy_simplenet::ClientReport::IsDead) = websocket_client.next_report()
    else { panic!("client should be closed by server"); };
    let None = websocket_server.next_report()
    else { panic!("server should not connect to another client"); };
    assert_eq!(websocket_server.num_connections(), max_connections as u64);


    // no more connection reports
    let None = websocket_server.next_report()
    else { panic!("server should receive no more connection reports"); };
    let None = websocket_client.next_report()
    else { panic!("client should receive no more connection reports"); };
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
fn bevy_simplenet_connections_limit()
{
    connections_limit_test(1);
    connections_limit_test(2);
    connections_limit_test(10);
}

//-------------------------------------------------------------------------------------------------------------------
