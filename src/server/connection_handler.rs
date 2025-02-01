//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;
use enfync::{Handle, TryAdopt};

//standard shortcuts
use core::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::HashMap;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn reject_client_request<Channel: ChannelPack>(
    session    : &ezsockets::Session<SessionId, ()>,
    session_id : SessionId,
    request_id : u64
){
    // pack the message
    let packed_msg = ClientMetaEventFrom::<Channel>::Reject(request_id);

    // serialize message
    tracing::trace!(session_id, "sending request rejection to session");
    let Ok(ser_msg) = bincode::DefaultOptions::new().serialize(&packed_msg)
    else { tracing::error!(session_id, "serializing rejection failed"); return; };

    // forward server message to target session
    // - this may fail if the session is disconnected
    if let Err(_) = session.binary(ser_msg)
    { tracing::debug!(session_id, "dropping request rejection sent to broken session"); }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

//todo: shut down procedure (implementation currently assumes the server lives until the executable closes)
#[derive(Debug)]
pub(crate) struct ConnectionHandler<Channel: ChannelPack>
{
    /// authenticator used to evaluate authentication requests
    pub(crate) authenticator: Arc<Authenticator>,

    /// config
    pub(crate) config: ServerConfig,

    /// counter for number of pending connections
    ///
    /// Includes both pending and fully-connected clients.
    pub(crate) pending_counter: PendingCounter,
    /// counter for number of authenticated connections
    pub(crate) connection_counter: ConnectionCounter,
    /// counter for assigning session ids
    pub(crate) session_counter: u64,
    /// counter for total number of authenticated connections encountered
    pub(crate) total_connections_count: u64,

    /// registered sessions
    pub(crate) session_registry: HashMap<SessionId, ezsockets::Session<SessionId, ()>>,

    /// session id to client id maps
    ///
    /// The client-session map includes the connection count for each session, which is used to synchronize with
    /// connection events.
    pub(crate) client_to_session: HashMap<ClientId, (SessionId, u64)>,
    pub(crate) session_to_client: HashMap<SessionId, ClientId>,

    /// Sends client events to the internal connection handler.
    pub(crate) client_event_sender: tokio::sync::mpsc::UnboundedSender<
        ClientTargetMsg<ClientId, SessionCommand<Channel>>
    >,
    /// cached sender endpoint for constructing new sessions
    /// - receiver is in server owner
    pub(crate) server_event_sender: crossbeam::channel::Sender<ClientSourceMsg<ClientId, ServerEventFrom<Channel>>>,
}

#[async_trait::async_trait]
impl<Channel: ChannelPack> ezsockets::ServerExt for ConnectionHandler<Channel>
{
    type Session = SessionHandler<Channel>;  //Self::Session, not ezsockets::Session
    type Call    = ClientTargetMsg<ClientId, SessionCommand<Channel>>;

    /// Produces server sessions for new connections.
    async fn on_connect(
        &mut self,
        socket   : ezsockets::Socket,
        request  : ezsockets::Request,
        _address : std::net::SocketAddr,
    ) -> Result<ezsockets::Session<SessionId, ()>, Option<ezsockets::CloseFrame>>
    {
        // reject connection if max connections reached
        if self.session_registry.len() >= (self.config.max_connections + self.config.max_pending) as usize
        {
            tracing::trace!("max connections reached, dropping connection request...");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Protocol,
                    reason : "max connections".into()
                }));
        }

        // extract info from the request
        let info = extract_connection_info(&request)?;

        // increment the pending counter
        self.pending_counter.increment();
        self.session_counter = self.session_counter.checked_add(1).expect("ran out of session ids");
        let session_id = self.session_counter;

        // make a session
        let authenticator       = self.authenticator.clone();
        let client_event_sender = self.client_event_sender.clone();
        let server_event_sender = self.server_event_sender.clone();
        let max_msg_size        = self.config.max_msg_size;
        let rate_limit_config   = self.config.rate_limit_config.clone();
        let (auth_signal_sender, mut auth_signal_receiver) = tokio::sync::mpsc::channel(1);

        let session = ezsockets::Session::create(
                move |session|
                {
                    // prep client request rejector
                    let session_clone = session.clone();
                    let request_rejector =
                        move |request_id: u64|
                        {
                            reject_client_request::<Channel>(&session_clone, session_id, request_id);
                        };

                    // make session handler
                    SessionHandler::<Channel>{
                            authenticator,
                            id: session_id,
                            client_id: None,
                            session,
                            auth_signal_sender,
                            client_event_sender,
                            server_event_sender,
                            max_msg_size,
                            env_type: info.client_env_type,
                            rate_limit_tracker: RateLimitTracker::new(rate_limit_config),
                            request_rejector: Arc::new(request_rejector),
                            death_signal: Arc::new(AtomicBool::new(false)),
                        }
                },
                session_id,
                socket
            );

        // wait for the session to become fully authenticated, and if doesn't then close it
        let handle = enfync::builtin::native::TokioHandle::try_adopt().unwrap();
        let auth_timeout = self.config.auth_timeout;
        let session_clone = session.clone();

        handle.spawn(
                async move {
                    tokio::select! {
                        biased; // Successful auth takes priority.
                        _ = auth_signal_receiver.recv() => return,
                        _ = tokio::time::sleep(auth_timeout) =>
                        {
                            // Tell the session to close itself.
                            let _ = session_clone.close(Some(
                                    ezsockets::CloseFrame
                                    {
                                        code   : ezsockets::CloseCode::Policy,
                                        reason : "no auth received".into()
                                    }
                                ));
                        }
                    }
                }
            );

        // save session in registry while it's waiting to be authenticated
        self.session_registry.insert(session_id, session.clone());

        Ok(session)
    }

    /// Responds to session disconnects.
    async fn on_disconnect(
        &mut self,
        id      : SessionId,
        _reason : Result<Option<ezsockets::CloseFrame>, ezsockets::Error>
    ) -> Result<(), ezsockets::Error>
    {
        // unregister session
        tracing::info!(id, "unregistering session");
        self.session_registry.remove(&id);

        // clean up session/client id maps
        let Some(client_id) = self.session_to_client.remove(&id)
        else
        {
            self.pending_counter.decrement();
            tracing::debug!(id, "disconnecting unathenticated session");
            return Ok(());
        };
        self.connection_counter.decrement();
        let _ = self.client_to_session.remove(&client_id);

        // send disconnect report
        let report = ServerReport::<Channel::ConnectMsg>::Disconnected;
        if let Err(err) = self.server_event_sender.send(
                ClientSourceMsg::new(client_id, ServerEventFrom::<Channel>::Report(report))
            )
        {
            // This is not an error if the disconnect was received when shutting down the server.
            tracing::warn!(?err, "forwarding disconnect report failed");
            return Err(Box::new(ConnectionError::SystemError));
        }

        Ok(())
    }

    /// Responds to calls to the server connected to this handler (i.e. ezsockets::Server::call()).
    async fn on_call(
        &mut self,
        client_msg: ClientTargetMsg<ClientId, SessionCommand<Channel>>
    ) -> Result<(), ezsockets::Error>
    {
        // handle newly authenticated clients
        // - We overload ClientTargetMsg for this due to the limited API surface.
        if let SessionCommand::<Channel>::Add{ session_id, msg, env_type } = client_msg.msg
        {
            let Some(session) = self.session_registry.get(&session_id)
            else
            {
                // Not an error since the client may have disconnected while this message was in transit.
                tracing::debug!(session_id, "ignoring authentication from unknown session");
                return Ok(());
            };

            // check if the client already exists
            if self.client_to_session.contains_key(&client_msg.id)
            {
                let _ = session.close(Some(
                    ezsockets::CloseFrame
                    {
                        code   : ezsockets::CloseCode::Policy,
                        reason : "client already connected".into()
                    }
                ));

                return Ok(());
            }

            // add client to connected
            self.pending_counter.decrement();
            self.connection_counter.increment();
            self.total_connections_count += 1;

            self.client_to_session.insert(client_msg.id, (session_id, self.total_connections_count));
            self.session_to_client.insert(session_id, client_msg.id);

            // report the connection
            let report = ServerReport::Connected(env_type, msg);
            if let Err(err) = self.server_event_sender.send(
                    ClientSourceMsg::new(client_msg.id, ServerEvent::Report(report))
                )
            {
                tracing::debug!(?err, "client msg sender is broken, closing session...");
                let _ = session.close(Some(
                    ezsockets::CloseFrame
                    {
                        code   : ezsockets::CloseCode::Away,
                        reason : Default::default(),
                    }
                ));
            };

            return Ok(())
        }

        // try to get targeted session (ignore if missing)
        let Some((session_id, connection_idx)) = self.client_to_session.get(&client_msg.id)
        else
        {
            tracing::debug!(client_msg.id, "dropping message sent to unknown client");
            return Ok(());
        };
        let Some(session) = self.session_registry.get(session_id)
        else
        {
            tracing::debug!(client_msg.id, "dropping message sent to unknown session");
            return Ok(());
        };

        // handle input
        match client_msg.msg
        {
            //todo: consider marshalling the message into the session via Session::call() so the session's
            //      thread can do serializing instead of the connection handler which is a bottleneck
            SessionCommand::<Channel>::Send(msg_to_send, maybe_consumed_count, maybe_death_signal) =>
            {
                // check if the connection event for the target session was consumed before this message was sent
                if let Some(consumed_count) = maybe_consumed_count
                {
                    if consumed_count < *connection_idx
                    {
                        tracing::debug!(consumed_count, connection_idx,
                            "dropping message targeted at session before its connection event was handled");
                        return Ok(());
                    }
                }

                // check if the target session is still alive (for request/response patterns)
                // - Note that this check synchronizes with the session registry, guaranteeing our response can only be
                //   sent to the request's originating session. Synchronization: in order for `self.session_registry`
                //   to return a new session, `Self::on_disconnect` must be called for the old session. Once
                //   `Self::on_disconnect` is called, the SessionHandler for the old session will have been dropped,
                //   guaranteeing this death signal will be set.
                if let Some(death_signal) = maybe_death_signal
                {
                    if death_signal.is_dead()
                    { tracing::debug!(client_msg.id, "dropping response targeted at dead session"); return Ok(()); }
                }

                // serialize message
                tracing::trace!(client_msg.id, "sending message to client");
                let Ok(ser_msg) = bincode::DefaultOptions::new().serialize(&msg_to_send)
                else { tracing::error!(client_msg.id, "serializing message failed"); return Ok(()); };

                // forward server message to target session
                // - this may fail if the session is disconnected
                if let Err(_) = session.binary(ser_msg)
                { tracing::debug!(client_msg.id, "dropping message sent to broken session"); }
            }
            SessionCommand::<Channel>::Close(close_frame) =>
            {
                // command the target session to close
                // - this may fail if the session is disconnected
                tracing::info!(client_msg.id, "closing session");
                if let Err(_) = session.close(close_frame)
                { tracing::debug!(client_msg.id, "failed closing session"); }
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------------------------
