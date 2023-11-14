//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;

//standard shortcuts
use core::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::HashMap;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn reject_client_request<Channel: ChannelPack>(
    session    : &ezsockets::Session<SessionID, ()>,
    session_id : SessionID,
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
    /// config: maximum message size (bytes)
    pub(crate) config: ServerConfig,
    /// counter for number of connections
    pub(crate) connection_counter: ConnectionCounter,

    /// registered sessions
    pub(crate) session_registry: HashMap<SessionID, ezsockets::Session<SessionID, ()>>,

    /// cached sender endpoint for constructing new sessions
    /// - receiver is in server owner
    pub(crate) server_event_sender: crossbeam::channel::Sender<SessionSourceMsg<SessionID, ServerEventFrom<Channel>>>,
}

#[async_trait::async_trait]
impl<Channel: ChannelPack> ezsockets::ServerExt for ConnectionHandler<Channel>
{
    type Session = SessionHandler<Channel>;  //Self::Session, not ezsockets::Session
    type Call    = SessionTargetMsg<SessionID, SessionCommand<Channel>>;

    /// Produces server sessions for new connections.
    async fn on_connect(
        &mut self,
        socket   : ezsockets::Socket,
        request  : ezsockets::Request,
        _address : std::net::SocketAddr,
    ) -> Result<ezsockets::Session<SessionID, ()>, Option<ezsockets::CloseFrame>>
    {
        // reject connection if max connections reached
        if self.session_registry.len() >= self.config.max_connections as usize
        {
            tracing::trace!("max connections reached, dropping connection request...");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Protocol,
                    reason : String::from("Max connections reached.")
                }));
        }

        // extract info from the request
        let info = extract_connection_info(&request, &self.session_registry)?;

        // report the new connection
        let report = ServerReport::<Channel::ConnectMsg>::Connected(info.client_env_type, info.connect_msg);
        if let Err(err) = self.server_event_sender.send(
                SessionSourceMsg::new(info.id, ServerEventFrom::<Channel>::Report(report))
            )
        {
            tracing::error!(?err, "forwarding connection report failed");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Error,
                    reason : String::from("Server internal error.")
                }));
        };

        // increment the connection counter now so the updated value is available asap
        self.connection_counter.increment();

        // make a session
        let session_id        = info.id;
        let server_event_sender = self.server_event_sender.clone();
        let max_msg_size      = self.config.max_msg_size;
        let rate_limit_config = self.config.rate_limit_config.clone();

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
                            id: session_id,
                            session,
                            server_event_sender,
                            max_msg_size,
                            client_env_type: info.client_env_type,
                            rate_limit_tracker: RateLimitTracker::new(rate_limit_config),
                            request_rejector: Arc::new(request_rejector),
                            death_signal: Arc::new(AtomicBool::new(false)),
                        }
                },
                session_id,
                socket
            );

        // register the session
        self.session_registry.insert(info.id, session.clone());

        Ok(session)
    }

    /// Responds to session disconnects.
    async fn on_disconnect(
        &mut self,
        id      : SessionID,
        _reason : Result<Option<ezsockets::CloseFrame>, ezsockets::Error>
    ) -> Result<(), ezsockets::Error>
    {
        // unregister session
        tracing::info!(id, "unregistering session");
        self.connection_counter.decrement();
        self.session_registry.remove(&id);

        // send disconnect report
        let report = ServerReport::<Channel::ConnectMsg>::Disconnected;
        if let Err(err) = self.server_event_sender.send(
                SessionSourceMsg::new(id, ServerEventFrom::<Channel>::Report(report))
            )
        {
            tracing::error!(?err, "forwarding disconnect report failed");
            return Err(Box::new(ConnectionError::SystemError));
        }

        Ok(())
    }

    /// Responds to calls to the server connected to this handler (i.e. ezsockets::Server::call()).
    async fn on_call(
        &mut self,
        session_msg: SessionTargetMsg<SessionID, SessionCommand<Channel>>
    ) -> Result<(), ezsockets::Error>
    {
        // try to get targeted session (ignore if missing)
        let Some(session) = self.session_registry.get(&session_msg.id)
        else
        {
            tracing::debug!(session_msg.id, "dropping message sent to unknown session");
            return Ok(());
        };

        // handle input
        match session_msg.msg
        {
            //todo: consider marshalling the message into the session via Session::call() so the session's
            //      thread can do serializing instead of the connection handler which is a bottleneck
            SessionCommand::<Channel>::Send(msg_to_send, maybe_death_signal) =>
            {
                // check if the target session is still alive (for request/response patterns)
                // - note that this check synchronizes with the session registry, guaranteeing our response can only be
                //   sent to the request's originating session
                if let Some(death_signal) = maybe_death_signal
                {
                    if death_signal.is_dead()
                    { tracing::debug!("dropping response targeted at dead session"); return Ok(()); }
                }

                // serialize message
                tracing::trace!(session_msg.id, "sending message to session");
                let Ok(ser_msg) = bincode::DefaultOptions::new().serialize(&msg_to_send)
                else { tracing::error!(session_msg.id, "serializing message failed"); return Ok(()); };

                // forward server message to target session
                // - this may fail if the session is disconnected
                if let Err(_) = session.binary(ser_msg)
                { tracing::debug!(session_msg.id, "dropping message sent to broken session"); }
            }
            SessionCommand::<Channel>::Close(close_frame) =>
            {
                // command the target session to close
                // - this may fail if the session is disconnected
                tracing::info!(session_msg.id, "closing session");
                if let Err(_) = session.close(Some(close_frame))
                { tracing::debug!(session_msg.id, "failed closing session"); }
            }
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------------------------
