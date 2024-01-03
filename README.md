# Bevy Simplenet

Provides a bi-directional server/client channel implemented over websockets. This crate is suitable for user authentication, talking to a matchmaking service, communicating between micro-services, games that don't have strict latency requirements, etc.

- Client/server channel includes one-shot messages and a request/response API.
- Client message statuses can be tracked.
- Clients automatically work on native and WASM targets.
- Clients can be authenticated by the server (WIP).
- Provides optional server TLS.

Check out the example for a demonstration of how to build a Bevy client using this crate.

Check out [bevy_simplenet_events](https://github.com/UkoeHB/bevy_simplenet_events) for an event-based framework for networking that builds on this crate.

This crate requires nightly rust.



## Features

- `default`: includes `bevy`, `client`, `server` features
- `bevy`: derives `Resource` on [`Client`](bevy_simplenet::Client) and [`Server`](bevy_simplenet::Server)
- `client`: enables clients (native and WASM targets)
- `server`: enables servers (native-only targets)
- `tls-rustls`: enables TLS for servers via [`rustls`](https://crates.io/crates/rustls)
- `tls-openssl`: enables TLS for servers via [`OpenSSL`](https://crates.io/crates/openssl)



## WASM

On WASM targets the client backend will not update while any other tasks are running. You must either build an IO-oriented application that naturally spends a lot of time polling tasks, or manually release the main thread periodically (e.g. with `web_sys::Window::set_timeout_with_callback_and_timeout_and_arguments_0()`). For Bevy apps the latter happens automatically at the end of every app update/tick (see the `bevy::app::ScheduleRunnerPlugin` [implementation](https://github.com/bevyengine/bevy)).



## Usage notes

- Servers and clients must be created with [enfync](https://crates.io/crates/enfync) runtimes. The backend is [ezsockets](https://github.com/gbaranski/ezsockets).
- A client's [`AuthRequest`](bevy_simplenet::AuthRequest) type must match the corresponding server's [`Authenticator`](bevy_simplenet::Authenticator) type.
- Client ids are defined by clients via their [`AuthRequest`](bevy_simplenet::AuthRequest) when connecting to a server. This means multiple sessions from the same client will have the same session id. Connections will be rejected if an id is already connected.
- Client connect messages will be cloned for all reconnect attempts, so they should be treated as static data.
- Server or client messages may fail to send if the underlying connection is broken. Clients can use the signals returned from [`Client::send()`](bevy_simplenet::Client::send) and [`Client::request()`](bevy_simplenet::Client::request) to track the status of a message. Client request results will always be emitted by [`Client::next()`](bevy_simplenet::Client::next). Message tracking is not available for servers.
- Tracing levels assume the server is trusted and clients are not trusted.



## Example

### Setup

**Common**

Define a channel.

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestConnectMsg(pub String);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestServerMsg(pub u64);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestServerResponse(pub u64);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestClientMsg(pub u64);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestClientRequest(pub u64);

#[derive(Debug, Clone)]
pub struct TestChannel;
impl ChannelPack for TestChannel
{
    type ConnectMsg = TestConnectMsg;
    type ServerMsg = TestServerMsg;
    type ServerResponse = TestServerResponse;
    type ClientMsg = TestClientMsg;
    type ClientRequest = TestClientRequest;
}
```

**Server**

Prepare to make servers.

```rust
type TestServerEvent = ServerEventFrom<TestChannel>;

fn server_factory() -> ServerFactory<TestChannel>
{
    // It is recommended to make server/client factories with baked-in protocol versions (e.g.
    //   with env!("CARGO_PKG_VERSION")).
    ServerFactory::<TestChannel>::new("test")
}
```

Make a server and insert it into an app.

```rust
fn setup_server(mut commands: Commands)
{
    let server = server_factory().new_server(
            enfync::builtin::native::TokioHandle::default(),
            "127.0.0.1:0",
            AcceptorConfig::Default,
            Authenticator::None,
            ServerConfig::default(),
        );
    commands.insert_resource(server);
}
```

**Client**

Prepare to make clients.

```rust
type TestClientEvent = ClientEventFrom<TestChannel>;

fn client_factory() -> ClientFactory<TestChannel>
{
    // You must use the same protocol version string as the server factory.
    ClientFactory::<TestChannel>::new("test")
}
```

Make a client and insert it into an app.

```rust
fn setup_client(mut commands: Commands)
{
    let client_id = 0u128;
    let client = client_factory().new_client(
            enfync::builtin::Handle::default(),  //automatically selects native/WASM runtime
            server.url(),
            AuthRequest::None{ client_id },
            ClientConfig::default(),
            TestConnectMsg(String::from("hello"))
        );
    commands.insert_resource(client);
}
```

### Sending from the client

Send a message.

```rust
fn send_client_message(client: Client<TestChannel>)
{
    let message_signal = client.send(TestClientMsg(42)).unwrap();
}
```

Send a request.

```rust
fn send_client_request(client: Client<TestChannel>)
{
    let request_signal = client.request(TestClientRequest(24)).unwrap();
}
```

### Sending from the Server

Send a message.

```rust
fn send_server_message(server: Server<TestChannel>)
{
    server.send(0u128, TestServerMsg(111)).unwrap();
}
```

Send a response.

```rust
fn send_server_response(In(token): In<RequestToken>, server: Server<TestChannel>)
{
    server.respond(token, TestServerResponse(1)).unwrap();
}
```

### Reading on the client

```rust
fn read_on_client(client: Client<TestChannel>)
{
    while let Some(client_event) = client.next()
    {
        match client_event
        {
            TestClientEvent::Report(connection_report) => match connection_report
            {
                ClientReport::Connected                => todo!(),
                ClientReport::Disconnected             => todo!(),
                ClientReport::ClosedByServer(reason)   => todo!(),
                ClientReport::ClosedBySelf             => todo!(),
                ClientReport::IsDead(pending_requests) => todo!(),
            }
            TestClientEvent::Msg(message)                   => todo!(),
            TestClientEvent::Response(response, request_id) => todo!(),
            TestClientEvent::Ack(request_id)                => todo!(),
            TestClientEvent::Reject(request_id)             => todo!(),
            TestClientEvent::SendFailed(request_id)         => todo!(),
            TestClientEvent::ResponseLost(request_id)       => todo!(),
        }
    }
}
```

### Reading on the server

```rust
fn read_on_server(server: Server<TestChannel>)
{
    while let Some((session_id, server_event)) = server.next()
    {
        match server_event
        {
            TestServerEvent::Report(connection_report) => match connection_report
            {
                ServerReport::Connected(env, message) => todo!(),
                ServerReport::Disconnected            => todo!(),
            }
            TestServerEvent::Msg(message)            => todo!(),
            TestServerEvent::Request(token, request) => todo!(),
        }
    }
}
```


## TODOs

- Fix race condition that allows sending a client or server message to a new session before the old session's `Disconnected` event has been processed.
- Fix linker errors when the `bevy/dynamic_linking` feature is enabled.
- Implement `AuthToken` for client/server authentication.
- Add server shut down procedure.
- Use const generics to bake protocol versions into `Server` and `Client` directly, instead of relying on factories (currently blocked by lack of robust compiler support).
- Move to stable rust once `HashMap::extract_if()` is stabilized.



## Bevy compatability

| bevy   | bevy_simplenet  |
|--------|-----------------|
| 0.12   | v0.5.0 - master |
| 0.11   | v0 - v0.4.0     |
