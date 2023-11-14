//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

//-------------------------------------------------------------------------------------------------------------------

/// Errors emitted by the internal connection handler.
#[derive(Debug, Clone)]
pub enum ConnectionError
{
    SystemError,
}

impl std::fmt::Display for ConnectionError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let _ = write!(f, "ConnectionError::");
        match self
        {
            ConnectionError::SystemError => write!(f, "SystemError"),
        }
    }
}
impl std::error::Error for ConnectionError {}

//-------------------------------------------------------------------------------------------------------------------

/// Config for the [`Server`].
#[derive(Debug, Copy, Clone)]
pub struct ServerConfig
{
    /// Max number of concurrent client connections. Defaults to 100K.
    pub max_connections: u32,
    /// Max message size allowed from clients (bytes). Defaults to 1MB.
    pub max_msg_size: u32,
    /// Rate limit for messages received from a session. See [`RateLimitConfig`] for defaults.
    pub rate_limit_config: RateLimitConfig,
    /// Duration between socket heartbeat pings if the connection is inactive. Defaults to 5 seconds.
    pub heartbeat_interval: Duration,
    /// Duration after which a socket will shut down if the connection is inactive. Defaults to 10 seconds.
    pub keepalive_timeout: Duration,
}

impl Default for ServerConfig
{
    fn default() -> ServerConfig
    {
        ServerConfig{
                max_connections    : 100_000u32,
                max_msg_size       : 1_000_000u32,
                rate_limit_config  : RateLimitConfig::default(),
                heartbeat_interval : Duration::from_secs(5),
                keepalive_timeout  : Duration::from_secs(10),
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Configuration for accepting connections to the [`Server`]. Defaults to non-TLS.
pub enum AcceptorConfig
{
    Default,
    #[cfg(feature = "tls-rustls")]
    Rustls(axum_server::tls_rustls::RustlsConfig),
    #[cfg(feature = "tls-openssl")]
    OpenSSL(axum_server::tls_openssl::OpenSSLConfig),
}

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by servers when a client connects/disconnects.
#[derive(Debug, Clone)]
pub enum ServerReport<ConnectMsg: Debug + Clone>
{
    Connected(SessionID, EnvType, ConnectMsg),
    Disconnected(SessionID)
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct SessionDeathSignal
{
    death_signal: Arc<AtomicBool>
}

impl SessionDeathSignal
{
    pub(crate) fn new(death_signal: Arc<AtomicBool>) -> Self { Self{ death_signal } }
    pub(crate) fn is_dead(&self) -> bool { self.death_signal.load(Ordering::Acquire) }
}

//-------------------------------------------------------------------------------------------------------------------

/// Represents a client request on the server.
///
/// When dropped without using [`Server::respond()`] or [`Server::acknowledge()`], a [`ServerVal::Reject`] message will be
/// sent to the client. If the client is disconnected, then the rejection message will fail and the client will
/// see their request status change to [`RequestStatus::ResponseLost`] after they reconnect.
pub struct RequestToken
{
    client_id    : SessionID,
    request_id   : u64,
    rejector     : Option<Arc<dyn RequestRejectorFn>>,
    death_signal : Option<SessionDeathSignal>,
}

impl RequestToken
{
    /// New token.
    pub(crate) fn new(
        client_id    : SessionID,
        request_id   : u64,
        rejector     : Arc<dyn RequestRejectorFn>,
        death_signal : Arc<AtomicBool>
    ) -> Self
    {
        Self{ client_id, request_id, rejector: Some(rejector), death_signal: Some(SessionDeathSignal::new(death_signal)) }
    }

    /// The id of the client that sent this request.
    pub fn client_id(&self) -> SessionID
    {
        self.client_id
    }

    /// The request id defined by the client who sent this request.
    pub fn request_id(&self) -> u64
    {
        self.request_id
    }

    /// Check if the destination session is dead.
    ///
    /// Request tokens are tied to a specific server session. When a client reconnects they get a new session and
    /// old request tokens become invalid.
    pub fn destination_is_dead(&self) -> bool
    {
        self.death_signal.as_ref().unwrap().is_dead()
    }

    /// Consume the token, preventing it from sending a rejection message when dropped.
    pub(crate) fn take(mut self) -> (u64, SessionDeathSignal)
    {
        let _ = self.rejector.take();
        let death_signal = self.death_signal.take().unwrap();
        (self.request_id, death_signal)
    }
}

impl Drop for RequestToken
{
    fn drop(&mut self)
    {
        let Some(rejector) = self.rejector.take() else { return; };
        if self.destination_is_dead() { return; }
        (rejector)(self.request_id);
    }
}

impl std::fmt::Debug for RequestToken
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "RequestToken [{}, {}]", self.client_id, self.request_id)
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// A client value that may be received by a server.
#[derive(Debug)]
pub enum ClientVal<ClientMsg, ClientRequest>
{
    /// A one-shot client message.
    Msg(ClientMsg),
    /// A request to the server.
    ///
    /// The server should reply with a response, ack, or rejection.
    ///
    /// It is NOT recommended to use a request/response pattern for requests that are not immediately handled on the
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
    Request(ClientRequest, RequestToken),
}

//-------------------------------------------------------------------------------------------------------------------

/// Get a [`ClientVal`] from a [`ChannelPack`].
pub type ClientValFrom<Channel> = ClientVal<
    <Channel as ChannelPack>::ClientMsg,
    <Channel as ChannelPack>::ClientRequest
>;

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
    /// Send a server value.
    ///
    /// Includes an optional 'death signal' for the target session of responses. We need this signal in order to
    /// address a race condition between the server API and the server backend where a response for a request received
    /// by an old session can be sent via a new session.
    Send(ServerValFrom<Channel>, Option<SessionDeathSignal>),
    /// Close a session.
    Close(ezsockets::CloseFrame)
}

//-------------------------------------------------------------------------------------------------------------------

/// Wrapper trait for `Fn(u64)`.
pub(crate) trait RequestRejectorFn: Fn(u64) + Send + Sync + 'static {}
impl<F> RequestRejectorFn for F where F: Fn(u64) + Send + Sync + 'static {}
pub(crate) type RequestRejectorFnT = dyn RequestRejectorFn<Output = ()>;

impl std::fmt::Debug for RequestRejectorFnT
{
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
}

//-------------------------------------------------------------------------------------------------------------------
