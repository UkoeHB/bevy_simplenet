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

#[derive(Debug, Clone)]
pub struct DemoChannel;
impl bevy_simplenet::ChannelPack for DemoChannel
{
    type ConnectMsg = ();
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

// Client message sends should synchronize with processing of connection events
#[test]
fn client_send_sync_msg()
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
    let websocket_server = server_demo_factory().new_server(
            server_runtime,
            "127.0.0.1:0",
            bevy_simplenet::AcceptorConfig::Default,
            bevy_simplenet::Authenticator::None,
            bevy_simplenet::ServerConfig::default()
        );

    let websocket_url = websocket_server.url();
    assert_eq!(websocket_server.num_connections(), 0u64);


    // make client
    let mut websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 0u128 },
            bevy_simplenet::ClientConfig::default(),
            ()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((client_id, DemoServerEvent::Report(DemoServerReport::Connected(_, ())))) = websocket_server.next()
    else { unreachable!() };

    // sending a message before the client report is consumed should fail
    let client_val = 24;
    let signal = websocket_client.send(DemoClientMsg(client_val));
    assert_eq!(signal.status(), bevy_simplenet::MessageStatus::Failed);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let None = websocket_server.next() else { unreachable!() };


    // consume connected report
    let Some(DemoClientEvent::Report(bevy_simplenet::ClientReport::Connected)) = websocket_client.next()
    else { unreachable!() };


    // sending a message after the client report is consumed should succeed
    let client_val = 42;
    let signal = websocket_client.send(DemoClientMsg(client_val));
    assert_eq!(signal.status(), bevy_simplenet::MessageStatus::Sending);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((msg_client_id, DemoServerEvent::Msg(DemoClientMsg(msg_client_val)))) = websocket_server.next()
    else { unreachable!() };
    assert_eq!(client_id, msg_client_id);
    assert_eq!(client_val, msg_client_val);
    assert_eq!(signal.status(), bevy_simplenet::MessageStatus::Sent);


    // no more events
    let None = websocket_server.next() else { unreachable!() };
    let None = websocket_client.next() else { unreachable!() };
}

//-------------------------------------------------------------------------------------------------------------------

// Client message requests should synchronize with processing of connection events
#[test]
fn client_send_sync_request()
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
    let websocket_server = server_demo_factory().new_server(
            server_runtime,
            "127.0.0.1:0",
            bevy_simplenet::AcceptorConfig::Default,
            bevy_simplenet::Authenticator::None,
            bevy_simplenet::ServerConfig::default()
        );

    let websocket_url = websocket_server.url();
    assert_eq!(websocket_server.num_connections(), 0u64);


    // make client
    let mut websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 0u128 },
            bevy_simplenet::ClientConfig::default(),
            ()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((_, DemoServerEvent::Report(DemoServerReport::Connected(_, ())))) = websocket_server.next()
    else { unreachable!() };

    // sending a request before the client report is consumed should fail
    let signal = websocket_client.request(());
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::SendFailed);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let None = websocket_server.next() else { unreachable!() };


    // consume connected report
    let Some(DemoClientEvent::Report(bevy_simplenet::ClientReport::Connected)) = websocket_client.next()
    else { unreachable!() };


    // sending a request after the client report is consumed should succeed
    let signal = websocket_client.request(());
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Sending);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((_, DemoServerEvent::Request(token, ()))) = websocket_server.next() else { unreachable!() };
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Waiting);
    websocket_server.ack(token);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(DemoClientEvent::Ack(_)) = websocket_client.next() else { unreachable!() };
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Acknowledged);


    // no more events
    let None = websocket_server.next() else { unreachable!() };
    let None = websocket_client.next() else { unreachable!() };
}

//-------------------------------------------------------------------------------------------------------------------
