//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;
use serde::Deserialize;

//standard shortcuts
use core::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ClientHandler<Channel: ChannelPack>
{
    /// config
    pub(crate) config: ClientConfig,
    /// core websockets client
    pub(crate) client: ezsockets::Client<ClientHandler<Channel>>,
    /// collects connection events
    pub(crate) connection_report_sender: crossbeam::channel::Sender<ClientReport>,
    /// collects messages from the server
    pub(crate) server_val_sender: crossbeam::channel::Sender<ServerVal<Channel>>,
    /// synchronized tracker for pending requests
    pub(crate) pending_requests: Arc<Mutex<PendingRequestTracker>>,
    /// tracks the most recently acknowledged sync point (the lowest request id that the server is currently aware of)
    pub(crate) last_acked_sync_point: u64,
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
                        tracing::info!(?value, "server ping");
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
        match server_msg
        {
            ServerMeta::<Channel>::Val(msg) =>
            {
                // handle pending request meta
                if let Some((request_id, request_status)) = msg.into_request_status()
                {
                    // clean up pending request
                    self.pending_requests.lock().set_status_and_remove(request_id, request_status);

                    // discard message if request id is below the latest acknowledged sync point
                    if request_id < self.last_acked_sync_point
                    {
                        // this should never happen, but there is technically at least one race condition in the server
                        // that **could** lead to this branch if the stars align
                        tracing::warn!("ignoring server response that somehow got past a sync point");
                        return Ok(());
                    }
                }

                // forward to client owner
                if let Err(err) = self.server_val_sender.send(msg)
                {
                    tracing::error!(?err, "failed to forward server msg to client");
                    return Err(Box::new(ClientError::SendError));
                }
            }
            ServerMeta::<Channel>::Sync(response) =>
            {
                // clean up pending requests
                self.pending_requests.lock().handle_sync_response(response);
                self.last_acked_sync_point = std::cmp::max(response.earliest_req, self.last_acked_sync_point);
            }
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
        // make sync request if there are pending requests
        self.pending_requests.lock().try_make_sync_request(&self.client);

        // forward connection event to client owner
        if let Err(err) = self.connection_report_sender.send(ClientReport::Connected)
        {
            tracing::error!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));
        }
        Ok(())
    }

    //todo: on_connect_fail() (need ezsockets update)

    /// Respond to the client being disconnected.
    async fn on_disconnect(&mut self) -> Result<ezsockets::client::ClientCloseMode, ezsockets::Error>
    {
        tracing::info!("disconnected");

        // forward event to client owner
        if let Err(err) = self.connection_report_sender.send(ClientReport::Disconnected)
        {
            tracing::error!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));
        }

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

        // forward event to client owner
        if let Err(err) = self.connection_report_sender.send(ClientReport::ClosedByServer(close_frame))
        {
            tracing::error!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));
        }

        // choose response
        match self.config.reconnect_on_server_close
        {
            true  => return Ok(ezsockets::client::ClientCloseMode::Reconnect),
            false => return Ok(ezsockets::client::ClientCloseMode::Close),
        }
    }
}

impl<Channel: ChannelPack> Drop for ClientHandler<Channel>
{
    fn drop(&mut self)
    {
        // forward event to client owner
        if let Err(err) = self.connection_report_sender.send(ClientReport::IsDead)
        {
            // failing may not be an error since the owning client could have been dropped
            tracing::debug!(?err, "failed to forward 'client is dead' report to client");
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
