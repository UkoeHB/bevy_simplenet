# Bevy SimpleNet {INITIAL RELEASE IS WIP}

Provides a bi-directional server/client channel implemented over websockets that can be stored in bevy resources: `Res<Client>`, `Res<Server`. This crate is suitable for user authentication, talking to a matchmaking service, communicating between micro-services, games that don't have strict latency requirements, etc.

**Warning**: This crate requires nightly rust (see open TODOs).


## Features

- `default`: `bevy`, `client`, `server`
- `bevy`: derives `bevy_ecs::system::Resource` on `Client` and `Server`
- `client`: enables simplenet clients
- `server`: enables simplenet servers
- `tls-rustls`: enables TLS for servers via [`rustls`](https://crates.io/crates/rustls)
- `tls-openssl`: enables TLS for servers via [`OpenSSL`](https://crates.io/crates/openssl)


## Usage notes

- Uses `enfync` runtimes to create servers/clients (`tokio` or `wasm_bindgen_futures::spawn_local()`). The backend is `ezsockets` (TODO: WASM client backend).
- A client's `AuthRequest` type must match the corresponding server's `Authenticator` type.
- Server session ids equal client ids. Client ids are defined by clients via their `AuthRequest` when connecting to a server. This means multiple sessions from the same client auth request will have the same session id. Connections will be rejected if an id is already connected.
- Connect messages will be reused for all reconnect attempts by clients, so they should be treated as static data.
- Server or client messages may fail to send if the underlying connection is broken. Clients can use the [`ezsockets::MessageSignal`] returned from [`Client::send()`] to track the status of a message. Message tracking is not currently available for servers.
- Tracing levels assume the server is trusted and clients are not trusted.



## Usage

```rust
// path shortcuts
use serde::{Deserialize, Serialize};
use std::sync::Arc;


// define a channel
// - it is recommended to make server/client factories with baked-in protocol versions (e.g.
//   with env!("CARGO_PKG_VERSION"))
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
        enfync::builtin::native::TokioHandle::default(),
        "127.0.0.1:0",
        bevy_simplenet::AcceptorConfig::Default,
        bevy_simplenet::Authenticator::None,
        bevy_simplenet::ServerConfig{
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
let client = enfync::blocking::extract(client_factory().new_client(
        enfync::builtin::Handle::default(),
        server.url(),
        bevy_simplenet::AuthRequest::None{ client_id },
        bevy_simplenet::ClientConfig::default(),
        ConnectMsg(String::from("hello"))
    )).unwrap();
std::thread::sleep(std::time::Duration::from_millis(15));  //wait for async machinery


// read connection messages
let bevy_simplenet::ServerReport::Connected(client_id, connect_msg) =
    server.next_report().unwrap() else { panic!("client not connected to server"); };
let bevy_simplenet::ClientReport::Connected =
    client.next_report().unwrap() else { panic!("client not connected to server"); };
assert_eq!(connect_msg.0, String::from("hello"));


// send message: client -> server
let signal = client.send(&ClientMsg(42)).unwrap();
assert_eq!(signal.status(), ezsockets::MessageStatus::Sending);
std::thread::sleep(std::time::Duration::from_millis(15));  //wait for async machinery
assert_eq!(signal.status(), ezsockets::MessageStatus::Sent);


// read message from client
let (msg_client_id, ClientMsg(msg_client_val)) = server.next_msg().unwrap();
assert_eq!(msg_client_id, client_id);
assert_eq!(msg_client_val, 42);


// send message: server -> client
server.send(client_id, ServerMsg(24)).unwrap();
std::thread::sleep(std::time::Duration::from_millis(15));  //wait for async machinery


// read message from server
let ServerMsg(msg_server_val) = client.next_msg().unwrap();
assert_eq!(msg_server_val, 24);


// client closes itself
client.close();
std::thread::sleep(std::time::Duration::from_millis(15));  //wait for async machinery


// read disconnection messages
let bevy_simplenet::ServerReport::Disconnected(client_id) = server.next_report().unwrap()
else { panic!("client not disconnected"); };
let bevy_simplenet::ClientReport::ClosedBySelf = client.next_report().unwrap()
else { panic!("client not closed by self"); };
```



## TODOs

- Implement `AuthToken`:
    - client id = hash(client key)
    - auth key signs { client id, token expiry }
    - client key signs { auth signature }
- The server should count connections to better support authentication workflows that use an external service to issue auth tokens only if the server is not over-subscribed. Auth tokens should include an expiration time so disconnected clients can be forced to reconnect via the auth service.
- Use const generics to bake protocol versions into `Server` and `Client` directly, instead of relying on factories (currently blocked by lack of robust compiler support). Ultimately this will allow switching to stable rust.
- Add WASM-compatible client backend (see [this crate](https://github.com/workflow-rs/workflow-rs) or [this crate](https://docs.rs/ws_stream_wasm/latest/ws_stream_wasm/)).
- Message status tracking for server messages. This may require changes to `ezsockets` in order to inject a `MessageSignal` insantiated in the `Server::send()` method.



## Bevy compatability

| bevy   | bevy_simplenet |
|--------|----------------|
| 0.11   | master         |
