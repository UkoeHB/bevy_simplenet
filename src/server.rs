//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::Resource;
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Resource)]
pub struct Server<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
{
    /// sends messages to the internal connection handler
    server_msg_sender: tokio::sync::mpsc::UnboundedSender<SessionTargetMsg<SessionID, SessionCommand<ServerMsg>>>,
    /// receives reports from the internal connection handler
    connection_report_receiver: crossbeam::channel::Receiver<ConnectionReport<ConnectMsg>>,
    /// receives client messages from the internal connection handler
    client_msg_receiver: crossbeam::channel::Receiver<SessionSourceMsg<SessionID, ClientMsg>>,

    /// signal indicates if server internal worker has stopped
    server_closed_signal: TokioPendingResult<()>,
    /// signal indicates if server runner has stopped
    server_running_signal: TokioPendingResult<()>,

    /// cached runtime to ensure server remains operational (optional)
    _runtime: Option<Arc<tokio::runtime::Runtime>>,
}

impl<ServerMsg, ClientMsg, ConnectMsg> Server<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize + 'static,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    /// Associated factory type.
    pub type Factory = ServerFactory<ServerMsg, ClientMsg, ConnectMsg>;

    /// send a message to the target session
    pub fn send_msg(&self, id: SessionID, msg: ServerMsg) -> Result<(), ()>
    {
        // send to endpoint of ezsockets::Server::call() (will be picked up by ConnectionHandler::on_call())
        if self.is_dead()
        {
            tracing::error!(id, "Server: tried to send message to session but server is dead");
            return Err(());
        }
        if let Err(err) = self.server_msg_sender.send(SessionTargetMsg::new(id, SessionCommand::SendMsg(msg)))
        {
            tracing::error!(?err, "Server: failed to forward session message to session");
            return Err(());
        }

        Ok(())
    }

    /// Close the target session.
    /// note: the target session may not be closed until some time after this method is called
    pub fn close_session(&self, id: SessionID, close_frame: ezsockets::CloseFrame) -> Result<(), ()>
    {
        // send to endpoint of ezsockets::Server::call() (will be picked up by ConnectionHandler::on_call())
        tracing::info!(id, "Server: closing client");
        if self.is_dead()
        {
            tracing::error!(id, "Server: tried to close session but server is dead");
            return Err(());
        }
        if let Err(err) = self.server_msg_sender.send(SessionTargetMsg::new(id, SessionCommand::Close(close_frame)))
        {
            tracing::error!(?err, "Server: failed to forward session close command to session");
            return Err(());
        }

        Ok(())
    }

    /// Try to get next available connection report.
    pub fn try_get_next_connection_report(&self) -> Option<ConnectionReport<ConnectMsg>>
    {
        //todo: count connections
        let Ok(msg) = self.connection_report_receiver.try_recv() else { return None; };
        Some(msg)
    }

    /// Try to extract next available message from a client.
    pub fn try_get_next_msg(&self) -> Option<(SessionID, ClientMsg)>
    {
        let Ok(msg) = self.client_msg_receiver.try_recv() else { return None; };
        Some((msg.id, msg.msg))
    }

    /// Test if server is dead.
    pub fn is_dead(&self) -> bool
    {
        self.server_closed_signal.is_done() || self.server_running_signal.is_done()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Factory for producing servers that all bake in the same protocol version.
//todo: use const generics on the protocol version instead (currently broken, async methods cause compiler errors)
#[derive(Debug, Clone)]
pub struct ServerFactory<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
{
    protocol_version : &'static str,
    _phantom         : PhantomData<(ServerMsg, ClientMsg, ConnectMsg)>,
}

impl<ServerMsg, ClientMsg, ConnectMsg> ServerFactory<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize + 'static,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    /// Make a new server factory with a given protocol version.
    pub fn new(protocol_version: &'static str) -> Self
    {
        ServerFactory{ protocol_version, _phantom: PhantomData::default() }
    }

    /// Make a new server.
    pub fn new_server<A>(&self,
        runtime             : Arc<tokio::runtime::Runtime>,
        address             : A,
        connection_acceptor : ezsockets::tungstenite::Acceptor,
        authenticator       : Authenticator,
        config              : ConnectionConfig,
    ) -> Server<ServerMsg, ClientMsg, ConnectMsg>
    where
        A: tokio::net::ToSocketAddrs + Send + 'static
    {
        tracing::info!("new Server (pending)");
        let factory_clone = self.clone();
        let runtime_clone = runtime.clone();
        TokioPendingResult::<Server<ServerMsg, ClientMsg, ConnectMsg>>::new(
                runtime.spawn( async move {
                        factory_clone.new_server_async(
                                Some(runtime_clone),
                                address,
                                connection_acceptor,
                                authenticator,
                                config
                            ).await
                    } )
            ).extract().unwrap().unwrap()
    }

    /// Make a new server (async).
    /// - Must be invoked from within a persistent tokio runtime.
    pub async fn new_server_async<A>(&self,
        _runtime            : Option<Arc<tokio::runtime::Runtime>>,
        address             : A,
        connection_acceptor : ezsockets::tungstenite::Acceptor,
        authenticator       : Authenticator,
        config              : ConnectionConfig
    ) -> Server<ServerMsg, ClientMsg, ConnectMsg>
    where
        A: tokio::net::ToSocketAddrs + Send + 'static
    {
        // prepare message channels that point out of connection handler
        let (
                connection_report_sender,
                connection_report_receiver
            ) = crossbeam::channel::unbounded::<ConnectionReport<ConnectMsg>>();
        let (
                client_msg_sender,
                client_msg_receiver
            ) = crossbeam::channel::unbounded::<SessionSourceMsg<SessionID, ClientMsg>>();

        // make server core with our connection handler
        let (server, server_worker) = ezsockets::Server::create(
                move |_server|
                ConnectionHandler::<ServerMsg, ClientMsg, ConnectMsg>{
                        authenticator,
                        protocol_version: self.protocol_version,
                        config,
                        connection_report_sender,
                        session_registry: HashMap::default(),
                        client_msg_sender,
                        _phantom: std::marker::PhantomData::default()
                    }
            );
        let server_closed_signal = TokioPendingResult::<()>::new(
                tokio::spawn(
                        async move {
                            if let Err(err) = server_worker.await
                            {
                                tracing::error!(?err, "Server: server closed with error");
                            }
                        }
                    )
            );

        // prepare listener
        let connection_listener = tokio::net::TcpListener::bind(address).await.unwrap();

        // launch the server core
        let server_clone = server.clone();
        let server_running_signal = TokioPendingResult::<()>::new(
                tokio::spawn(
                        async move {
                            if let Err(err) = ezsockets::tungstenite::run_on(
                                    server_clone,
                                    connection_listener,
                                    connection_acceptor
                                ).await
                            {
                                tracing::error!(?err, "Server: server stopped running with error");
                            }
                        }
                    )
            );

        // finish assembling our server
        tracing::info!("new Server (done)");
        Server{
                server_msg_sender: server.into(),  //extract the call sender
                connection_report_receiver,
                client_msg_receiver,
                server_closed_signal,
                server_running_signal,
                _runtime
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------
