//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::collections::HashMap;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SessionTargetMsg<I: Debug + Clone, T: Debug + Clone>
{
    pub(crate) id  : I,
    pub(crate) msg : T
}

impl<I: Debug + Clone, T: Debug + Clone> SessionTargetMsg<I, T>
{
    pub(crate) fn new(id: I, msg: T) -> SessionTargetMsg<I, T> { SessionTargetMsg::<I, T> { id, msg } }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct SessionSourceMsg<I: Debug + Clone, T: Debug + Clone>
{
    pub(crate) id  : I,
    pub(crate) msg : T
}

impl<I: Debug + Clone, T: Debug + Clone> SessionSourceMsg<I, T>
{
    pub(crate) fn new(id: I, msg: T) -> SessionSourceMsg<I, T> { SessionSourceMsg::<I, T> { id, msg } }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) enum SessionCommand<ServerMsg: Debug + Clone>
{
    SendMsg(ServerMsg),
    Close(ezsockets::CloseFrame)
}

//-------------------------------------------------------------------------------------------------------------------

//todo: shut down procedure (implementation currently assumes the server lives until the executable closes)
#[derive(Debug)]
pub(crate) struct ConnectionHandler<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    /// config: maximum message size (bytes)
    pub(crate) config: ServerConfig,
    /// counter for number of connections
    pub(crate) connection_counter: ConnectionCounter,

    /// sender endpoint for reporting connection events
    /// - receiver is in server owner
    pub(crate) connection_report_sender: crossbeam::channel::Sender<ServerReport<ConnectMsg>>,
    /// registered sessions
    pub(crate) session_registry: HashMap<SessionID, ezsockets::Session<SessionID, ()>>,

    /// cached sender endpoint for constructing new sessions
    /// - receiver is in server owner
    pub(crate) client_msg_sender: crossbeam::channel::Sender<SessionSourceMsg<SessionID, ClientMsg>>,

    /// phantom
    pub(crate) _phantom: std::marker::PhantomData<ServerMsg>
}

#[async_trait::async_trait]
impl<ServerMsg, ClientMsg, ConnectMsg> ezsockets::ServerExt for ConnectionHandler<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    type Session = SessionHandler<ClientMsg>;  //Self::Session, not ezsockets::Session
    type Call    = SessionTargetMsg<SessionID, SessionCommand<ServerMsg>>;

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
        if let Err(err) = self.connection_report_sender.send(
                ServerReport::<ConnectMsg>::Connected(info.id, info.connect_msg)
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
        let client_msg_sender = self.client_msg_sender.clone();
        let max_msg_size      = self.config.max_msg_size;
        let rate_limit_config = self.config.rate_limit_config.clone();

        let session = ezsockets::Session::create(
                move | session |
                {
                    SessionHandler::<ClientMsg>{
                            id: info.id,
                            session,
                            client_msg_sender,
                            max_msg_size,
                            client_env_type: info.client_env_type,
                            rate_limit_tracker: RateLimitTracker::new(rate_limit_config)
                        }
                },
                info.id,
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
        if let Err(err) = self.connection_report_sender.send(ServerReport::<ConnectMsg>::Disconnected(id))
        {
            tracing::error!(?err, "forwarding disconnect report failed");
            return Err(Box::new(ConnectionError::SystemError));
        }

        Ok(())
    }

    /// Responds to calls to the server connected to this handler (i.e. ezsockets::Server::call()).
    async fn on_call(
        &mut self,
        session_msg: SessionTargetMsg<SessionID, SessionCommand<ServerMsg>>
    ) -> Result<(), ezsockets::Error>
    {
        // try to get targeted session (ignore if missing)
        let Some(session) = self.session_registry.get(&session_msg.id)
        else
        {
            tracing::warn!(session_msg.id, "dropping message sent to unknown session");
            return Ok(());
        };

        // handle input
        match session_msg.msg
        {
            SessionCommand::<ServerMsg>::SendMsg(msg_to_send) =>
            {
                // serialize message
                tracing::trace!(session_msg.id, "sending message to session");
                let Ok(ser_msg) = bincode::DefaultOptions::new().serialize(&msg_to_send)
                else { tracing::error!(session_msg.id, "serializing message failed"); return Ok(()); };

                // forward server message to target session
                if let Err(_) = session.binary(ser_msg)
                { tracing::error!(session_msg.id, "dropping message sent to broken session"); }
            }
            SessionCommand::<ServerMsg>::Close(close_frame) =>
            {
                // command the target session to close
                tracing::info!(session_msg.id, "closing session");
                if let Err(_) = session.close(Some(close_frame)).await
                { tracing::error!(session_msg.id, "failed closing session"); }
            }
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------------------------
