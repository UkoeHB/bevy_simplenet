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

fn authentication_test(authenticator: bevy_simplenet::Authenticator, auth_request: bevy_simplenet::AuthRequest) -> bool
{
    // prepare tokio runtimes for server and client
    let server_runtime = enfync::builtin::native::TokioHandle::default();
    let client_runtime = enfync::builtin::Handle::default();

    // launch websocket server
    let websocket_server = server_demo_factory().new_server(
            server_runtime,
            "127.0.0.1:0",
            bevy_simplenet::AcceptorConfig::Default,
            authenticator,
            bevy_simplenet::ServerConfig{
                max_connections   : 10,
                max_msg_size      : 10_000,
                rate_limit_config : bevy_simplenet::RateLimitConfig{
                        period    : std::time::Duration::from_millis(15),
                        max_count : 25
                    },
                heartbeat_interval : std::time::Duration::from_secs(5),
                keepalive_timeout  : std::time::Duration::from_secs(10),
            }
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

    std::thread::sleep(std::time::Duration::from_millis(25));  //wait for async machinery

    // return connection result
    return !websocket_client.is_dead();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
fn bevy_simplenet_authentication()
{
    // prep authenticators
    let none_authenticator = bevy_simplenet::Authenticator::None;
    let secret_authenticator_a = bevy_simplenet::Authenticator::Secret{secret: (0u128).to_le_bytes()};
    let secret_authenticator_b = bevy_simplenet::Authenticator::Secret{secret: (1u128).to_le_bytes()};
    //let token_authenticator = bevy_simplenet::Authenticator::Token{};

    // prep auth requests
    let none_request = bevy_simplenet::AuthRequest::None{client_id: 0u128};
    let secret_request_a = bevy_simplenet::AuthRequest::Secret{client_id: 1u128, secret: (0u128).to_le_bytes()};

    // test cases
    assert!(authentication_test(none_authenticator.clone(), none_request.clone()));
    assert!(!authentication_test(none_authenticator.clone(), secret_request_a.clone()));
    assert!(!authentication_test(secret_authenticator_a.clone(), none_request.clone()));

    assert!(authentication_test(secret_authenticator_a.clone(), secret_request_a.clone()));
    assert!(!authentication_test(secret_authenticator_b.clone(), secret_request_a.clone()));
}

//-------------------------------------------------------------------------------------------------------------------
