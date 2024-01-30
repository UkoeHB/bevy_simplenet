//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;

//standard shortcuts
use core::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ClientHandler<Channel: ChannelPack>
{
    /// config
    pub(crate) config: ClientConfig,
    /// core websockets client
    pub(crate) client: ezsockets::Client<ClientHandler<Channel>>,
    /// send client events to the client
    pub(crate) client_event_sender: crossbeam::channel::Sender<ClientEventFrom<Channel>>,
    /// synchronized tracker for pending requests
    pub(crate) pending_requests: Arc<Mutex<PendingRequestTracker>>,
    /// signal to communicate how many disconnects have occurred; synchronizes with connection events
    pub(crate) client_disconnected_count: Arc<AtomicU16>,
    /// signal to communicate when the client handler is dead; synchronizes with draining the pending request cache
    pub(crate) client_closed_signal: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl<Channel: ChannelPack> ezsockets::ClientExt for ClientHandler<Channel>
{
    type Call = ();

    /// Text from server.
    /// - Does nothing on native.
    /// - Echoes the text back to the server on WASM for custom Ping/Pong protocol.
    async fn on_text(&mut self, text: String) -> Result<(), ezsockets::Error>
    {
        match env_type()
        {
            EnvType::Native =>
            {
                // ignore text received
                tracing::warn!("received text from server (not handled)");
            }
            EnvType::Wasm =>
            {
                // received Ping or Pong
                let Some((var, value)) = text.as_str().split_once(':')
                else { tracing::warn!("ignoring invalid text from server..."); return Ok(()); };

                // try to deserialize timestamp
                let Ok(timestamp) = u128::from_str_radix(value, 10u32)
                else { tracing::warn!("ignoring invalid ping/pong from server..."); return Ok(()); };

                match var
                {
                    "ping" =>
                    {
                        // received Ping, send Pong back
                        let _ = self.client.text(format!("pong:{}", value))?;
                    }
                    "pong" =>
                    {
                        // received Pong, log latency
                        log_ping_pong_latency(timestamp);
                    }
                    _ => tracing::warn!("ignoring invalid ping/pong from server...")
                }
            }
        }

        Ok(())
    }

    /// Binary from server.
    async fn on_binary(&mut self, bytes: Vec<u8>) -> Result<(), ezsockets::Error>
    {
        tracing::trace!("received binary from server");

        // deserialize message
        let Ok(server_msg) = bincode::DefaultOptions::new().deserialize(&bytes[..])
        else
        {
            tracing::warn!("received server msg that failed to deserialize");
            return Ok(());
        };

        // decide how to handle the message
        let client_event = match server_msg
        {
            ClientMetaEventFrom::<Channel>::Msg(msg) =>
            {
                // msg
                ClientEventFrom::<Channel>::Msg(msg)
            }
            ClientMetaEventFrom::<Channel>::Response(response, request_id) =>
            {
                // discard message if request id is unknown
                // - this should never happen
                let Ok(mut pending_requests) = self.pending_requests.lock() else { return Ok(()); };
                if !pending_requests.set_status_and_remove(request_id, RequestStatus::Responded)
                {
                    tracing::error!(request_id, "ignoring server response for unknown request");
                    return Ok(());
                }

                // response
                ClientEventFrom::<Channel>::Response(response, request_id)
            }
            ClientMetaEventFrom::<Channel>::Ack(request_id) =>
            {
                // discard message if request id is unknown
                // - this should never happen
                let Ok(mut pending_requests) = self.pending_requests.lock() else { return Ok(()); };
                if !pending_requests.set_status_and_remove(request_id, RequestStatus::Acknowledged)
                {
                    tracing::error!(request_id, "ignoring server ack for unknown request");
                    return Ok(());
                }

                // ack
                ClientEventFrom::<Channel>::Ack(request_id)
            }
            ClientMetaEventFrom::<Channel>::Reject(request_id) =>
            {
                // discard message if request id is unknown
                // - this should never happen
                let Ok(mut pending_requests) = self.pending_requests.lock() else { return Ok(()); };
                if !pending_requests.set_status_and_remove(request_id, RequestStatus::Rejected)
                {
                    tracing::error!(request_id, "ignoring server rejection for unknown request");
                    return Ok(());
                }

                // rejection
                ClientEventFrom::<Channel>::Reject(request_id)
            }
        };

        // forward to client owner
        if let Err(err) = self.client_event_sender.send(client_event)
        {
            tracing::debug!(?err, "failed to forward server message to client");
            return Err(Box::new(ClientError::SendError));
        }

        Ok(())
    }

    /// Call from associated client.
    ///
    /// Does nothing.
    async fn on_call(&mut self, _msg: ()) -> Result<(), ezsockets::Error>
    {
        // ignore call
        tracing::error!("on_call() invocation (not handled)");
        Ok(())
    }

    /// Respond to the client acquiring a connection.
    async fn on_connect(&mut self) -> Result<(), ezsockets::Error>
    {
        tracing::info!("connected");

        // lock the pending requests cache
        let Ok(mut pending_requests) = self.pending_requests.lock() else { return Ok(()); };

        // clean up existing requests
        // - do this before sending connection event so the event stream is synchronized
        let aborted_sends = Self::final_request_cleanup(&mut pending_requests, &self.client_event_sender);

        // convert aborted sends to `SendFailed`
        // - `aborted_sends` may not be empty if the socket for the previous connection did not fully shut down yet.
        //   It is possible for messages to linger in the socket's internal sink even after the socket has
        //   been dropped. Those messages are guaranteed to fail because on_connect() synchronizes with the
        //   server's old session completely shutting down, so we can treat them as such here.
        for aborted_send in aborted_sends
        {
            if let Err(err) = self.client_event_sender.send(ClientEventFrom::<Channel>::SendFailed(aborted_send))
            {
                tracing::debug!(?err, "failed to forward client event to client");
                return Err(Box::new(ClientError::SendError));
            }
        }

        // forward connection event to client owner
        if let Err(err) = self.client_event_sender.send(ClientEventFrom::<Channel>::Report(ClientReport::Connected))
        {
            tracing::debug!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));
        }

        Ok(())
    }

    /// Respond to the client failing a connection attempt.
    async fn on_connect_fail(
        &mut self,
        _error: ezsockets::WSError
    ) -> Result<ezsockets::client::ClientCloseMode, ezsockets::Error>
    {
        // lock the pending requests cache
        let Ok(mut pending_requests) = self.pending_requests.lock()
        else { return Ok(ezsockets::client::ClientCloseMode::Close); };

        // note: We do NOT increment the disconnected counter here, since 'connect fail' just means we have remained
        //       disconnected.

        // clean up pending requests
        Self::clean_pending_requests(&mut pending_requests, &self.client_event_sender);

        //todo: don't try to reconnect if auth token expired
        Ok(ezsockets::client::ClientCloseMode::Reconnect)
    }

    /// Respond to the client being disconnected.
    async fn on_disconnect(&mut self) -> Result<ezsockets::client::ClientCloseMode, ezsockets::Error>
    {
        tracing::info!("disconnected");

        // lock the pending requests cache
        let Ok(mut pending_requests) = self.pending_requests.lock()
        else { return Ok(ezsockets::client::ClientCloseMode::Close); };

        // mark the client as disconnected
        // - We do this within the pending requests lock in order to synchronize with the client API.
        self.client_disconnected_count.fetch_add(1u16, Ordering::Release);

        // forward event to client owner
        if let Err(err) = self.client_event_sender.send(ClientEventFrom::<Channel>::Report(ClientReport::Disconnected))
        {
            tracing::debug!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));
        }

        // clean up pending requests
        // - do this after sending the client report so request failures appear between client disconnected and client
        //   connected reports (except when the client is dying)
        Self::clean_pending_requests(&mut pending_requests, &self.client_event_sender);

        // choose response
        match self.config.reconnect_on_disconnect
        {
            true  => return Ok(ezsockets::client::ClientCloseMode::Reconnect),
            false => return Ok(ezsockets::client::ClientCloseMode::Close),
        }
    }

    /// Respond to the client being closed by the server.
    async fn on_close(
        &mut self,
        close_frame: Option<ezsockets::CloseFrame>
    ) -> Result<ezsockets::client::ClientCloseMode, ezsockets::Error>
    {
        tracing::info!(?close_frame, "closed by server");

        // lock the pending requests cache
        let Ok(mut pending_requests) = self.pending_requests.lock()
        else { return Ok(ezsockets::client::ClientCloseMode::Close); };

        // mark the client as disconnected
        // - We do this within the pending requests lock in order to synchronize with the client API.
        self.client_disconnected_count.fetch_add(1u16, Ordering::Release);

        // forward event to client owner
        if let Err(err) = self.client_event_sender.send(
                ClientEventFrom::<Channel>::Report(ClientReport::ClosedByServer(close_frame))
            )
        {
            tracing::error!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));
        }

        // clean up pending requests
        // - do this after sending the client report so request failures appear between client disconnected and client
        //   connected reports (except when the client is dying)
        Self::clean_pending_requests(&mut pending_requests, &self.client_event_sender);

        // choose response
        match self.config.reconnect_on_server_close
        {
            true  => return Ok(ezsockets::client::ClientCloseMode::Reconnect),
            false => return Ok(ezsockets::client::ClientCloseMode::Close),
        }
    }
}

