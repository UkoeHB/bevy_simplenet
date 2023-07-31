//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

pub type SessionID = u128;

//-------------------------------------------------------------------------------------------------------------------

pub(crate) const VERSION_MSG_HEADER: &'static str = "WSC-vers";
pub(crate) const AUTH_MSG_HEADER: &'static str    = "WSC-auth";
pub(crate) const CONNECT_MSG_HEADER: &'static str = "WSC-connect";

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by servers when a client connects/disconnects.
#[derive(Debug, Clone)]
pub enum ConnectionReport<ConnectMsg: Debug + Clone>
{
    Connected(SessionID, ConnectMsg),
    Disconnected(SessionID)
}

//-------------------------------------------------------------------------------------------------------------------

/// Server-enforced constraints on client connections.
#[derive(Debug)]
pub struct ConnectionConfig
{
    /// Max number of concurrent client connections.
    pub max_connections: u32,
    /// Max message size allowed from clients (bytes).
    pub max_msg_size: u32,
    /// Rate limit for messages received from a session.
    pub rate_limit_config: RateLimitConfig
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
