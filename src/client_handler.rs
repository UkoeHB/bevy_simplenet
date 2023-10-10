//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;
use serde::Deserialize;

//standard shortcuts
use core::fmt::Debug;
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------

pub(crate) type ClientHandlerFromPack<Msgs> = ClientHandler<
        <Msgs as MsgPack>::ServerMsg,
        <Msgs as MsgPack>::ServerResponse,
    >;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ClientHandler<ServerMsg, ServerResponse>
where
    ServerMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ServerResponse: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
{
    /// config
    pub(crate) config: ClientConfig,
    /// core websockets client
    pub(crate) client: ezsockets::Client<ClientHandler<ServerMsg, ServerResponse>>,
    /// collects connection events
    pub(crate) connection_report_sender: crossbeam::channel::Sender<ClientReport>,
    /// collects messages from the server
    pub(crate) server_val_sender: crossbeam::channel::Sender<ServerVal<ServerMsg, ServerResponse>>,
}

#[async_trait::async_trait]
impl<ServerMsg, ServerResponse> ezsockets::ClientExt for ClientHandler<ServerMsg, ServerResponse>
where
    ServerMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ServerResponse: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
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
            return Ok(());  //ignore it
        };

        // forward to client owner
        if let Err(err) = self.server_val_sender.send(server_msg)
        {
            tracing::error!(?err, "failed to forward server msg to client");
            return Err(Box::new(ClientError::SendError));  //client is broken
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
        // forward event to client owner
        if let Err(err) = self.connection_report_sender.send(ClientReport::Connected)
        {
            tracing::error!(?err, "failed to forward connection event to client");
            return Err(Box::new(ClientError::SendError));  //client is broken
        }
        Ok(())
    }

    /// Respond to the client being disconnected.
    async fn on_disconnect(&mut self) -> Result<ezsockets::client::ClientCloseMode, ezsockets::Error>
    {
        tracing::info!("disconnected");

        // forward event to client owner
        if let Err(err) = self.connection_report_sender.send(ClientReport::Disconnected)
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

impl<ServerMsg, ServerResponse> Drop for ClientHandler<ServerMsg, ServerResponse>
where
    ServerMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ServerResponse: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
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