impl<Channel: ChannelPack> ClientHandler<Channel>
{
    fn clean_pending_requests(
        pending_requests    : &mut PendingRequestTracker,
        client_event_sender : &crossbeam::channel::Sender<ClientEventFrom<Channel>>
    ){
        for failed_req in pending_requests.drain_failed_requests()
        {
            match failed_req.status()
            {
                RequestStatus::SendFailed =>
                {
                    if let Err(err) = client_event_sender.send(ClientEventFrom::<Channel>::SendFailed(failed_req.id()))
                    {
                        tracing::debug!(?err, "failed to forward client report to client");
                    }
                }
                RequestStatus::ResponseLost =>
                {
                    if let Err(err) = client_event_sender.send(ClientEventFrom::<Channel>::ResponseLost(failed_req.id()))
                    {
                        tracing::debug!(?err, "failed to forward client report to client");
                    }
                }
                status =>
                {
                    tracing::error!(?status, "unexpected request status while draining failed requests");
                }
            }
        }
    }

    fn final_request_cleanup(
        pending_requests    : &mut PendingRequestTracker,
        client_event_sender : &crossbeam::channel::Sender<ClientEventFrom<Channel>>
    ) -> Vec<u64>
    {
        let mut aborted_reqs = Vec::new();

        for failed_req in pending_requests.abort_all()
        {
            match failed_req.status()
            {
                RequestStatus::SendFailed =>
                {
                    if let Err(err) = client_event_sender.send(ClientEventFrom::<Channel>::SendFailed(failed_req.id()))
                    {
                        tracing::debug!(?err, "failed to forward client report to client");
                    }
                }
                RequestStatus::ResponseLost =>
                {
                    if let Err(err) = client_event_sender.send(ClientEventFrom::<Channel>::ResponseLost(failed_req.id()))
                    {
                        tracing::debug!(?err, "failed to forward client report to client");
                    }
                }
                RequestStatus::Sending =>
                {
                    aborted_reqs.push(failed_req.id());
                }
                status =>
                {
                    tracing::error!(?status, "unexpected request status while aborting requests");
                    aborted_reqs.push(failed_req.id());
                }
            }
        }

        aborted_reqs
    }
}

