//local shortcuts

//third-party shortcuts
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::net::SocketAddr;

//-------------------------------------------------------------------------------------------------------------------

pub(crate) const VERSION_MSG_KEY : &'static str = "v";
pub(crate) const TYPE_MSG_KEY    : &'static str = "t";
pub(crate) const AUTH_MSG_KEY    : &'static str = "a";
pub(crate) const CONNECT_MSG_KEY : &'static str = "c";

//-------------------------------------------------------------------------------------------------------------------

pub type SessionID = u128;

//-------------------------------------------------------------------------------------------------------------------

/// Represents the message types that can be sent between a client and server.
pub trait MsgPack: Debug + 'static
{
    /// A client sends this to a server as part of connection requests.
    type ConnectMsg: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;

    /// A client one-shot message.
    type ClientMsg: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
    /// A client request, the server should send a response or acknowledge it.
    type ClientRequest: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;

    /// A server one-shot message.
    type ServerMsg: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
    /// A server response to a client request.
    type ServerResponse: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
}

//-------------------------------------------------------------------------------------------------------------------

/// A server message that may be received by a client.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerVal<ServerMsg, ServerResponse>
where
    ServerMsg: Clone + Debug + Send + Sync,
    ServerResponse: Clone + Debug + Send + Sync,
{
    /// A one-shot server message.
    Msg(ServerMsg),
    /// A response to a client request with the given id.
    Response(ServerResponse, u64),
    /// Acknowledges receiving a client request with the given id.
    ///
    /// Will not be followed by a subsequent response (you either get a response or an ack).
    Ack(u64),
}

//-------------------------------------------------------------------------------------------------------------------

pub type ServerValFromPack<Msgs> = ServerVal<
        <Msgs as MsgPack>::ServerMsg,
        <Msgs as MsgPack>::ServerResponse,
    >;

//-------------------------------------------------------------------------------------------------------------------

/// A client message that may be received by a server.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientVal<ClientMsg, ClientRequest>
where
    ClientMsg: Clone + Debug + Send + Sync,
    ClientRequest: Clone + Debug + Send + Sync,
{
    /// A one-shot client message.
    Msg(ClientMsg),
    /// A request the server should reply to with a response or ack.
    Request(ClientRequest, u64),
}

//-------------------------------------------------------------------------------------------------------------------

pub type ClientValFromPack<Msgs> = ClientVal<
        <Msgs as MsgPack>::ClientMsg,
        <Msgs as MsgPack>::ClientRequest,
    >;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvType
{
    Native,
    Wasm,
}

pub fn env_type() -> EnvType
{
    #[cfg(not(target_family = "wasm"))]
    { EnvType::Native }

    #[cfg(target_family = "wasm")]
    { EnvType::Wasm }
}

pub fn env_type_as_str(env_type: EnvType) -> &'static str
{
    match env_type
    {
        EnvType::Native => "0",
        EnvType::Wasm   => "1",
    }
}

pub fn env_type_from_str(env_type: &str) -> Option<EnvType>
{
    match env_type
    {
        "0" => Some(EnvType::Native),
        "1" => Some(EnvType::Wasm),
        _   => None
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Make a websocket url: {ws, wss}://[ip:port]/ws.
pub fn make_websocket_url(with_tls: bool, address: SocketAddr) -> Result<url::Url, ()>
{
    let mut url = url::Url::parse("https://example.net").map_err(|_| ())?;
    let scheme = match with_tls { true => "wss", false => "ws" };
    url.set_scheme(scheme)?;
    url.set_ip_host(address.ip())?;
    url.set_port(Some(address.port()))?;
    url.set_path("/ws");
    Ok(url)
}

//-------------------------------------------------------------------------------------------------------------------
