//disable warnings
//#![allow(irrefutable_let_patterns)]

//local shortcuts
use crate::*;

//third-party shortcuts
use enfync::HandleTrait;
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

async fn websocket_handler<ServerMsg, ClientMsg, ConnectMsg>(
    axum::Extension(server)     : axum::Extension<ezsockets::Server<ConnectionHandler<ServerMsg, ClientMsg, ConnectMsg>>>,
    axum::extract::Query(_query) : axum::extract::Query<HashMap<String, String>>,
    ezsocket_upgrade            : ezsockets::axum::Upgrade,
) -> impl axum::response::IntoResponse
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize + 'static,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    //todo
    /*{
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "we won't accept you because of ......",
        ).into_response();
    }*/
    ezsocket_upgrade.on_upgrade(server)
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

async fn run_server(app: axum::Router, listener: std::net::TcpListener, acceptor_config: AcceptorConfig)
{
    // set listener
    let server = axum_server::Server::from_tcp(listener);

    // set acceptor
    let server = match acceptor_config
    {
        AcceptorConfig::Default         => server.acceptor(axum_server::accept::DefaultAcceptor::new()),
        #[cfg(feature = "tls-rustls")]
        AcceptorConfig::Rustls(config)  => server.acceptor(axum_server::tls_rustls::RustlsAcceptor::new(config)),
        #[cfg(feature = "tls-openssl")]
        AcceptorConfig::OpenSSL(config) => server.acceptor(axum_server::tls_openssl::OpenSSLAcceptor::new(config)),
    };

    // serve it
    if let Err(err) = server.serve(app.into_make_service_with_connect_info::<SocketAddr>()).await
    {
        tracing::error!(?err, "server stopped running with error");
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::system::Resource))]
pub struct Server<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
{
    /// the server's address
    server_address: SocketAddr,
    /// whether or not the server uses TLS
    uses_tls: bool,

    /// sends messages to the internal connection handler
    server_msg_sender: tokio::sync::mpsc::UnboundedSender<SessionTargetMsg<SessionID, SessionCommand<ServerMsg>>>,
    /// receives reports from the internal connection handler
    connection_report_receiver: crossbeam::channel::Receiver<ServerReport<ConnectMsg>>,
    /// receives client messages from the internal connection handler
    client_msg_receiver: crossbeam::channel::Receiver<SessionSourceMsg<SessionID, ClientMsg>>,

    /// signal indicates if server internal worker has stopped
    server_closed_signal: enfync::PendingResult<()>,
    /// signal indicates if server runner has stopped
    server_running_signal: enfync::PendingResult<()>,
}

