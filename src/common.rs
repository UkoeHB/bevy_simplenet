//local shortcuts
use crate::*;

//third-party shortcuts
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::net::SocketAddr;

//-------------------------------------------------------------------------------------------------------------------

pub type SessionID = u128;

//-------------------------------------------------------------------------------------------------------------------

/// Represents the message types that can be sent between a client and server.
pub trait ChannelPack: Clone + Debug + 'static
{
    /// A client sends this to a server as part of connection requests.
    ///
    /// Note that a client's connect message is defined when creating the client and can't be modified for
    /// reconnect attempts.
    type ConnectMsg: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;

    /// A server one-shot message.
    type ServerMsg: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
    /// A server response to a client request.
    type ServerResponse: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;

    /// A client one-shot message.
    type ClientMsg: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
    /// A client request. The server may respond to it, acknowledge it, or reject it.
    type ClientRequest: Clone + Debug + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
}

//-------------------------------------------------------------------------------------------------------------------

/// A server value that may be received by a client.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerVal<ServerMsg, ServerResponse>
{
    /// A one-shot server message.
    Msg(ServerMsg),
    /// A response to a client request.
    Response(ServerResponse, u64),
    /// Acknowledges receiving a client request.
    ///
    /// Will not be followed by a subsequent response (you either get a response, ack, or rejection).
    Ack(u64),
    /// Rejects a client request.
    Reject(u64),
}

impl<ServerMsg, ServerResponse> ServerVal<ServerMsg, ServerResponse>
{
    /// Convert a server value into a request status.
    pub(crate) fn into_request_status(&self) -> Option<(u64, RequestStatus)>
    {
        match self
        {
            Self::Msg(_)          => None,
            Self::Response(_, id) => Some((*id, RequestStatus::Responded)),
            Self::Ack(id)         => Some((*id, RequestStatus::Acknowledged)),
            Self::Reject(id)      => Some((*id, RequestStatus::Rejected)),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub type ServerValFrom<Channel> = ServerVal<
    <Channel as ChannelPack>::ServerMsg,
    <Channel as ChannelPack>::ServerResponse
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
