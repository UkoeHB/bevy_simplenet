//local shortcuts
use crate::*;

//third-party shortcuts
use axum::response::IntoResponse;
use enfync::Handle;

//standard shortcuts
use core::fmt::Debug;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn socket_config(prevalidator: &ConnectionPrevalidator, client_env_type: EnvType) -> ezsockets::SocketConfig
{
    match client_env_type
    {
        EnvType::Native =>
        {
            // use the default heartbeat ping message function
            ezsockets::SocketConfig{
                heartbeat : prevalidator.heartbeat_interval,
                timeout   : prevalidator.keepalive_timeout,
                ..Default::default()
            }
        }
        EnvType::Wasm =>
        {
            // use a custom Text-based ping message
            ezsockets::SocketConfig{
                    heartbeat : prevalidator.heartbeat_interval,
                    timeout   : prevalidator.keepalive_timeout,
                    heartbeat_ping_msg_fn : Arc::new(text_ping_fn)
                }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

async fn websocket_handler<Msgs: MsgPack>(
    axum::Extension(server) : axum::Extension<ezsockets::Server<ConnectionHandler<Msgs>>>,
    axum::Extension(count)  : axum::Extension<ConnectionCounter>,
    axum::Extension(preval) : axum::Extension<Arc<ConnectionPrevalidator>>,
    ezsocket_upgrade        : ezsockets::axum::Upgrade,
) -> impl axum::response::IntoResponse
{
    // prevalidate then prepare upgrade
    match prevalidate_connection_request(ezsocket_upgrade.request(), &count, &preval)
    {
        Ok(client_env_type) => ezsocket_upgrade.on_upgrade_with_config(server, socket_config(&preval, client_env_type)),
        Err(err) => err.into_response()
    }
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

/// Server-enforced constraints on client connections.
#[derive(Debug, Copy, Clone)]
pub struct ServerConfig
{
    /// Max number of concurrent client connections. Defaults to 100K.
    pub max_connections: u32,
    /// Max message size allowed from clients (bytes). Defaults to 1MB.
    pub max_msg_size: u32,
    /// Rate limit for messages received from a session. See [`RateLimitConfig`] for defaults.
    pub rate_limit_config: RateLimitConfig,
    /// Duration between socket heartbeat pings if the connection is inactive. Defaults to 5 seconds.
    pub heartbeat_interval: Duration,
    /// Duration after which a socket will shut down if the connection is inactive. Defaults to 10 seconds.
    pub keepalive_timeout: Duration,
}

impl Default for ServerConfig
{
    fn default() -> ServerConfig
    {
        ServerConfig{
                max_connections    : 100_000u32,
                max_msg_size       : 1_000_000u32,
                rate_limit_config  : RateLimitConfig::default(),
                heartbeat_interval : Duration::from_secs(5),
                keepalive_timeout  : Duration::from_secs(10),
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Emitted by servers when a client connects/disconnects.
#[derive(Debug, Clone)]
pub enum ServerReport<ConnectMsg: Debug + Clone>
{
    Connected(SessionID, ConnectMsg),
    Disconnected(SessionID)
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::system::Resource))]
pub struct Server<Msgs: MsgPack>
{
    /// the server's address
    server_address: SocketAddr,
    /// whether or not the server uses TLS
    uses_tls: bool,
    /// number of current connections
    connection_counter: ConnectionCounter,

    /// sends messages to the internal connection handler
    server_val_sender: tokio::sync::mpsc::UnboundedSender<
        SessionTargetMsg<SessionID, SessionCommandFromPack<Msgs>>
    >,
    /// receives reports from the internal connection handler
    connection_report_receiver: crossbeam::channel::Receiver<ServerReport<Msgs::ConnectMsg>>,
    /// receives client messages from the internal connection handler
    client_val_receiver: crossbeam::channel::Receiver<SessionSourceMsg<SessionID, ClientValFromPack<Msgs>>>,

    /// signal indicates if server internal worker has stopped
    server_closed_signal: enfync::PendingResult<()>,
    /// signal indicates if server runner has stopped
    server_running_signal: enfync::PendingResult<()>,
}

impl<Msgs: MsgPack> Server<Msgs>
{
    /// Send a message to the target session.
    /// - Messages will be silently dropped if the session is not connected (there may or may not be a trace message).
    /// - Returns `Err` if an internal server error occurs.
    pub fn send(&self, id: SessionID, msg: Msgs::ServerMsg) -> Result<(), ()>
    {
        // send to endpoint of ezsockets::Server::call() (will be picked up by ConnectionHandler::on_call())
        if self.is_dead()
        {
            tracing::warn!(id, "tried to send message to session but server is dead");
            return Err(());
        }
        if let Err(err) = self.server_val_sender.send(
                SessionTargetMsg::new(id, SessionCommandFromPack::<Msgs>::Send(ServerValFromPack::<Msgs>::Msg(msg)))
            )
        {
            tracing::error!(?err, "failed to forward session message to session");
            return Err(());
        }

        Ok(())
    }

    //todo: respond

    //todo: acknowledge

    //todo: reject request

    /// Close the target session.
    ///
    /// Note: The target session may remain open until some time after this method is called.
    pub fn close_session(&self, id: SessionID, close_frame: ezsockets::CloseFrame) -> Result<(), ()>
    {
        // send to endpoint of ezsockets::Server::call() (will be picked up by ConnectionHandler::on_call())
        tracing::info!(id, "closing client");
        if self.is_dead()
        {
            tracing::warn!(id, "tried to close session but server is dead");
            return Err(());
        }
        if let Err(err) = self.server_val_sender.send(
                SessionTargetMsg::new(id, SessionCommandFromPack::<Msgs>::Close(close_frame))
            )
        {
            tracing::error!(?err, "failed to forward session close command to session");
            return Err(());
        }

        Ok(())
    }

    /// Try to get the next available connection report.
    pub fn next_report(&self) -> Option<ServerReport<Msgs::ConnectMsg>>
    {
        //todo: count connections
        let Ok(msg) = self.connection_report_receiver.try_recv() else { return None; };
        Some(msg)
    }

    /// Try to extract the next available value from a client.
    pub fn next_val(&self) -> Option<(SessionID, ClientValFromPack<Msgs>)>
    {
        let Ok(msg) = self.client_val_receiver.try_recv() else { return None; };
        Some((msg.id, msg.msg))
    }

    /// get the server's url
    pub fn url(&self) -> url::Url
    {
        make_websocket_url(self.uses_tls, self.server_address).unwrap()
    }

    /// Get number of connections.
    pub fn num_connections(&self) -> u64
    {
        self.connection_counter.load()
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
pub struct ServerFactory<Msgs: MsgPack>
{
    protocol_version : &'static str,
    _phantom         : PhantomData<Msgs>,
}

impl<Msgs: MsgPack> ServerFactory<Msgs>
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
    ) -> Server<Msgs>
    where
        A: std::net::ToSocketAddrs + Send + 'static,
    {
        #[cfg(wasm)]
        { panic!("bevy simplenet servers not supported on WASM!"); }

        // prepare message channels that point out of connection handler
        let (
                connection_report_sender,
                connection_report_receiver
            ) = crossbeam::channel::unbounded::<ServerReport<Msgs::ConnectMsg>>();
        let (
                client_val_sender,
                client_val_receiver
            ) = crossbeam::channel::unbounded::<SessionSourceMsg<SessionID, ClientValFromPack<Msgs>>>();

        // prepare connection counter
        // - this is used to communication the current number of connections from the connection handler to the
        //   connection prevalidator
        let connection_counter = ConnectionCounter::default();

        // make server core with our connection handler
        // note: ezsockets::Server::create() must be called from within a tokio runtime
        let connection_counter_clone = connection_counter.clone();

        let (server, server_worker) = enfync::blocking::extract(runtime_handle.spawn(async move {
                ezsockets::Server::create(
                        move |_server|
                        ConnectionHandler::<Msgs>{
                                config,
                                connection_counter: connection_counter_clone,
                                connection_report_sender,
                                session_registry: HashMap::default(),
                                client_val_sender,
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

        // prepare prevalidator
        let prevalidator = ConnectionPrevalidator{
                protocol_version   : self.protocol_version,
                authenticator,
                max_connections    : config.max_connections,
                max_msg_size       : config.max_msg_size,
                heartbeat_interval : config.heartbeat_interval,
                keepalive_timeout  : config.keepalive_timeout,
            };

        // prepare router
        let app = axum::Router::new()
            .route("/ws", axum::routing::get(websocket_handler::<Msgs>))
            .layer(axum::Extension(server.clone()))
            .layer(axum::Extension(Arc::new(prevalidator)))
            .layer(axum::Extension(connection_counter.clone()));

        // prepare listener
        let connection_listener = std::net::TcpListener::bind(address).unwrap();
        let server_address = connection_listener.local_addr().unwrap();
        let uses_tls = !matches!(acceptor_config, AcceptorConfig::Default);

        // launch the server core
        let server_running_signal = runtime_handle.spawn(
                async move { run_server(app, connection_listener, acceptor_config).await }
            );

        // finish assembling our server
        tracing::info!("new server created");
        Server{
                server_address,
                uses_tls,
                connection_counter,
                server_val_sender: server.into(),  //extract the call sender
                connection_report_receiver,
                client_val_receiver,
                server_closed_signal,
                server_running_signal,
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------