impl<ServerMsg, ClientMsg, ConnectMsg> Server<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize + 'static,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    /// Associated factory type.
    pub type Factory = ServerFactory<ServerMsg, ClientMsg, ConnectMsg>;

    /// Send a message to the target session.
    /// Messages will be silently dropped if the session is not connected (there may or may not be a trace message).
    /// Returns `Err` if an internal server error occurs.
    pub fn send(&self, id: SessionID, msg: ServerMsg) -> Result<(), ()>
    {
        // send to endpoint of ezsockets::Server::call() (will be picked up by ConnectionHandler::on_call())
        if self.is_dead()
        {
            tracing::warn!(id, "tried to send message to session but server is dead");
            return Err(());
        }
        if let Err(err) = self.server_msg_sender.send(SessionTargetMsg::new(id, SessionCommand::SendMsg(msg)))
        {
            tracing::error!(?err, "failed to forward session message to session");
            return Err(());
        }

        Ok(())
    }

    /// Close the target session.
    /// note: the target session may not be closed until some time after this method is called
    pub fn close_session(&self, id: SessionID, close_frame: ezsockets::CloseFrame) -> Result<(), ()>
    {
        // send to endpoint of ezsockets::Server::call() (will be picked up by ConnectionHandler::on_call())
        tracing::info!(id, "closing client");
        if self.is_dead()
        {
            tracing::warn!(id, "tried to close session but server is dead");
            return Err(());
        }
        if let Err(err) = self.server_msg_sender.send(SessionTargetMsg::new(id, SessionCommand::Close(close_frame)))
        {
            tracing::error!(?err, "failed to forward session close command to session");
            return Err(());
        }

        Ok(())
    }

    /// Try to get the next available connection report.
    pub fn next_report(&self) -> Option<ServerReport<ConnectMsg>>
    {
        //todo: count connections
        let Ok(msg) = self.connection_report_receiver.try_recv() else { return None; };
        Some(msg)
    }

    /// Try to extract the next available message from a client.
    pub fn next_msg(&self) -> Option<(SessionID, ClientMsg)>
    {
        let Ok(msg) = self.client_msg_receiver.try_recv() else { return None; };
        Some((msg.id, msg.msg))
    }

    /// get the server's url
    pub fn url(&self) -> url::Url
    {
        make_websocket_url(self.uses_tls, self.server_address).unwrap()
    }

    /// Test if the server is dead.
    pub fn is_dead(&self) -> bool
    {
        self.server_closed_signal.done() || self.server_running_signal.done()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Configuration for server's connection acceptor. Default is non-TLS.
pub enum AcceptorConfig
{
    Default,
    #[cfg(feature = "tls-rustls")]
    Rustls(axum_server::tls_rustls::RustlsConfig),
    #[cfg(feature = "tls-openssl")]
    OpenSSL(axum_server::tls_openssl::OpenSSLConfig),
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
    ///
    /// Only works with a tokio runtime handle.
    pub fn new_server<A>(&self,
        runtime_handle  : enfync::builtin::native::TokioHandle,
        address         : A,
        acceptor_config : AcceptorConfig,
        authenticator   : Authenticator,
        config          : ServerConfig
    ) -> Server<ServerMsg, ClientMsg, ConnectMsg>
    where
        A: std::net::ToSocketAddrs + Send + 'static,
    {
        #[cfg(wasm)]
        { panic!("bevy simplenet servers not supported on WASM!"); }

        // prepare message channels that point out of connection handler
        let (
                connection_report_sender,
                connection_report_receiver
            ) = crossbeam::channel::unbounded::<ServerReport<ConnectMsg>>();
        let (
                client_msg_sender,
                client_msg_receiver
            ) = crossbeam::channel::unbounded::<SessionSourceMsg<SessionID, ClientMsg>>();

        // make server core with our connection handler
        // note: ezsockets::Server::create() must be called from within a tokio runtime
        let protocol_version = self.protocol_version;
        let (server, server_worker) = enfync::blocking::extract(runtime_handle.spawn(async move {
                ezsockets::Server::create(
                        move |_server|
                        ConnectionHandler::<ServerMsg, ClientMsg, ConnectMsg>{
                                authenticator,
                                protocol_version,
                                config,
                                connection_report_sender,
                                session_registry: HashMap::default(),
                                client_msg_sender,
                                _phantom: std::marker::PhantomData::default()
                            }
                    )
            })).unwrap();
        let server_closed_signal = runtime_handle.spawn(
                async move {
                    if let Err(err) = server_worker.await
                    {
                        tracing::error!(?err, "server closed with error");
                    }
                }
            );

        // prepare listener
        let connection_listener = std::net::TcpListener::bind(address).unwrap();
        let server_address = connection_listener.local_addr().unwrap();
        let uses_tls = !matches!(acceptor_config, AcceptorConfig::Default);

        // launch the server core
        let app = axum::Router::new()
            .route("/ws", axum::routing::get(websocket_handler::<ServerMsg, ClientMsg, ConnectMsg>))
            .layer(axum::Extension(server.clone()));

        let server_running_signal = runtime_handle.spawn(
                async move { run_server(app, connection_listener, acceptor_config).await }
            );

        // finish assembling our server
        tracing::info!("new server created");
        Server{
                server_address,
                uses_tls,
                server_msg_sender: server.into(),  //extract the call sender
                connection_report_receiver,
                client_msg_receiver,
                server_closed_signal,
                server_running_signal,
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------
