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
type _DemoClientEvent = bevy_simplenet::ClientEventFrom<DemoChannel>;
type _DemoServerEvent = bevy_simplenet::ServerEventFrom<DemoChannel>;

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

fn authentication_test(authenticator: bevy_simplenet::Authenticator, auth_request: bevy_simplenet::AuthRequest) -> bool
{
    // prepare tokio runtimes for server and client
    let server_runtime = enfync::builtin::native::TokioHandle::default();
    let client_runtime = enfync::builtin::Handle::default();

    // launch websocket server
    let websocket_server = server_demo_factory().new_server(
            server_runtime,
            "127.0.0.1:0",
            bevy_simplenet::AcceptorConfig::default(),
            authenticator,
            bevy_simplenet::ServerConfig::default(),
        );

    // make client
    let websocket_client = client_demo_factory().new_client(
            client_runtime,
            websocket_server.url(),
            auth_request,
            bevy_simplenet::ClientConfig{
                max_initial_connect_attempts: 1usize,
                ..Default::default()
            },
            DemoConnectMsg(String::from("hello"))
        );

    std::thread::sleep(std::time::Duration::from_millis(50));  //wait for async machinery

    // return connection result
    return !websocket_client.is_dead();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
fn authentication()
{
    // prep authenticators
    let none_authenticator = bevy_simplenet::Authenticator::None;

    let secret_authenticator_a = bevy_simplenet::Authenticator::Secret{secret: (0u128).to_le_bytes()};
    let secret_authenticator_b = bevy_simplenet::Authenticator::Secret{secret: (1u128).to_le_bytes()};

    let (token_privkey_a, token_pubkey_a) = bevy_simplenet::generate_auth_token_keys();
    let (_token_privkey_b, token_pubkey_b) = bevy_simplenet::generate_auth_token_keys();
    let token_authenticator_a = bevy_simplenet::Authenticator::Token{pubkey: token_pubkey_a};
    let token_authenticator_b = bevy_simplenet::Authenticator::Token{pubkey: token_pubkey_b};

    // prep auth requests
    let none_request = bevy_simplenet::AuthRequest::None{client_id: 0u128};
    let secret_request_a = bevy_simplenet::AuthRequest::Secret{client_id: 1u128, secret: (0u128).to_le_bytes()};
    let token_a = bevy_simplenet::make_auth_token_from_lifetime(&token_privkey_a, 1, 2u128);
    let token_request_a = bevy_simplenet::AuthRequest::Token{token: token_a};

    // test cases
    assert!(authentication_test(none_authenticator.clone(), none_request.clone()));
    assert!(authentication_test(none_authenticator.clone(), secret_request_a.clone()));
    assert!(authentication_test(none_authenticator.clone(), token_request_a.clone()));

    assert!(authentication_test(secret_authenticator_a.clone(), secret_request_a.clone()));
    assert!(!authentication_test(secret_authenticator_a.clone(), none_request.clone()));
    assert!(!authentication_test(secret_authenticator_a.clone(), token_request_a.clone()));
    assert!(!authentication_test(secret_authenticator_b.clone(), secret_request_a.clone()));

    assert!(authentication_test(token_authenticator_a.clone(), token_request_a.clone()));
    assert!(!authentication_test(token_authenticator_a.clone(), none_request.clone()));
    assert!(!authentication_test(token_authenticator_a.clone(), secret_request_a.clone()));
    assert!(!authentication_test(token_authenticator_b.clone(), token_request_a.clone()));
}

//-------------------------------------------------------------------------------------------------------------------
