//local shortcuts

//third-party shortcuts
use serde::{Serialize, Deserialize};

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// message from server
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DemoServerResponse(pub u64);

/// message from client
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DemoClientRequest(pub u64);

#[derive(Debug, Clone)]
pub struct DemoChannel;
impl bevy_simplenet::ChannelPack for DemoChannel
{
    type ConnectMsg = ();
    type ClientMsg = ();
    type ClientRequest = DemoClientRequest;
    type ServerMsg = ();
    type ServerResponse = DemoServerResponse;
}

type _DemoServer = bevy_simplenet::Server<DemoChannel>;
type _DemoClient = bevy_simplenet::Client<DemoChannel>;
type DemoServerVal = bevy_simplenet::ServerValFrom<DemoChannel>;
type DemoClientVal = bevy_simplenet::ClientValFrom<DemoChannel>;

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
fn request_response()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
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
            bevy_simplenet::ServerConfig::default(),
        );

    let websocket_url = websocket_server.url();
    assert_eq!(websocket_server.num_connections(), 0u64);


    // make client
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 44718u128 },
            bevy_simplenet::ClientConfig::default(),
            ()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(bevy_simplenet::ServerReport::Connected(client_id, _, ())) = websocket_server.next_report()
    else { panic!("server should be connected once client is connected"); };
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    assert_eq!(websocket_server.num_connections(), 1u64);


    // send request: client -> server
    let client_val = 42;
    let signal = websocket_client.request(DemoClientRequest(client_val)).unwrap();
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Sending);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((
            msg_client_id,
            DemoClientVal::Request(DemoClientRequest(msg_client_val), token)
        )) = websocket_server.next_val()
    else { panic!("server did not receive client msg"); };
    assert_eq!(client_id, msg_client_id);
    assert_eq!(client_id, token.client_id());
    assert_eq!(signal.id(), token.request_id());
    assert_eq!(client_val, msg_client_val);
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Waiting);
    assert!(!token.destination_is_dead());


    // send response: server -> client
    let server_val = 24;
    websocket_server.respond(token, DemoServerResponse(server_val)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(DemoServerVal::Response(DemoServerResponse(msg_server_val), request_id)) = websocket_client.next_val()
    else { panic!("client did not receive server msg"); };
    assert_eq!(server_val, msg_server_val);
    assert_eq!(signal.id(), request_id);
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Responded);


    // no more outputs
    let None = websocket_server.next_report()
    else { panic!("server should receive no more connection reports"); };
    let None = websocket_client.next_report()
    else { panic!("client should receive no more connection reports"); };
    let None = websocket_client.next_val()
    else { panic!("client should receive no more values"); };
    let None = websocket_client.next_val()
    else { panic!("client should receive no more values"); };
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn request_ack()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
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
            bevy_simplenet::ServerConfig::default(),
        );

    let websocket_url = websocket_server.url();
    assert_eq!(websocket_server.num_connections(), 0u64);


    // make client
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 44718u128 },
            bevy_simplenet::ClientConfig::default(),
            ()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(bevy_simplenet::ServerReport::Connected(client_id, _, ())) = websocket_server.next_report()
    else { panic!("server should be connected once client is connected"); };
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    assert_eq!(websocket_server.num_connections(), 1u64);


    // send request: client -> server
    let client_val = 42;
    let signal = websocket_client.request(DemoClientRequest(client_val)).unwrap();
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Sending);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((
            msg_client_id,
            DemoClientVal::Request(DemoClientRequest(msg_client_val), token)
        )) = websocket_server.next_val()
    else { panic!("server did not receive client msg"); };
    assert_eq!(client_id, msg_client_id);
    assert_eq!(client_id, token.client_id());
    assert_eq!(signal.id(), token.request_id());
    assert_eq!(client_val, msg_client_val);
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Waiting);
    assert!(!token.destination_is_dead());


    // send ack: server -> client
    websocket_server.acknowledge(token).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(DemoServerVal::Ack(request_id)) = websocket_client.next_val()
    else { panic!("client did not receive server msg"); };
    assert_eq!(signal.id(), request_id);
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Acknowledged);


    // no more outputs
    let None = websocket_server.next_report()
    else { panic!("server should receive no more connection reports"); };
    let None = websocket_client.next_report()
    else { panic!("client should receive no more connection reports"); };
    let None = websocket_client.next_val()
    else { panic!("client should receive no more values"); };
    let None = websocket_client.next_val()
    else { panic!("client should receive no more values"); };
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn request_rejected()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
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
            bevy_simplenet::ServerConfig::default(),
        );

    let websocket_url = websocket_server.url();
    assert_eq!(websocket_server.num_connections(), 0u64);


    // make client
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 44718u128 },
            bevy_simplenet::ClientConfig::default(),
            ()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(bevy_simplenet::ServerReport::Connected(client_id, _, ())) = websocket_server.next_report()
    else { panic!("server should be connected once client is connected"); };
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    assert_eq!(websocket_server.num_connections(), 1u64);


    // send request: client -> server
    let client_val = 42;
    let signal = websocket_client.request(DemoClientRequest(client_val)).unwrap();
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Sending);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((
            msg_client_id,
            DemoClientVal::Request(DemoClientRequest(msg_client_val), token)
        )) = websocket_server.next_val()
    else { panic!("server did not receive client msg"); };
    assert_eq!(client_id, msg_client_id);
    assert_eq!(client_id, token.client_id());
    assert_eq!(signal.id(), token.request_id());
    assert_eq!(client_val, msg_client_val);
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Waiting);
    assert!(!token.destination_is_dead());


    // reject
    websocket_server.reject(token);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(DemoServerVal::Reject(request_id)) = websocket_client.next_val()
    else { panic!("client did not receive server msg"); };
    assert_eq!(signal.id(), request_id);
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Rejected);


    // no more outputs
    let None = websocket_server.next_report()
    else { panic!("server should receive no more connection reports"); };
    let None = websocket_client.next_report()
    else { panic!("client should receive no more connection reports"); };
    let None = websocket_client.next_val()
    else { panic!("client should receive no more values"); };
    let None = websocket_client.next_val()
    else { panic!("client should receive no more values"); };
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn request_dropped()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
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
            bevy_simplenet::ServerConfig::default(),
        );

    let websocket_url = websocket_server.url();
    assert_eq!(websocket_server.num_connections(), 0u64);


    // make client
    let websocket_client = client_demo_factory().new_client(
            client_runtime.clone(),
            websocket_url.clone(),
            bevy_simplenet::AuthRequest::None{ client_id: 44718u128 },
            bevy_simplenet::ClientConfig{
                reconnect_on_disconnect   : true,
                reconnect_on_server_close : true,  //we want client to reconnect but fail to get response
                ..Default::default()
            },
            ()
        );
    assert!(!websocket_client.is_dead());

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(bevy_simplenet::ServerReport::Connected(client_id, _, ())) = websocket_server.next_report()
    else { panic!("server should be connected once client is connected"); };
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    assert_eq!(websocket_server.num_connections(), 1u64);


    // send request: client -> server
    let client_val = 42;
    let signal = websocket_client.request(DemoClientRequest(client_val)).unwrap();
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Sending);

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some((
            msg_client_id,
            DemoClientVal::Request(DemoClientRequest(msg_client_val), token)
        )) = websocket_server.next_val()
    else { panic!("server did not receive client msg"); };
    assert_eq!(client_id, msg_client_id);
    assert_eq!(client_id, token.client_id());
    assert_eq!(signal.id(), token.request_id());
    assert_eq!(client_val, msg_client_val);
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Waiting);
    assert!(!token.destination_is_dead());


    // server closes client
    let closure_frame =
        ezsockets::CloseFrame{
            code   : ezsockets::CloseCode::Normal,
            reason : String::from("test")
        };
    websocket_server.close_session(client_id, closure_frame).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    let Some(bevy_simplenet::ClientReport::ClosedByServer(_)) = websocket_client.next_report()
    else { panic!("client should be closed by server"); };
    let Some(bevy_simplenet::ServerReport::Disconnected(dc_client_id)) = websocket_server.next_report()
    else { panic!("server should be disconnected after client is disconnected (by server)"); };
    assert_eq!(client_id, dc_client_id);


    // client auto-reconnects
    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery
    let Some(bevy_simplenet::ServerReport::Connected(_, _, ())) = websocket_server.next_report()
    else { panic!("server should be connected once client is connected"); };
    let Some(bevy_simplenet::ClientReport::Connected) = websocket_client.next_report()
    else { panic!("client should be connected to server"); };
    assert_eq!(websocket_server.num_connections(), 1u64);


    // request has updated
    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery
    assert!(token.destination_is_dead());
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::ResponseLost);


    // try to acknowledge the token (nothing should happen since the original target session was replaced)
    websocket_server.acknowledge(token).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery
    assert_eq!(signal.status(), bevy_simplenet::RequestStatus::ResponseLost);


    // no more outputs
    let None = websocket_server.next_report()
    else { panic!("server should receive no more connection reports"); };
    let None = websocket_client.next_report()
    else { panic!("client should receive no more connection reports"); };
    let None = websocket_client.next_val()
    else { panic!("client should receive no more values"); };
    let None = websocket_client.next_val()
    else { panic!("client should receive no more values"); };
}

//-------------------------------------------------------------------------------------------------------------------
