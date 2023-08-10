//local shortcuts
use crate::*;

//third-party shortcuts
use serde::Deserialize;

//standard shortcuts
use std::fmt::Debug;
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SessionHandler<ClientMsg>
where
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
{
    /// id of this session
    pub(crate) id: SessionID,
    /// this session
    pub(crate) session: ezsockets::Session<SessionID, ()>,
    /// sender for forwarding messages from the session's client to the server
    pub(crate) client_msg_sender: crossbeam::channel::Sender<SessionSourceMsg<SessionID, ClientMsg>>,

    /// config: maximum message size (bytes)
    pub(crate) max_msg_size: u32,

    /// rate limit tracker
    pub(crate) rate_limit_tracker: RateLimitTracker
}

#[async_trait::async_trait]
impl<ClientMsg> ezsockets::SessionExt for SessionHandler<ClientMsg>
where
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
{
    type ID   = SessionID;
    type Call = ();

    fn id(&self) -> &SessionID
    {
        &self.id
    }

    // Receive text from client (via session connection).
    async fn on_text(&mut self, _text: String) -> Result<(), ezsockets::Error>
    {
        // reject text from client
        tracing::trace!("received text from client (not implemented), closing session...");
        self.close("text not allowed").await; return Ok(());
    }

    // Receive binary from client (via session connection).
    async fn on_binary(&mut self, bytes: Vec<u8>) -> Result<(), ezsockets::Error>
    {
        // try to update rate limit tracker
        if !self.rate_limit_tracker.try_count_msg()
        {
            tracing::trace!("client messages exceeded rate limit, closing session...");
            self.close("rate limit violation").await; return Ok(());
        }

        // try to deserialize message
        if bytes.len() > self.max_msg_size as usize
        {
            tracing::trace!("received client message that's too large, closing session...");
            self.close("message size violation").await; return Ok(());
        }
        let Ok(message) = bincode::deserialize::<ClientMsg>(&bytes[..])
        else
        {
            tracing::trace!("received client message that failed to deserialize, closing session...");
            self.close("deserialization failure").await; return Ok(());
        };

        // try to forward client message to session owner
        if let Err(err) = self.client_msg_sender.send(SessionSourceMsg::new(self.id, message))
        {
            tracing::error!(?err, "client msg sender is broken, closing session...");
            self.close("session error").await; return Ok(());
        }

        Ok(())
    }

    // Responds to calls to the session connected to this handler (i.e. ezsockets::Session::call()).
    async fn on_call(&mut self, _msg: ()) -> Result<(), ezsockets::Error>
    {
        tracing::info!(self.id, "received call (not implemented), closing session...");
        self.close("session error").await; return Ok(());
    }
}

impl<ClientMsg> SessionHandler<ClientMsg>
where
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
{
    /// Close the session
    async fn close(&mut self, reason: &str)
    {
        tracing::info!(self.id, "closing...");
        if let Err(_) = self.session.close(Some(
                ezsockets::CloseFrame
                {
                    code   : ezsockets::CloseCode::Error,
                    reason : String::from(reason)
                }
            )).await
        {
            tracing::error!(self.id, "failed closing session");
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
