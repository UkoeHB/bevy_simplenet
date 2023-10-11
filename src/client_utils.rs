//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;

//standard shortcuts
use core::fmt::Debug;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

//-------------------------------------------------------------------------------------------------------------------

pub type MessageSignal = ezsockets::MessageSignal;
pub type MessageStatus = ezsockets::MessageStatus;

//-------------------------------------------------------------------------------------------------------------------

/// Errors emitted by the internal client handler.
#[derive(Debug)]
pub enum ClientError
{
    //ClosedByServer,
    SendError
}

impl std::fmt::Display for ClientError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let _ = write!(f, "ClientError::");
        match self
        {
            //ClientError::ClosedByServer => write!(f, "ClosedByServer"),
            ClientError::SendError      => write!(f, "SendError"),
        }
    }
}
impl std::error::Error for ClientError {}

//-------------------------------------------------------------------------------------------------------------------

/// Config controlling how clients respond to connection events.
#[derive(Debug)]
pub struct ClientConfig
{
    /// Try to reconnect if the client is disconnected. Defaults to `true`.
    pub reconnect_on_disconnect: bool,
    /// Try to reconnect if the client is closed by the server. Defaults to `false`.
    pub reconnect_on_server_close: bool,
    /// Reconnect interval (delay between reconnect attempts). Defaults to 2 seconds.
    pub reconnect_interval: Duration,
    /// Maximum number of connection attempts when initially connecting. Defaults to infinite.
    pub max_initial_connect_attempts: usize,
    /// Maximum number of reconnect attempts when reconnecting. Defaults to infinite.
    pub max_reconnect_attempts: usize,
    /// Duration between socket heartbeat pings if the connection is inactive. Defaults to 5 seconds.
    pub heartbeat_interval: Duration,
    /// Duration after which a socket will shut down if the connection is inactive. Defaults to 10 seconds
    pub keepalive_timeout: Duration,
}

