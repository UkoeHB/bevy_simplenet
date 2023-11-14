//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

/// Wrapper trait for `Fn(u64)`.
pub(crate) trait RequestRejectorFn: Fn(u64) + Send + Sync + 'static {}
impl<F> RequestRejectorFn for F where F: Fn(u64) + Send + Sync + 'static {}
pub(crate) type RequestRejectorFnT = dyn RequestRejectorFn<Output = ()>;

impl std::fmt::Debug for RequestRejectorFnT
{
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
}

//-------------------------------------------------------------------------------------------------------------------

/// Represents a client request on the server.
///
/// When dropped without using [`Server::respond()`] or [`Server::ack()`], a [`ClientEvent::Reject`] message will be
/// sent to the client. If the client is disconnected, then the rejection message will fail and the client will
/// see their request status change to [`RequestStatus::ResponseLost`].
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
        Self{
            client_id,
            request_id,
            rejector     : Some(rejector),
            death_signal : Some(SessionDeathSignal::new(death_signal))
        }
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
        (self.request_id, self.death_signal.take().unwrap())
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
