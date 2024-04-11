//local shortcuts
use crate::*;

//third-party shortcuts
use serde::{Serialize, Deserialize};

//standard shortcuts
use std::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

pub(crate) const VERSION_MSG_KEY : &'static str = "v";
pub(crate) const TYPE_MSG_KEY    : &'static str = "t";

//-------------------------------------------------------------------------------------------------------------------

/// Id for sessions inside the server.
pub(crate) type SessionId = u64;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ClientAuthMsg<ConnectMsg>
{
    pub(crate) auth: AuthRequest,
    pub(crate) msg: ConnectMsg,
}

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
pub(crate) enum ServerMetaEvent<ConnectMsg, ClientMsg, ClientRequest>
{
    Authenticate(ClientAuthMsg<ConnectMsg>),
    Msg(ClientMsg),
    Request(ClientRequest, u64),
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) type ServerMetaEventFrom<Channel> = ServerMetaEvent<
    <Channel as ChannelPack>::ConnectMsg,
    <Channel as ChannelPack>::ClientMsg,
    <Channel as ChannelPack>::ClientRequest
>;

//-------------------------------------------------------------------------------------------------------------------
