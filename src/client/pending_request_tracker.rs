//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::collections::HashMap;

//-------------------------------------------------------------------------------------------------------------------

/// Tracks pending requests in order to coordinate request status updates.
#[derive(Debug)]
pub(crate) struct PendingRequestTracker
{
    /// counter for requests
    request_counter: u64,
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
    pub(crate) fn set_status_and_remove(&mut self, request_id: u64, status: RequestStatus) -> bool
    {
        let Some(signal) = self.pending_requests.remove(&request_id) else { return false; };
        signal.inner().set(status);

        true
    }

    /// Convert requests with [`MessageStatus::Sent`] or [`MessageStatus::Failed`] to [`RequestStatus::ResponseLost`]
    /// and drain them.
    pub(crate) fn drain_failed_requests(&mut self) -> impl Iterator<Item = RequestSignal> + '_
    {
        self.pending_requests.drain_filter(
                move |_, signal| -> bool
                {
                    if signal.status() == RequestStatus::Sending { return false; }
                    signal.inner().set(RequestStatus::ResponseLost);
                    true
                }
            ).map(|(_, signal)| signal)
    }

    /// Abort and drain all pending requests.
    pub(crate) fn abort_all(&mut self) -> impl Iterator<Item = RequestSignal> + '_
    {
        self.pending_requests.drain_filter(
                move |_, signal| -> bool
                {
                    signal.inner().set(RequestStatus::ResponseLost);
                    true
                }
            ).map(|(_, signal)| signal)
    }
}

impl Default for PendingRequestTracker
{
    fn default() -> Self
    {
        Self{
            request_counter  : 0u64,
            pending_requests : HashMap::default(),
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
