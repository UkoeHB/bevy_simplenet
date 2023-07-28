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
    /// channel into this session's socket
    pub(crate) socket_msg_sender: ezsockets::Sink,
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
        tracing::warn!("SessionHandler: received text from client (not implemented), closing session...");
        Err(self.close().await)
    }

    // Receive binary from client (via session connection).
    async fn on_binary(&mut self, bytes: Vec<u8>) -> Result<(), ezsockets::Error>
    {
        // try to update rate limit tracker
        if !self.rate_limit_tracker.try_count_msg()
        {
            tracing::warn!("SessionHandler: client messages exceeded rate limit, closing session...");
            return Err(self.close().await);
        }

        // try to deserialize message
        if bytes.len() > self.max_msg_size as usize
        {
            tracing::warn!("SessionHandler: received client message that's too large, closing session...");
            return Err(self.close().await);
        }
        let Ok(message) = bincode::deserialize::<ClientMsg>(&bytes[..])
        else
        {
            tracing::warn!("SessionHandler: received client message that failed to deserialize, closing session...");
            return Err(self.close().await);
        };

        // try to forward client message to session owner
        if let Err(err) = self.client_msg_sender.send(SessionSourceMsg::new(self.id, message))
        {
            tracing::error!(?err, "SessionHandler: client msg sender is broken, closing session...");
            return Err(self.close().await);
        }

        Ok(())
    }

    // Responds to calls to the session connected to this handler (i.e. ezsockets::Session::call()).
    // Returns an error, which forces the session to close.
    //todo: return an informative error message to the client (error is not currently forwarded to client)
    async fn on_call(&mut self, _msg: ()) -> Result<(), ezsockets::Error>
    {
        tracing::info!(self.id, "SessionHandler: closed by server");
        Err(self.close().await)
    }
}

impl<ClientMsg> SessionHandler<ClientMsg>
where
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
{
    /// Close the session
    //todo: return an informative error message to the client (error is not currently forwarded to client)
    async fn close(&mut self) -> ezsockets::Error
    {
        tracing::info!(self.id, "SessionHandler: closing...");

        // close socket (if we don't do this the socket will hang open)
        //todo: higher-granularity close reasons (match on close code)
        //todo: this should not need to be async
        self.socket_msg_sender.send(ezsockets::Message::Close(Some(
                ezsockets::CloseFrame
                {
                    code   : ezsockets::CloseCode::Error,
                    reason : String::from("closed by server")
                }
            ))).await;

        // return error to force-close the session
        Box::new(SessionError::ClosedByServer)
    }
}

//-------------------------------------------------------------------------------------------------------------------
