//local shortcuts

//third-party shortcuts
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::net::SocketAddr;

//-------------------------------------------------------------------------------------------------------------------

/// Id for client sessions on the server. Equals the client id.
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

/// Environment type of a binary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EnvType
{
    Native,
    Wasm,
}

/// Get the binary's target environment.
pub fn env_type() -> EnvType
{
    #[cfg(not(target_family = "wasm"))]
    { EnvType::Native }

    #[cfg(target_family = "wasm")]
    { EnvType::Wasm }
}

/// Convert [`EnvType`] to a string.
pub fn env_type_as_str(env_type: EnvType) -> &'static str
{
    match env_type
    {
        EnvType::Native => "0",
        EnvType::Wasm   => "1",
    }
}

/// Get an [`EnvType`] from a string.
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

/// Make a websocket url: `{ws, wss}://[ip:port]/ws`.
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
