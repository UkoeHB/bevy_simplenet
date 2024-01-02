//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by servers when a client connects/disconnects.
#[derive(Debug, Clone)]
pub enum ServerReport<ConnectMsg: Debug + Clone>
{
    /// The client connected.
    ///
    /// This event synchronizes with the corresponding [`ClientReport::Connected`] event in the client. This means you
    /// can send a 'server state sync' message to the client in response to this event, and the client will obtain that
    /// message immediately after its [`ClientReport::Connected`] event.
    /// 
    /// See the [`ServerEvent::Request`] docs for one qualification on state syncing.
    Connected(EnvType, ConnectMsg),
    /// The client disconnected.
    Disconnected,
}

//-------------------------------------------------------------------------------------------------------------------

/// An event received by the server.
#[derive(Debug)]
pub enum ServerEvent<ConnectMsg: Debug + Clone, ClientMsg: Debug, ClientRequest: Debug>
{
    /// A report about a client connection.
    Report(ServerReport<ConnectMsg>),
    /// A one-shot client message.
    Msg(ClientMsg),
    /// A request to the server.
    ///
    /// The server should reply with a response, ack, or rejection.
    ///
    /// It is not recommended to use a request/response pattern for requests that are not immediately handled on the
    /// server (i.e. not handled before pulling any new client values from the server API).
    ///
    /// Consider this sequence of server API accesses:
    /// 1) Server receives request from client A. The request is not immediately handled.
    /// 2) Server receives disconnect event for client A.
    /// 3) Server receives connect event for client A.
    /// 4) Server sends server-state sync message to client A to ensure the client's view of the server is correct.
    /// 5) Server mutates itself while handling the request from step 1. Note that this step may be asynchronously
    ///    distributed across the previous steps, which means we cannot simply check if the request token is dead before
    ///    making any mutations (i.e. in order to synchronize with step 4).
    /// 6) Server sends response to request from step 1, representing the updated server state.
    ///
    /// The client will *not* receive the response in step 5, because we discard all responses sent for pre-reconnect
    /// requests. Doing so allows us to promptly notify clients that requests have failed when they disconnect,
    /// which is important for client responsiveness. As a result, any mutations from
    /// step 5 will not be visible to the client. If you use a client-message/server-message pattern then the server
    /// message will not be discarded, at the cost of weaker message tracking in the client.
    ///
    /// **Caveat**: It is viable to combine a request/response pattern with a server-message fallback. In step 6 if
    ///             the request token is dead, then you can send the response as a server message. If it is not dead and the
    ///             response fails anyway (due to another disconnect), then we know that the client wouldn't have received
    ///             the fallback message either. In that case, when the client reconnects (for the second time) they
    ///             will receive a server-state sync message that will include the updated state from the prior request
    ///             (which at that point would have been sent two full reconnect cycles ago).
    Request(RequestToken, ClientRequest),
}

//-------------------------------------------------------------------------------------------------------------------

/// Get a [`ServerEvent`] from a [`ChannelPack`].
pub type ServerEventFrom<Channel> = ServerEvent<
    <Channel as ChannelPack>::ConnectMsg,
    <Channel as ChannelPack>::ClientMsg,
    <Channel as ChannelPack>::ClientRequest
>;

//-------------------------------------------------------------------------------------------------------------------
