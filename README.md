# Bevy Simplenet

Provides a bi-directional server/client channel implemented over websockets. This crate is suitable for user authentication, talking to a matchmaking service, communicating between micro-services, games that don't have strict latency requirements, etc.

- Client/server channel includes one-shot messages and a request/response API.
- Client message statuses can be tracked.
- Clients automatically work on native and WASM targets.
- Clients can be authenticated by the server (WIP).
- Provides optional server TLS.

Check out the example for a demonstration of how to build a Bevy client using this crate.

Check out [bevy_simplenet_events](https://github.com/UkoeHB/bevy_simplenet_events) for an event-based framework for networking that builds on this crate.



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
- Client ids are defined by clients via their [`AuthRequest`](bevy_simplenet::AuthRequest) when connecting to a server. Connections will be rejected if an id is already connected.
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
    let message_signal = client.send(TestClientMsg(42));
}
```

Send a request.

```rust
fn send_client_request(client: Client<TestChannel>)
{
    let request_signal = client.request(TestClientRequest(24));
}
```

### Sending from the Server

Send a message.

```rust
fn send_server_message(server: Server<TestChannel>)
{
    server.send(0u128, TestServerMsg(111));
}
```

Send a response.

```rust
fn send_server_response(In(token): In<RequestToken>, server: Server<TestChannel>)
{
    server.respond(token, TestServerResponse(1));
}
```

### Reading on the client

```rust
fn read_on_client(client: &mut Client<TestChannel>)
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
fn read_on_server(server: &mut Server<TestChannel>)
{
    while let Some((client_id, server_event)) = server.next()
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


## Client authentication

Servers have three options for authentication clients:
- `Authentication::None`: All connections are valid.
- `Authentication::Secret`: A connection is valid if the client provides `AuthRequest::Secret` with a matching secret.
- `Authentication::Token`: A connection is valid if the client provides `AuthRequest::Token` with a token produced by your backend.

Generating and managing auth tokens is very simple.

1. Generate auth keys in your backend and setup a server.
```rust
let (privkey, pubkey) = bevy_simplenet::generate_auth_token_keys();

let server = server_factory().new_server(
    enfync::builtin::native::TokioHandle::default(),
    "127.0.0.1:0",
    AcceptorConfig::Default,
    Authenticator::Token{pubkey},
    ServerConfig::default(),
);
```

Typically the keypair will be generated on one frontend server, and the `pubkey` will be sent to other servers for running `bevy_simplenet` servers that will authenticate tokens.

2. Client sends their credentials (e.g. login name and password) to your frontend.

3. Your frontend validates the credentials and produces an auth token allowing the user's client id `123u128` to connect to your backend.
```rust
// This token expires after 10 seconds.
let token = bevy_simplenet::make_auth_token_from_lifetime(&privkey, 10, 123u128);
```

You send this token to the client.

4. Client makes a `bevy_simplenet` client using the received token (the `server_url` can be transmitted alongside the token)).
```rust
let client = client_factory().new_client(
    enfync::builtin::Handle::default(),
    server_url,
    AuthRequest::Token{ token },
    ClientConfig::default(),
    TestConnectMsg(String::from("hello"))
);
```

Note that when the token has expired, `bevy_simplenet` clients will fail all automatic reconnect attempts (e.g. after a network error). You should adjust `ClientConfig::max_reconnect_attempts` and `ClientConfig::reconnect_interval` so the client will shut down once the token has expired. Then when the client emits `ClientEvent::Report(ClientReport::IsDead(_))` you can request a new auth token and set up a new client.

It is recommended to set a relatively low auth token expiry if you are concerned about DoS from clients clogging up the server's capacity, or if you have a force-disconnect/blacklist mechanism on the server.


## TODOs

- Add server shut down procedure.
- Use const generics to bake protocol versions into `Server` and `Client` directly, instead of relying on factories (currently blocked by lack of robust compiler support).



## Bevy compatability

| bevy   | bevy_simplenet |
|--------|----------------|
| 0.14   | v0.12 - master |
| 0.13   | v0.9 - v0.11   |
| 0.12   | v0.5 - v0.8    |
| 0.11   | v0 - v0.4      |