impl Default for ClientConfig
{
    fn default() -> ClientConfig
    {
        ClientConfig{
                reconnect_on_disconnect      : true,
                reconnect_on_server_close    : false,
                reconnect_interval           : Duration::from_secs(2),
                max_initial_connect_attempts : usize::MAX,
                max_reconnect_attempts       : usize::MAX,
                heartbeat_interval           : Duration::from_secs(5),
                keepalive_timeout            : Duration::from_secs(10),
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by clients when they connect/disconnect/shut down.
#[derive(Debug, Clone)]
pub enum ClientReport
{
    Connected,
    Disconnected,
    ClosedByServer(Option<ezsockets::CloseFrame>),
    ClosedBySelf,
    IsDead,
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct RequestSignalInner
{
    signal: Arc<AtomicU8>,
}

impl RequestSignalInner {
    pub(crate) fn status(&self) -> RequestStatus {
        match self.signal.load(Ordering::Acquire) {
            0u8 => RequestStatus::Waiting,
            1u8 => RequestStatus::Responded,
            2u8 => RequestStatus::Acknowledged,
            3u8 => RequestStatus::Rejected,
            _   => RequestStatus::ResponseLost,
        }
    }

    pub(crate) fn set(&self, state: RequestStatus) {
        match state {
            RequestStatus::Waiting      => self.signal.store(0u8, Ordering::Release),
            RequestStatus::Responded    => self.signal.store(1u8, Ordering::Release),
            RequestStatus::Acknowledged => self.signal.store(2u8, Ordering::Release),
            RequestStatus::Rejected     => self.signal.store(3u8, Ordering::Release),
            RequestStatus::ResponseLost => self.signal.store(4u8, Ordering::Release),
            _ => panic!("invalid request status sent to RequestSignalInner"),
        }
    }
}

impl Default for RequestSignalInner { fn default() -> Self { Self { signal: Arc::new(AtomicU8::new(0u8)) } } }

//-------------------------------------------------------------------------------------------------------------------

/// Tracks the current status of a client request.
#[derive(Clone, Debug)]
pub struct RequestSignal
{
    request_id     : u64,
    message_signal : MessageSignal,
    request_signal : RequestSignalInner,
}

impl RequestSignal
{
    /// Make a new signal.
    pub fn new(request_id: u64, message_signal: MessageSignal) -> Self
    {
        Self{
            request_id,
            message_signal,
            request_signal: RequestSignalInner::default(),
        }
    }

    /// Get the id of the request corresponding to this signal.
    pub fn id(&self) -> u64
    {
        self.request_id
    }

    /// Get the request status.
    pub fn status(&self) -> RequestStatus
    {
        match self.message_signal.status()
        {
            MessageStatus::Sending => RequestStatus::Sending,
            MessageStatus::Sent    => self.request_signal.status(),
            MessageStatus::Failed  => RequestStatus::SendFailed,
        }
    }

    /// Access the inner request signal tracker.
    pub(crate) fn inner(&self) -> &RequestSignalInner
    {
        &self.request_signal
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Tracks pending requests in order to coordinate request status updates.
#[derive(Debug)]
pub(crate) struct PendingRequestTracker
{
    /// counter for requests
    request_counter: u64,
    /// id of last sent sync request
    latest_sync_request: Option<u64>,
    /// pending requests
    pending_requests: HashMap<u64, RequestSignal>,
}

impl PendingRequestTracker
{
    /// Reserve a request id.
    pub(crate) fn reserve_id(&mut self) -> u64
    {
        let id = self.request_counter;
        self.request_counter += 1;
        id
    }

    /// Add a new pending request.
    pub(crate) fn add_request(&mut self, id: u64, message_signal: MessageSignal) -> RequestSignal
    {
        let signal = RequestSignal::new(id, message_signal);
        self.pending_requests.insert(id, signal.clone());
        signal
    }

    /// Set the status of a pending request and remove it from the tracker.
    pub(crate) fn set_status_and_remove(&mut self, request_id: u64, status: RequestStatus)
    {
        let Some(signal) = self.pending_requests.remove(&request_id) else { return; };
        signal.inner().set(status);
    }

    /// Try to send a sync request to the server.
    ///
    /// Note that we assume if a sync request fails then it will coincide with a reconnect cycle that will trigger
    /// another sync request (or cause the client to shut down and ultimately mark pending requests as `ResponseLost`).
    /// This assumption may be broken by upstream bugs.
    pub(crate) fn try_make_sync_request<Channel: ChannelPack>(
        &mut self,
        client: &ezsockets::Client<ClientHandler<Channel>>
    ){
        // if there are no pending requests, there is no need for a sync request
        if self.pending_requests.is_empty() { return; }

        // make sync request
        let request_id = self.reserve_id();
        let request = SyncRequest{ request_id };
        self.latest_sync_request = Some(request_id);

        // forward message to server
        let Ok(ser_msg) = bincode::DefaultOptions::new().serialize(&ClientMetaFrom::<Channel>::Sync(request))
        else { tracing::error!("failed serializing client sync request"); return; };

        if client.binary(ser_msg).is_err() { tracing::warn!("tried to send sync request to dead client"); }
    }

    /// Handle a sync response from the server.
    ///
    /// We mark all pending requests lower than the server's earliest-seen request as [`RequestStatus::ResponseLost`].
    pub(crate) fn handle_sync_response(&mut self, response: SyncResponse)
    {
        // ignore response if not responding to latest request
        if Some(response.request.request_id) != self.latest_sync_request
        { tracing::debug!(?response, ?self.latest_sync_request, "received stale sync response"); return; }

        // remove all entries lower than the sync point
        self.pending_requests.retain(
                |id, signal| -> bool
                {
                    if *id >= response.earliest_req { return true; }
                    signal.inner().set(RequestStatus::ResponseLost);
                    false
                }
            );
    }
}

impl Default for PendingRequestTracker
{
    fn default() -> Self
    {
        Self{
            request_counter     : 0u64,
            latest_sync_request : None,
            pending_requests    : HashMap::default(),
        }
    }
}

impl Drop for PendingRequestTracker
{
    fn drop(&mut self)
    {
        for (_, signal) in self.pending_requests.iter()
        {
            signal.inner().set(RequestStatus::ResponseLost);
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
