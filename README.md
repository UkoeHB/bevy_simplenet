# Bevy SimpleNet

Provides a simple server/client channel implemented over websockets that can be stored in bevy resources: `Res<Client>`, `Res<Server`. It is not recommended to use this crate for game messages, but it may be useful for other networking requirements like authentication, talking to a matchmaking service, communicating between micro-services, etc.

**Warning**: This crate requires nightly rust to enable the `Server::Factory` and `Client::Factory` subtypes (see open TODOs).



## Usage notes

- Uses `tokio` runtimes to create servers/clients. The backend is the `ezsockets` crate on top of `tokio-tungstenite`.
- Session ids equal client ids, which are defined by clients via their `AuthRequest` when connecting to a server. This means multiple sessions from the same client will have the same session id. Connections will be rejected if an id is already connected.
- A client's `AuthRequest` type must match the corresponding server's `Authenticator` type.
- Connect messages will be reused for all reconnect attempts by clients, so they should be treated as static data.
- Tracing levels assume the server is trusted and clients are not trusted.
- When defining a channel, it is recommended to write functions that spit out server/client factories. Those functions can reference the desired protocol version, e.g. the constant `env!("CARGO_PKG_VERSION")`.
- Servers can use TLS via `ezsockets::tungstenite::Acceptor`. See [ezsockets](https://docs.rs/ezsockets/latest/ezsockets/) documentation.



## Usage

```rust
// path shortcuts
use serde::{Deserialize, Serialize};
use std::sync::Arc;


// define a channel
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerMsg(pub u64);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientMsg(pub u64);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectMsg(pub String);

type Server = bevy_simplenet::Server::<ServerMsg, ClientMsg, ConnectMsg>;
type Client = bevy_simplenet::Client::<ServerMsg, ClientMsg, ConnectMsg>;

fn server_factory() -> Server::Factory
{
    Server::Factory::new("test")
}

fn client_factory() -> Client::Factory
{
    Client::Factory::new("test")  //must use same protocol version string as the server
}


// enable tracing (with crate `tracing-subscriber`)
/*
let subscriber = tracing_subscriber::FmtSubscriber::builder()
    .with_max_level(tracing::Level::TRACE)
    .finish();
tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
tracing::info!("README test start");
*/


// make a server
let server = server_factory().new_server(
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
        "127.0.0.1:57633",
        ezsockets::tungstenite::Acceptor::Plain,
        bevy_simplenet::Authenticator::None,
        bevy_simplenet::ConnectionConfig{
            max_connections   : 10,
            max_msg_size      : 10_000,
            rate_limit_config : bevy_simplenet::RateLimitConfig{
                    period    : std::time::Duration::from_millis(15),
                    max_count : 25
                }
        }
    );


// make a client
let client_id = 0u128;
let client = client_factory().new_client(
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
        url::Url::parse("ws://127.0.0.1:57633/websocket").expect("invalid websocket url"),
        bevy_simplenet::AuthRequest::None{ client_id },
        ConnectMsg(String::from("hello"))
    ).extract().unwrap().unwrap();  //fails if could not connect to server
std::thread::sleep(std::time::Duration::from_millis(15));  //wait for async machinery


// read connection message
let Some(bevy_simplenet::ConnectionReport::Connected(client_id, connect_msg)) =
    server.try_get_next_connection_report()
else { panic!("did not receive connection report"); };
assert_eq!(connect_msg.0, String::from("hello"));


// send message: client -> server
client.send_msg(&ClientMsg(42)).unwrap();
std::thread::sleep(std::time::Duration::from_millis(15));  //wait for async machinery


// read message from client
let (msg_client_id, ClientMsg(msg_client_val)) = server.try_get_next_msg().unwrap();
assert_eq!(msg_client_id, client_id);
assert_eq!(msg_client_val, 42);


// send message: server -> client
server.send_msg(client_id, ServerMsg(24)).unwrap();
std::thread::sleep(std::time::Duration::from_millis(15));  //wait for async machinery


// read message from server
let ServerMsg(msg_server_val) = client.try_get_next_msg().unwrap();
assert_eq!(msg_server_val, 24);
```



## TODOs

- [ezsockets](https://docs.rs/ezsockets/latest/ezsockets/) backend needs to be upgraded to:
    - remove panics from public interfaces
    - emit closure reasons from clients
    - allow clients to choose close vs reconnect when disconnected (e.g. close if server commands it, reconnect if there is a network issue) (follow-up todo: add reconnect policy to client config)
    - remove `async` requirement from `Client::close()`
- Implement `AuthToken`:
    - client id = hash(client key)
    - auth key signs { client id, token expiry }
    - client key signs { auth signature }
- The server should count connections to better support authentication workflows that use an external service to issue auth tokens only if the server is not over-subscribed. Auth tokens should include an expiration time so disconnected clients can be forced to reconnect via the auth service.
- Use const generics to bake protocol versions into `Server` and `Client` directly, instead of relying on factories (currently blocked by lack of robust compiler support). Ultimately this will allow switching to stable rust.



## Comments

- This crate does not use `rustfmt`.



## Bevy compatability

| bevy   | bevy_simplenet |
|--------|--------------|
| 0.11.0 | todo         |
| 0.10.1 | 0.1          |
