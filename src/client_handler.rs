//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;
use serde::Deserialize;

//standard shortcuts
use core::fmt::Debug;
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ClientHandler<ServerMsg>
{
    /// config
    pub(crate) config: ClientConnectionConfig,
    /// collects connection events
    pub(crate) connection_report_sender: crossbeam::channel::Sender<ClientConnectionReport>,
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
        let Ok(server_msg) = bincode::DefaultOptions::new().deserialize::<ServerMsg>(&bytes[..])
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

    /// respond to the client acquiring a connection
    async fn on_connect(&mut self) -> Result<(), ezsockets::Error>
    {
        // forward event to client owner
        if let Err(err) = self.connection_report_sender.send(ClientConnectionReport::Connected)
        {
            tracing::error!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));  //client is broken
        }
        Ok(())
    }

    /// respond to the client being disconnected
    async fn on_disconnect(&mut self) -> Result<ezsockets::client::ClientCloseMode, ezsockets::Error>
    {
        tracing::info!("disconnected");

        // forward event to client owner
        if let Err(err) = self.connection_report_sender.send(ClientConnectionReport::Disconnected)
        {
            tracing::error!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));  //client is broken
        }

        // choose response
        match self.config.reconnect_on_disconnect
        {
            true  => return Ok(ezsockets::client::ClientCloseMode::Reconnect),
            false => return Ok(ezsockets::client::ClientCloseMode::Close),
        }
    }

    /// respond to the client being closed by the server
    async fn on_close(
        &mut self,
        close_frame: Option<ezsockets::CloseFrame>
    ) -> Result<ezsockets::client::ClientCloseMode, ezsockets::Error>
    {
        tracing::info!(?close_frame, "closed by server");

        // forward event to client owner
        if let Err(err) = self.connection_report_sender.send(ClientConnectionReport::ClosedByServer(close_frame))
        {
            tracing::error!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));  //client is broken
        }

        // choose response
        match self.config.reconnect_on_server_close
        {
            true  => return Ok(ezsockets::client::ClientCloseMode::Reconnect),
            false => return Ok(ezsockets::client::ClientCloseMode::Close),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
