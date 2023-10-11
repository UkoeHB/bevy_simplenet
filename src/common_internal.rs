//local shortcuts
use crate::*;

//third-party shortcuts
use serde::{Serialize, Deserialize};

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

pub(crate) const VERSION_MSG_KEY : &'static str = "v";
pub(crate) const TYPE_MSG_KEY    : &'static str = "t";
pub(crate) const AUTH_MSG_KEY    : &'static str = "a";
pub(crate) const CONNECT_MSG_KEY : &'static str = "c";

//-------------------------------------------------------------------------------------------------------------------

/// A client request for synchronizing a server/client channel.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct SyncRequest
{
    pub(crate) request_id: u64,
}

//-------------------------------------------------------------------------------------------------------------------

/// A server response for synchronizing a server/client channel.
///
/// Includes the client's earliest request id that the server is aware of. This number may not be zero if the client has
/// reconnected at least once. We use the earliest request id to identify older requests that have failed due to a
/// reconnect.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct SyncResponse
{
    request: SyncRequest,
    earliest_req: u64,
}

//-------------------------------------------------------------------------------------------------------------------

/// A server meta-message that may be received by a client.
///
/// Includes backend-specific and client-side messages.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum ServerMeta<Channel: ChannelPack>
{
    Val(ServerVal<Channel>),
    Sync(SyncResponse)
}

//-------------------------------------------------------------------------------------------------------------------

/// A client meta-message that may be received by a server.
///
/// Includes backend-specific and server-side messages.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum ClientMeta<Channel: ChannelPack>
{
    Msg(Channel::ClientMsg),
    Request(Channel::ClientRequest, u64),
    Sync(SyncRequest)
}

//-------------------------------------------------------------------------------------------------------------------