impl<Channel: ChannelPack> Drop for ClientHandler<Channel>
{
    fn drop(&mut self)
    {
        tracing::info!("dropping client");

        // lock the pending requests cache
        let Ok(mut pending_requests) = self.pending_requests.lock() else { return; };

        // abort all pending requests
        // - do this before the client report so IsDead is the last event emitted
        let aborted_reqs = Self::final_request_cleanup(&mut pending_requests, &self.client_event_sender);

        // forward event to client owner
        if let Err(err) = self.client_event_sender.send(
                ClientEventFrom::<Channel>::Report(ClientReport::IsDead(aborted_reqs))
            )
        {
            // failing may not be an error since the owning client could have been dropped
            tracing::debug!(?err, "failed to forward 'client is dead' report to client");
        }

        // mark the client as dead
        // - We do this within the pending requests lock but after cleaning pending requests in order to synchronize
        //   with the client API. We want to prevent the client from sending requests after this lock zone, and we also
        //   want `Client::is_dead()` to only be true after the pending requests cache has been drained so that subsequent
        //   calls to `Client::next()` will reliably drain the client.
        self.client_disconnected_count.fetch_add(1u16, Ordering::Release);
        self.client_closed_signal.store(true, Ordering::Release);
    }
}

//-------------------------------------------------------------------------------------------------------------------
