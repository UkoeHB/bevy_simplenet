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
    death_signal : Arc<AtomicBool>,
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
        Self{ client_id, request_id, rejector: Some(rejector), death_signal }
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
    //todo: consider allowing request tokens to persist across reconnects
    pub fn destination_is_dead(&self) -> bool
    {
        self.death_signal.load(Ordering::Relaxed)
    }

    /// Consume the token, preventing it from sending a rejection message when dropped.
    pub(crate) fn take(mut self) -> u64
    {
        let _ = self.rejector.take();
        self.request_id
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
    Send(ServerValFrom<Channel>),
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
