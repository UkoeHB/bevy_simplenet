//local shortcuts

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

//-------------------------------------------------------------------------------------------------------------------

/// Re-exports `ezsockets::MessageSignal`.
pub type MessageSignal = ezsockets::MessageSignal;

/// Re-exports `ezsockets::MessageStatus`.
pub type MessageStatus = ezsockets::MessageStatus;

//-------------------------------------------------------------------------------------------------------------------

/// Indicates the current status of a client request.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RequestStatus
{
    /// The request is sending.
    Sending,
    /// The request was sent and now we are waiting for a response.
    ///
    /// If disconnected while in this state, the request status will change to `ResponseLost`.
    Waiting,
    /// The server responded to the request.
    Responded,
    /// The server acknowledged the request and will not respond.
    Acknowledged,
    /// The server rejected the request.
    Rejected,
    /// The request failed to send.
    SendFailed,
    /// The request was sent but the client disconnected from the server before we could receive a response.
    ///
    /// The request may have been responded to, acknowledged, or rejected, but we will never know.
    ///
    /// Note that if you drop the client, any `Waiting` requests will be set to `ResponseLost`.
    ResponseLost,
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

impl Default for RequestSignalInner { fn default() -> Self
{
    Self { signal: Arc::new(AtomicU8::new(0u8)) } }
}

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
            MessageStatus::Sent    => self.inner().status(),
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
