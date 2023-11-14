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

/// A meta event that may be received by a client.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum ClientMetaEvent<ServerMsg, ServerResponse>
{
    Msg(ServerMsg),
    Response(ServerResponse, u64),
    Ack(u64),
    Reject(u64),
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) type ClientMetaEventFrom<Channel> = ClientMetaEvent<
    <Channel as ChannelPack>::ServerMsg,
    <Channel as ChannelPack>::ServerResponse
>;

//-------------------------------------------------------------------------------------------------------------------

/// A meta event that may be received by a server.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum ServerMetaEvent<ClientMsg, ClientRequest>
{
    Msg(ClientMsg),
    Request(ClientRequest, u64),
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) type ServerMetaEventFrom<Channel> = ServerMetaEvent<
    <Channel as ChannelPack>::ClientMsg,
    <Channel as ChannelPack>::ClientRequest
>;

//-------------------------------------------------------------------------------------------------------------------
