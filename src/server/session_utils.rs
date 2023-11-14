//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

/// Message targeted at a session.
#[derive(Debug)]
pub(crate) struct SessionTargetMsg<I, T>
{
    pub(crate) id  : I,
    pub(crate) msg : T
}

impl<I, T> SessionTargetMsg<I, T>
{
    pub(crate) fn new(id: I, msg: T) -> SessionTargetMsg<I, T> { SessionTargetMsg::<I, T> { id, msg } }
}

//-------------------------------------------------------------------------------------------------------------------

/// Message sourced from a session.
#[derive(Debug)]
pub(crate) struct SessionSourceMsg<I, T>
{
    pub(crate) id  : I,
    pub(crate) msg : T
}

impl<I, T> SessionSourceMsg<I, T>
{
    pub(crate) fn new(id: I, msg: T) -> SessionSourceMsg<I, T> { SessionSourceMsg::<I, T> { id, msg } }
}

//-------------------------------------------------------------------------------------------------------------------

/// Command for a session.
#[derive(Debug, Clone)]
pub(crate) enum SessionCommand<Channel: ChannelPack>
{
    /// Send a client meta event.
    ///
    /// Includes an optional 'death signal' for the target session of responses. We need this signal in order to
    /// address a race condition between the server API and the server backend where a response for a request received
    /// by an old session could be sent via a new session.
    Send(ClientMetaEventFrom<Channel>, Option<SessionDeathSignal>),
    /// Close a session.
    Close(ezsockets::CloseFrame)
}

//-------------------------------------------------------------------------------------------------------------------
