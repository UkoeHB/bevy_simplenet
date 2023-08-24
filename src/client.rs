//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::Resource;
use bincode::Options;
use enfync::AdoptOrDefault;
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Resource)]
pub struct Client<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ClientMsg: Clone + Debug + Send + Sync + Serialize,
    ConnectMsg: Clone + Debug + Send + Sync + Serialize,
{
    /// this client's id
    client_id: u128,
    /// core websockets client
    client: ezsockets::Client<ClientHandler<ServerMsg>>,
    /// sender for connection events (used for 'closed by self')
    connection_report_sender: crossbeam::channel::Sender<ClientReport>,
    /// receiver for connection events
    connection_report_receiver: crossbeam::channel::Receiver<ClientReport>,
    /// receiver for messages sent by the server
    server_msg_receiver: crossbeam::channel::Receiver<ServerMsg>,
    /// signal for when the internal client is shut down
    client_closed_signal: enfync::builtin::IOPendingResult<()>,

    /// phantom
    _phantom: PhantomData<(ClientMsg, ConnectMsg)>,
}

impl<ServerMsg, ClientMsg, ConnectMsg> Client<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
    ClientMsg: Clone + Debug + Send + Sync + Serialize + 'static,
    ConnectMsg: Clone + Debug + Send + Sync + Serialize + 'static,
{
    /// Associated factory type.
    pub type Factory = ClientFactory<ServerMsg, ClientMsg, ConnectMsg>;

    /// Send message to server.
    pub fn send(&self, msg: &ClientMsg) -> Result<(), ()>
    {
        if self.is_dead()
        {
            tracing::warn!("tried to send message to dead client");
            return Err(());
        }

        // forward message to server
        let Ok(ser_msg) = bincode::DefaultOptions::new().serialize(msg) else { return Err(()); };
        if let Err(_) = self.client.binary(ser_msg)
        {
            tracing::warn!("tried to send message to dead client");
            return Err(());
        }
        Ok(())
    }

    /// Try to get next message received from server.
    pub fn next_msg(&self) -> Option<ServerMsg>
    {
        let Ok(msg) = self.server_msg_receiver.try_recv() else { return None; };
        Some(msg)
    }

    /// Try to get next client connection report.
    pub fn next_report(&self) -> Option<ClientReport>
    {
        let Ok(msg) = self.connection_report_receiver.try_recv() else { return None; };
        Some(msg)
    }

    /// Access this client's id.
    pub fn id(&self) -> u128
    {
        self.client_id
    }

    /// Test if client is dead (no longer connected to server and won't reconnect).
    pub fn is_dead(&self) -> bool
    {
        self.client_closed_signal.is_done()
    }

    /// Close the client.
    pub fn close(&self)
    {
        // sanity check
        if self.is_dead() { tracing::warn!("tried to close an already dead client"); return; }
        tracing::info!("client closing self");

        // close the client
        let closure_frame =
            ezsockets::CloseFrame{
                code   : ezsockets::CloseCode::Normal,
                reason : String::from("client done")
            };
        if self.client.close(Some(closure_frame)).is_err()
        {
            tracing::warn!("tried to close an already dead client");
            return;
        }

        // forward event to other end of channel
        if let Err(err) = self.connection_report_sender.send(ClientReport::ClosedBySelf)
        {
            tracing::error!(?err, "failed to forward connection event to client");
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Factory for producing servers that all bake in the same protocol version.
//todo: use const generics on the protocol version instead (currently broken, async methods cause compiler errors)
#[derive(Debug, Clone)]
pub struct ClientFactory<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ClientMsg: Clone + Debug + Send + Sync + Serialize,
    ConnectMsg: Clone + Debug + Send + Sync + Serialize,
{
    protocol_version : &'static str,
    _phantom         : PhantomData<(ServerMsg, ClientMsg, ConnectMsg)>,
}

impl<ServerMsg, ClientMsg, ConnectMsg> ClientFactory<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
    ClientMsg: Clone + Debug + Send + Sync + Serialize + 'static,
    ConnectMsg: Clone + Debug + Send + Sync + Serialize + 'static,
{
    /// Make a new server factory with a given protocol version.
    pub fn new(protocol_version: &'static str) -> Self
    {
        ClientFactory{ protocol_version, _phantom: PhantomData::default() }
    }

    /// New client (result is available once client is connected).
    pub fn new_client(&self,
        runtime_handle : enfync::builtin::IOHandle,
        url            : url::Url,
        auth           : AuthRequest,
        config         : ClientConfig,
        connect_msg    : ConnectMsg,
    ) -> enfync::builtin::IOPendingResult<Client<ServerMsg, ClientMsg, ConnectMsg>>
    {
        tracing::info!("new client pending");
        let factory_clone = self.clone();
        enfync::builtin::IOPendingResult::new(
                &runtime_handle.into(),
                async move {
                    factory_clone.new_client_async(
                            url,
                            auth,
                            config,
                            connect_msg,
                        ).await
                }
            )
    }

    /// New client (async).
    pub async fn new_client_async(&self,
        url           : url::Url,
        auth          : AuthRequest,
        config        : ClientConfig,
        connect_msg   : ConnectMsg,
    ) -> Client<ServerMsg, ClientMsg, ConnectMsg>
    {
        #[cfg(wasm)]
        { panic!("bevy simplenet clients not supported (yet) on WASM!"); }

        // prepare to make client connection
        // note: http headers cannot contain raw bytes so we must serialize as json
        let auth_msg_ser    = serde_json::to_string(&auth).expect("could not serialize authentication");
        let connect_msg_ser = serde_json::to_string(&connect_msg).expect("could not serialize connect msg");

        let client_config = ezsockets::ClientConfig::new(url)
            .query_parameter(VERSION_MSG_KEY, self.protocol_version)
            .query_parameter(TYPE_MSG_KEY, env_type_as_str(env_type()))
            .query_parameter(AUTH_MSG_KEY, auth_msg_ser.as_str())
            .query_parameter(CONNECT_MSG_KEY, connect_msg_ser.as_str());

        // prepare message channels that point out of our client
        let (
                connection_report_sender,
                connection_report_receiver
            ) = crossbeam::channel::unbounded::<ClientReport>();
        let (server_msg_sender, server_msg_receiver) = crossbeam::channel::unbounded::<ServerMsg>();

        // make client core with our handler
        let connection_report_sender_clone = connection_report_sender.clone();
        let (client, client_handler_worker) = ezsockets::connect(
                move |_client|
                {
                    ClientHandler::<ServerMsg>{
                            config,
                            connection_report_sender: connection_report_sender_clone,
                            server_msg_sender
                        }
                },
                client_config
            ).await;

        // track client closure
        let client_closed_signal = enfync::builtin::IOPendingResult::new(
                &enfync::builtin::IOHandle::adopt_or_default().into(),
                async move {
                    if let Err(err) = client_handler_worker.await
                    {
                        tracing::error!(err, "client closed with error");
                    }
                }
            );

        // finish assembling our client
        Client{
                client_id: auth.client_id(),
                client,
                connection_report_sender,
                connection_report_receiver,
                server_msg_receiver,
                client_closed_signal,
                _phantom: PhantomData::default(),
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------
