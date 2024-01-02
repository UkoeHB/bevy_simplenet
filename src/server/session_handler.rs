//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;

//standard shortcuts
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::fmt::Debug;
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SessionHandler<Channel: ChannelPack>
{
    /// id of this session
    pub(crate) id: SessionId,
    /// this session
    pub(crate) session: ezsockets::Session<SessionId, ()>,
    /// sender for forwarding messages from the session's client to the server
    pub(crate) server_event_sender: crossbeam::channel::Sender<
        SessionSourceMsg<SessionId, ServerEventFrom<Channel>>
    >,

    /// config: maximum message size (bytes)
    pub(crate) max_msg_size: u32,
    /// client's environment type
    pub(crate) client_env_type: EnvType,

    /// rate limit tracker
    pub(crate) rate_limit_tracker: RateLimitTracker,

    /// session wrapper for sending request rejections
    pub(crate) request_rejector: Arc<dyn RequestRejectorFn>,

    /// Signal used to inform request tokens of the session's death, to avoid sending responses to new sessions
    /// for requests made with old sessions.
    pub(crate) death_signal: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl<Channel: ChannelPack> ezsockets::SessionExt for SessionHandler<Channel>
{
    type ID   = SessionId;
    type Call = ();

    fn id(&self) -> &SessionId
    {
        &self.id
    }

    // Receive text from client (via session connection).
    async fn on_text(&mut self, text: String) -> Result<(), ezsockets::Error>
    {
        match self.client_env_type
        {
            EnvType::Native =>
            {
                // reject text from client
                tracing::trace!("received text from native client (not implemented), closing session...");
                self.close("text not allowed"); return Ok(());
            }
            EnvType::Wasm =>
            {
                // received Ping or Pong
                let Some((var, value)) = text.as_str().split_once(':')
                else
                {
                    tracing::trace!("received invalid text from WASM client, closing session...");
                    self.close("only ping/pong text allowed"); return Ok(());
                };

                // try to deserialize timestamp
                let Ok(timestamp) = u128::from_str_radix(value, 10u32)
                else
                {
                    tracing::trace!("received invalid ping/pong timestamp from WASM client, closing session...");
                    self.close("only timestamp ping/pong allowed"); return Ok(());
                };

                match var
                {
                    "ping" =>
                    {
                        // received Ping, send Pong back
                        let _ = self.session.text(format!("pong:{}", value))?;
                    }
                    "pong" =>
                    {
                        // received Pong, log latency
                        log_ping_pong_latency(timestamp);
                    }
                    _ =>
                    {
                        tracing::trace!("received invalid ping/pong timestamp from WASM client, closing session...");
                        self.close("only ping/pong prefixes allowed");
                    }
                }
            }
        }

        Ok(())
    }

    // Receive binary from client (via session connection).
    async fn on_binary(&mut self, bytes: Vec<u8>) -> Result<(), ezsockets::Error>
    {
        // try to update rate limit tracker
        if !self.rate_limit_tracker.try_count_msg()
        {
            tracing::trace!("client messages exceeded rate limit, closing session...");
            self.close("rate limit violation"); return Ok(());
        }

        // try to deserialize message
        if bytes.len() > self.max_msg_size as usize
        {
            tracing::trace!("received client message that's too large, closing session...");
            self.close("message size violation"); return Ok(());
        }
        let Ok(message) = bincode::DefaultOptions::new().deserialize(&bytes[..])
        else
        {
            tracing::trace!("received client message that failed to deserialize, closing session...");
            self.close("deserialization failure"); return Ok(());
        };

        // decide what to do with the message
        match message
        {
            ServerMetaEventFrom::<Channel>::Msg(msg) =>
            {
                // try to forward client message to session owner
                if let Err(err) = self.server_event_sender.send(
                        SessionSourceMsg::new(self.id, ServerEventFrom::<Channel>::Msg(msg))
                    )
                {
                    tracing::debug!(?err, "client msg sender is broken, closing session...");
                    self.close("session error"); return Ok(());
                }
            }
            ServerMetaEventFrom::<Channel>::Request(request, request_id) =>
            {
                // prepare token
                let token = RequestToken::new(
                        self.id,
                        request_id,
                        self.request_rejector.clone(),
                        self.death_signal.clone(),
                    );

                // try to forward client request to session owner
                if let Err(err) = self.server_event_sender.send(
                        SessionSourceMsg::new(self.id, ServerEventFrom::<Channel>::Request(token, request))
                    )
                {
                    tracing::debug!(?err, "client msg sender is broken, closing session...");
                    self.close("session error"); return Ok(());
                }
            }
        }

        Ok(())
    }

    // Responds to calls to the session connected to this handler (i.e. ezsockets::Session::call()).
    async fn on_call(&mut self, _msg: ()) -> Result<(), ezsockets::Error>
    {
        tracing::info!(self.id, "received call (not implemented), closing session...");
        self.close("session error"); return Ok(());
    }
}

impl<Channel: ChannelPack> SessionHandler<Channel>
{
    /// Close the session
    fn close(&mut self, reason: &str)
    {
        tracing::info!(self.id, "closing...");
        if let Err(_) = self.session.close(Some(
                ezsockets::CloseFrame
                {
                    code   : ezsockets::CloseCode::Error,
                    reason : String::from(reason)
                }
            ))
        {
            tracing::error!(self.id, "failed closing session");
        }
    }
}

impl<Channel: ChannelPack> Drop for SessionHandler<Channel>
{
    fn drop(&mut self)
    {
        self.death_signal.store(true, Ordering::Release);
    }
}

//-------------------------------------------------------------------------------------------------------------------
