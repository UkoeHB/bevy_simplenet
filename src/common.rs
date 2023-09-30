//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::net::SocketAddr;
use std::time::Duration;

//-------------------------------------------------------------------------------------------------------------------

pub type SessionID = u128;

//-------------------------------------------------------------------------------------------------------------------

pub(crate) const VERSION_MSG_KEY: &'static str = "v";
pub(crate) const TYPE_MSG_KEY: &'static str    = "t";
pub(crate) const AUTH_MSG_KEY: &'static str    = "a";
pub(crate) const CONNECT_MSG_KEY: &'static str = "c";

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by servers when a client connects/disconnects.
#[derive(Debug, Clone)]
pub enum ServerReport<ConnectMsg: Debug + Clone>
{
    Connected(SessionID, ConnectMsg),
    Disconnected(SessionID)
}

//-------------------------------------------------------------------------------------------------------------------

/// Server-enforced constraints on client connections.
#[derive(Debug, Copy, Clone)]
pub struct ServerConfig
{
    /// Max number of concurrent client connections.
    pub max_connections: u32,
    /// Max message size allowed from clients (bytes).
    pub max_msg_size: u32,
    /// Rate limit for messages received from a session.
    pub rate_limit_config: RateLimitConfig,
    /// Duration between socket heartbeat pings if the connection is inactive.
    pub heartbeat_interval: Duration,
    /// Duration after which a socket will shut down if the connection is inactive.
    pub keepalive_timeout: Duration,
}

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by clients when they connect/disconnect/shut down.
#[derive(Debug, Clone)]
pub enum ClientReport
{
    Connected,
    Disconnected,
    ClosedByServer(Option<ezsockets::CloseFrame>),
    ClosedBySelf,
    IsDead,
}

//-------------------------------------------------------------------------------------------------------------------

/// Config controlling how clients respond to connection events
#[derive(Debug)]
pub struct ClientConfig
{
    /// Try to reconnect if the client is disconnected (`true` by default).
    pub reconnect_on_disconnect: bool,
    /// Try to reconnect if the client is closed by the server (`false` by default).
    pub reconnect_on_server_close: bool,
    /// Reconnect interval (delay between reconnect attempts)
    pub reconnect_interval: Duration,
}

impl Default for ClientConfig
{
    fn default() -> ClientConfig
    {
        ClientConfig{
                reconnect_on_disconnect   : true,
                reconnect_on_server_close : false,
                reconnect_interval        : Duration::from_secs(2)
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EnvType
{
    Native,
    Wasm,
}

pub(crate) fn env_type() -> EnvType
{
    #[cfg(not(target_family = "wasm"))]
    { EnvType::Native }

    #[cfg(target_family = "wasm")]
    { EnvType::Wasm }
}

pub(crate) fn env_type_as_str(env_type: EnvType) -> &'static str
{
    match env_type
    {
        EnvType::Native => "0",
        EnvType::Wasm   => "1",
    }
}

pub(crate) fn env_type_from_str(env_type: &str) -> Option<EnvType>
{
    match env_type
    {
        "0" => Some(EnvType::Native),
        "1" => Some(EnvType::Wasm),
        _   => None
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SessionTargetMsg<I: Debug + Clone, T: Debug + Clone>
{
    pub(crate) id  : I,
    pub(crate) msg : T
}

impl<I: Debug + Clone, T: Debug + Clone> SessionTargetMsg<I, T>
{
    pub(crate) fn new(id: I, msg: T) -> SessionTargetMsg<I, T> { SessionTargetMsg::<I, T> { id, msg } }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct SessionSourceMsg<I: Debug + Clone, T: Debug + Clone>
{
    pub(crate) id  : I,
    pub(crate) msg : T
}

impl<I: Debug + Clone, T: Debug + Clone> SessionSourceMsg<I, T>
{
    pub(crate) fn new(id: I, msg: T) -> SessionSourceMsg<I, T> { SessionSourceMsg::<I, T> { id, msg } }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) enum SessionCommand<ServerMsg: Debug + Clone>
{
    SendMsg(ServerMsg),
    Close(ezsockets::CloseFrame)
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
