# Bevy Simplenet

Provides a bi-directional server/client channel implemented over websockets. This crate is suitable for user authentication, talking to a matchmaking service, communicating between micro-services, games that don't have strict latency requirements, etc.

- Client/server channel includes one-shot messages and a request/response API.
- Client message status tracking.
- Clients automatically work on native and WASM targets.
- Client authentication (WIP).
- Optional server TLS.

Check out the example for a demonstration of how to build a Bevy client using this crate.

This crate requires nightly rust.



## Features

- `default`: `bevy`, `client`, `server`
- `bevy`: derives `Resource` on [`Client`] and [`Server`]
- `client`: enables clients (native and WASM targets)
- `server`: enables servers (native-only targets)
- `tls-rustls`: enables TLS for servers via [`rustls`](https://crates.io/crates/rustls)
- `tls-openssl`: enables TLS for servers via [`OpenSSL`](https://crates.io/crates/openssl)



## WASM

On WASM targets the client backend will not update while any other tasks are running. You must either build an IO-oriented application that naturally spends a lot of time polling tasks, or manually release the main thread periodically (e.g. with `web_sys::Window::set_timeout_with_callback_and_timeout_and_arguments_0()`). For Bevy apps the latter happens automatically at the end of every app update/tick (see the `bevy::app::ScheduleRunnerPlugin` [implementation](https://github.com/bevyengine/bevy)).



## Usage notes

- Servers and clients must be created with [enfync](https://crates.io/crates/enfync) runtimes. The backend is [ezsockets](https://github.com/gbaranski/ezsockets).
- A client's [`AuthRequest`] type must match the corresponding server's [`Authenticator`] type.
- Client ids are defined by clients via their [`AuthRequest`] when connecting to a server. This means multiple sessions from the same client will have the same session id. Connections will be rejected if an id is already connected.
- Client connect messages will be cloned for all reconnect attempts, so they should be treated as static data.
- Server or client messages may fail to send if the underlying connection is broken. Clients can use the signals returned from [`Client::send()`] and [`Client::request()`] to track the status of a message. Message tracking is not currently available for servers.
- Tracing levels assume the server is trusted and clients are not trusted.



## Example

```rust
// path shortcuts
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;


// define a channel
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestConnectMsg(pub String);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestServerMsg(pub u64);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestClientMsg(pub u64);

#[derive(Debug, Clone)]
pub struct TestChannel;
impl bevy_simplenet::ChannelPack for TestChannel
{
    type ConnectMsg = TestConnectMsg;
    type ServerMsg = TestServerMsg;
    type ServerResponse = ();
    type ClientMsg = TestClientMsg;
    type ClientRequest = ();
}

type TestServer = bevy_simplenet::Server<TestChannel>;
type TestClient = bevy_simplenet::Client<TestChannel>;
type TestServerVal = bevy_simplenet::ServerValFrom<TestChannel>;
type TestClientVal = bevy_simplenet::ClientValFrom<TestChannel>;

fn server_factory() -> bevy_simplenet::ServerFactory<TestChannel>
{
    // It is recommended to make server/client factories with baked-in protocol versions (e.g.
    //   with env!("CARGO_PKG_VERSION")).
    bevy_simplenet::ServerFactory::<TestChannel>::new("test")
}

fn client_factory() -> bevy_simplenet::ClientFactory<TestChannel>
{
    // You must use the same protocol version string as the server.
    bevy_simplenet::ClientFactory::<TestChannel>::new("test")
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
        bevy_simplenet::ServerConfig::default(),
    );
assert_eq!(server.num_connections(), 0u64);


// sleep duration for async machinery
let sleep_duration = Duration::from_millis(15);


// make a client
let client_id = 0u128;
let client = client_factory().new_client(
        enfync::builtin::Handle::default(),  //automatically selects native/WASM runtime
        server.url(),
        bevy_simplenet::AuthRequest::None{ client_id },
        bevy_simplenet::ClientConfig::default(),
        TestConnectMsg(String::from("hello"))
    );
sleep(sleep_duration);
assert_eq!(server.num_connections(), 1u64);


// read connection messages
let bevy_simplenet::ServerReport::Connected(client_id, env_type, connect_msg) =
    server.next_report().unwrap() else { panic!("client not connected to server"); };
let bevy_simplenet::ClientReport::Connected =
    client.next_report().unwrap() else { panic!("client not connected to server"); };
assert_eq!(env_type, bevy_simplenet::EnvType::Native);
assert_eq!(connect_msg.0, String::from("hello"));


// send message: client -> server
let signal = client.send(TestClientMsg(42)).unwrap();
assert_eq!(signal.status(), bevy_simplenet::MessageStatus::Sending);
sleep(sleep_duration);
assert_eq!(signal.status(), bevy_simplenet::MessageStatus::Sent);


// read message from client
let (
        msg_client_id,
        TestClientVal::Msg(TestClientMsg(msg_val))
    ) = server.next_val().unwrap()
else { todo!() };
assert_eq!(msg_client_id, client_id);
assert_eq!(msg_val, 42);


// send message: server -> client
server.send(client_id, TestServerMsg(24)).unwrap();
sleep(sleep_duration);


// read message from server
let TestServerVal::Msg(TestServerMsg(msg_server_val)) = client.next_val().unwrap()
else { todo!() };
assert_eq!(msg_server_val, 24);


// send request to server
let signal = client.request(()).unwrap();
assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Sending);
sleep(sleep_duration);
assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Waiting);


// read request from client
let (_, TestClientVal::Request((), request_token)) = server.next_val().unwrap()
else { todo!() };


// acknowledge the request (consumes the token without sending a Response)
server.acknowledge(request_token).unwrap();
sleep(sleep_duration);
assert_eq!(signal.status(), bevy_simplenet::RequestStatus::Acknowledged);


// read acknowledgement from server
let TestServerVal::Ack(request_id) = client.next_val().unwrap()
else { todo!() };
assert_eq!(request_id, signal.id());


// client closes itself
client.close();
sleep(sleep_duration);
assert_eq!(server.num_connections(), 0u64);


// read disconnection messages
let bevy_simplenet::ServerReport::Disconnected(client_id) = server.next_report().unwrap()
else { panic!("client not disconnected"); };
let bevy_simplenet::ClientReport::ClosedBySelf = client.next_report().unwrap()
else { panic!("client not closed by self"); };
let bevy_simplenet::ClientReport::IsDead = client.next_report().unwrap()
else { panic!("client not dead"); };
```



## TODOs

- This crate causes linker errors when the `bevy/dynamic_linking` feature is enabled.
- Implement `AuthToken` for client/server authentication.
- Use const generics to bake protocol versions into `Server` and `Client` directly, instead of relying on factories (currently blocked by lack of robust compiler support).
- Move to stable rust once `HashMap::extract_if()` is stabilized.



## Bevy compatability

| bevy   | bevy_simplenet |
|--------|----------------|
| 0.11   | master         |
