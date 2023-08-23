//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::net::SocketAddr;

//-------------------------------------------------------------------------------------------------------------------

pub type SessionID = u128;

//-------------------------------------------------------------------------------------------------------------------

pub(crate) const VERSION_MSG_HEADER: &'static str = "WSCv";
pub(crate) const AUTH_MSG_HEADER: &'static str    = "WSCa";
pub(crate) const CONNECT_MSG_HEADER: &'static str = "WSCc";

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by servers when a client connects/disconnects.
#[derive(Debug, Clone)]
pub enum ServerConnectionReport<ConnectMsg: Debug + Clone>
{
    Connected(SessionID, ConnectMsg),
    Disconnected(SessionID)
}

//-------------------------------------------------------------------------------------------------------------------

/// Server-enforced constraints on client connections.
#[derive(Debug)]
pub struct ServerConnectionConfig
{
    /// Max number of concurrent client connections.
    pub max_connections: u32,
    /// Max message size allowed from clients (bytes).
    pub max_msg_size: u32,
    /// Rate limit for messages received from a session.
    pub rate_limit_config: RateLimitConfig
}

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by clients when they connect/disconnect.
#[derive(Debug, Clone)]
pub enum ClientConnectionReport
{
    Connected,
    Disconnected,
    ClosedByServer(Option<ezsockets::CloseFrame>),
    ClosedBySelf,
}

//-------------------------------------------------------------------------------------------------------------------

/// Config controlling how clients respond to connection events
#[derive(Debug)]
pub struct ClientConnectionConfig
{
    /// Try to reconnect if the client is disconnected (`true` by default).
    pub reconnect_on_disconnect: bool,
    /// Try to reconnect if the client is closed by the server (`false` by default).
    pub reconnect_on_server_close: bool,
}

impl Default for ClientConnectionConfig
{
    fn default() -> ClientConnectionConfig
    {
        ClientConnectionConfig{
                reconnect_on_disconnect   : true,
                reconnect_on_server_close : false,
            }
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

/// Make a websocket url: {ws, wss}://[ip:port]/websocket.
pub fn make_websocket_url(with_tls: bool, address: SocketAddr) -> Result<url::Url, ()>
{
    let mut url = url::Url::parse("https://example.net").map_err(|_| ())?;
    let scheme = match with_tls { true => "wss", false => "ws" };
    url.set_scheme(scheme)?;
    url.set_ip_host(address.ip())?;
    url.set_port(Some(address.port()))?;
    url.set_path("/websocket");
    Ok(url)
}

//-------------------------------------------------------------------------------------------------------------------
