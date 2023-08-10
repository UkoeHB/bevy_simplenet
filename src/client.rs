//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::Resource;
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource)]
pub struct Client<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ClientMsg: Clone + Debug + Send + Sync + Serialize,
    ConnectMsg: Clone + Debug + Send + Sync + Serialize,
{
    /// core websockets client
    client: ezsockets::Client<ClientHandler<ServerMsg>>,
    /// receiver for messages sent by the server
    server_msg_receiver: crossbeam::channel::Receiver<ServerMsg>,
    /// signal for when the internal client is shut down
    client_closed_signal: TokioPendingResult<()>,

    /// cached runtime to ensure client remains operational (optional)
    _runtime: Option<Arc<tokio::runtime::Runtime>>,

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
    pub fn send_msg(&self, msg: &ClientMsg) -> Result<(), ()>
    {
        if self.is_dead()
        {
            tracing::warn!("tried to send message to dead client");
            return Err(());
        }

        // forward message to server
        let Ok(ser_msg) = bincode::serialize(msg) else { return Err(()); };
        if let Err(_) = self.client.binary(ser_msg)
        {
            tracing::warn!("tried to send message to dead client");
            return Err(());
        }
        Ok(())
    }

    /// Try to get next message received from server.
    pub fn try_get_next_msg(&self) -> Option<ServerMsg>
    {
        let Ok(msg) = self.server_msg_receiver.try_recv() else { return None; };
        Some(msg)
    }

    /// Test if client is dead (no longer connected to server and won't reconnect).
    pub fn is_dead(&self) -> bool
    {
        self.client_closed_signal.is_done()
    }

    /// Close the client.
    pub fn close(&self)
    {
        if self.is_dead()
        {
            tracing::warn!("tried to close an already dead client");
            return;
        }
        tracing::info!("client closing self");
        let closure_frame =
            ezsockets::CloseFrame{
                code   : ezsockets::CloseCode::Normal,
                reason : String::from("client done")
            };
        if self.client.close(Some(closure_frame)).is_err()
        {
            tracing::warn!("tried to close an already dead client");
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
        runtime     : Arc<tokio::runtime::Runtime>,
        url         : url::Url,
        auth        : AuthRequest,
        connect_msg : ConnectMsg
    ) -> TokioPendingResult<Client<ServerMsg, ClientMsg, ConnectMsg>>
    {
        tracing::info!("new Client (pending)");
        let factory_clone = self.clone();
        let runtime_clone = runtime.clone();
        TokioPendingResult::<Client<ServerMsg, ClientMsg, ConnectMsg>>::new(
                runtime.spawn( async move {
                        factory_clone.new_client_async(
                                Some(runtime_clone),
                                url,
                                auth,
                                connect_msg
                            ).await
                    } )
            )
    }

    /// New client (async).
    /// - Must be invoked from within a persistent tokio runtime.
    pub async fn new_client_async(&self,
        _runtime    : Option<Arc<tokio::runtime::Runtime>>,
        url         : url::Url,
        auth        : AuthRequest,
        connect_msg : ConnectMsg
    ) -> Client<ServerMsg, ClientMsg, ConnectMsg>
    {
        // prepare to make client connection
        // note: http headers cannot contain raw bytes so we must serialize as json
        let auth_msg_ser    = serde_json::to_string(&auth).expect("could not serialize authentication");
        let connect_msg_ser = serde_json::to_string(&connect_msg).expect("could not serialize connect msg");

        let client_config = ezsockets::ClientConfig::new(url)
            .header(VERSION_MSG_HEADER, self.protocol_version)
            .header(AUTH_MSG_HEADER, auth_msg_ser.as_str())
            .header(CONNECT_MSG_HEADER, connect_msg_ser.as_str());

        // prepare message channel that points out of our client
        let (server_msg_sender, server_msg_receiver) = crossbeam::channel::unbounded::<ServerMsg>();

        // make client core with our handler
        let (client, client_handler_worker) = ezsockets::connect(
                move |_client| { ClientHandler::<ServerMsg>{ server_msg_sender } },
                client_config
            ).await;

        // track client closure
        let client_closed_signal = TokioPendingResult::<()>::new(tokio::spawn(
                async move {
                    if let Err(err) = client_handler_worker.await
                    {
                        tracing::error!(err, "client closed with error");
                    }
                }
            ));

        // finish assembling our client
        Client{
                client,
                server_msg_receiver,
                client_closed_signal,
                _runtime,
                _phantom: PhantomData::default(),
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------
