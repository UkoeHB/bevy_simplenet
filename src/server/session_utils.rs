//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

/// Id for sessions inside the server.
pub(crate) type SessionId = u64;

//-------------------------------------------------------------------------------------------------------------------

/// Message targeted at a session.
#[derive(Debug)]
pub(crate) struct ClientTargetMsg<I, T>
{
    pub(crate) id  : I,
    pub(crate) msg : T
}

impl<I, T> ClientTargetMsg<I, T>
{
    pub(crate) fn new(id: I, msg: T) -> ClientTargetMsg<I, T> { ClientTargetMsg::<I, T> { id, msg } }
}

//-------------------------------------------------------------------------------------------------------------------

/// Message sourced from a session.
#[derive(Debug)]
pub(crate) struct ClientSourceMsg<I, T>
{
    pub(crate) id  : I,
    pub(crate) msg : T
}

impl<I, T> ClientSourceMsg<I, T>
{
    pub(crate) fn new(id: I, msg: T) -> ClientSourceMsg<I, T> { ClientSourceMsg::<I, T> { id, msg } }
}

//-------------------------------------------------------------------------------------------------------------------

/// Command for a session.
#[derive(Debug, Clone)]
pub(crate) enum SessionCommand<Channel: ChannelPack>
{
    /// Adds a newly authenticated session to the internal session registry.
    Add {
        session_id: SessionId,
        msg: Channel::ConnectMsg,
        env_type: EnvType,
    },
    /// Send a client meta event.
    ///
    /// Includes an optional 'connection events consumed counter' for use in synchronizing message sends with
    /// connection events.
    ///
    /// Includes an optional 'death signal' for the target session of responses. We need this signal in order to
    /// address a race condition between the server API and the server backend where a response for a request received
    /// by an old session could be sent via a new session.
    Send(ClientMetaEventFrom<Channel>, Option<u64>, Option<SessionDeathSignal>),
    /// Close a session.
    Close(Option<ezsockets::CloseFrame>)
}

//-------------------------------------------------------------------------------------------------------------------
