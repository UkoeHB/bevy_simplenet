//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by clients when they connect/disconnect/shut down.
#[derive(Debug, Clone)]
pub enum ClientReport
{
    /// The client connected to the server.
    ///
    /// This event synchronizes with the request/response pattern. All requests sent before the client became connected
    /// will receive a result event (Response/Ack/Reject/SendFailed/ResponseLost) before this event is emitted.
    Connected,
    /// The client disconnected from the server.
    Disconnected,
    /// The client was closed by the server.
    ClosedByServer(Option<ezsockets::CloseFrame>),
    /// The client closed itself.
    ClosedBySelf,
    /// The client has died and will not try to reconnect.
    ///
    /// Stores the pending request ids for requests that were [`RequestStatus::Sending`] at the time the client died.
    /// The requests will eventually transition to either [`RequestStatus::SendFailed`] or [`RequestStatus::ResponseLost`].
    ///
    /// No more events will be emitted after this event appears.
    IsDead(Vec<u64>),
}

//-------------------------------------------------------------------------------------------------------------------

/// An event received by a client.
///
/// The `SendFailed` and `ResponseLost` events will only be emitted in these scenarios:
/// - Between [`ClientReport::Disconnected`] and [`ClientReport::Connected`] reports (if the `reconnect_on_disconnect`
///   config is set).
/// - Between [`ClientReport::Disconnected`] and [`ClientReport::IsDead`] reports (if the `reconnect_on_disconnect`
///   config is not set).
/// - Between [`ClientReport::ClosedByServer`] and [`ClientReport::Connected`] reports (if the `reconnect_on_server_close`
///   config is set).
/// - Between [`ClientReport::ClosedByServer`] and [`ClientReport::IsDead`] reports (if the `reconnect_on_server_close`
///   config is not set).
/// - Between [`ClientReport::ClosedBySelf`] and [`ClientReport::IsDead`] reports.
/// - Between an unexpected internal error and a [`ClientReport::IsDead`] report.
/// - Between dropping the client and a [`ClientReport::IsDead`] report. In this case the events will not be readable.
#[derive(Debug, Clone)]
pub enum ClientEvent<ServerMsg, ServerResponse>
{
    /// A connection report.
    Report(ClientReport),
    /// A one-shot server message.
    Msg(ServerMsg),
    /// A response to a client request.
    Response(ServerResponse, u64),
    /// The sever acknowledged receiving a client request.
    ///
    /// This will not be followed by a subsequent response (you either get a response, ack, or rejection).
    Ack(u64),
    /// The server rejected a client request.
    Reject(u64),
    /// Sending a request failed.
    SendFailed(u64),
    /// The server received a request but the client failed to receive a response.
    ResponseLost(u64),
}

//-------------------------------------------------------------------------------------------------------------------

/// Get a [`ClientEvent`] from a [`ChannelPack`].
pub type ClientEventFrom<Channel> = ClientEvent<
    <Channel as ChannelPack>::ServerMsg,
    <Channel as ChannelPack>::ServerResponse
>;

//-------------------------------------------------------------------------------------------------------------------
