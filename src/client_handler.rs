//local shortcuts
use crate::*;

//third-party shortcuts
use serde::Deserialize;

//standard shortcuts
use core::fmt::Debug;
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------

pub(crate) struct ClientHandler<ServerMsg>
{
    /// collects messages from the server
    pub(crate) server_msg_sender: crossbeam::channel::Sender<ServerMsg>,
}

#[async_trait::async_trait]
impl<ServerMsg> ezsockets::ClientExt for ClientHandler<ServerMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
{
    type Call = ();

    /// text from server
    /// Does nothing.
    async fn on_text(&mut self, _text: String) -> Result<(), ezsockets::Error>
    {
        // ignore text received
        tracing::warn!("received text from server (not handled)");
        Ok(())
    }

    /// binary from server
    async fn on_binary(&mut self, bytes: Vec<u8>) -> Result<(), ezsockets::Error>
    {
        tracing::trace!("received binary from server");

        // deserialize message
        let Ok(server_msg) = bincode::deserialize::<ServerMsg>(&bytes[..])
        else
        {
            tracing::warn!("received server msg that failed to deserialize");
            return Ok(());  //ignore it
        };

        // forward to client owner
        if let Err(err) = self.server_msg_sender.send(server_msg)
        {
            tracing::error!(?err, "failed to forward server msg to client");
            return Err(Box::new(ClientError::SendError));  //client is broken
        }

        Ok(())
    }

    /// call from associated client
    /// Does nothing.
    async fn on_call(&mut self, _msg: ()) -> Result<(), ezsockets::Error>
    {
        // ignore call
        tracing::error!("on_call() invocation (not handled)");
        Ok(())
    }

    /// respond to the client being closed
    //todo: customize behavior on closure reason
    //todo: send close reason to client
    async fn on_close(
        &mut self,
        _close_frame: Option<ezsockets::CloseFrame>
    ) -> Result<ezsockets::client::ClientCloseMode, ezsockets::Error>
    {
        tracing::info!("closed by ??");
        Ok(ezsockets::client::ClientCloseMode::Close)
        //Err(Box::new(ClientError::ClosedByServer))  //assume closed by server (todo: maybe closed by client)
    }

    /// respond to the client being disconnected
    //todo: customize behavior on config
    async fn on_disconnect(&mut self) -> Result<ezsockets::client::ClientCloseMode, ezsockets::Error>
    {
        tracing::info!("disconnected");
        Ok(ezsockets::client::ClientCloseMode::Close)
    }
}

//-------------------------------------------------------------------------------------------------------------------
