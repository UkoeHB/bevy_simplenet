//local shortcuts
use crate::*;

//third-party shortcuts
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::collections::HashMap;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn check_protocol_version(
    request          : &ezsockets::Request,
    protocol_version : &'static str
) -> Result<(), Option<ezsockets::CloseFrame>>
{
    // extract protocol version header
    let Some(version_msg_val) = request.headers().get(VERSION_MSG_HEADER)
    else
    {
        tracing::trace!("ConnectionHandler: invalid version message (not present)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Version message missing.")
                }));
    };

    // sanity check the version msg size so we can safely log the version if there is a mismatch
    if version_msg_val.as_bytes().len() > 20
    {
        tracing::trace!("ConnectionHandler: version too big");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Size,
                    reason : String::from("Version oversided.")
                }));
    }

    // check protocol version
    if version_msg_val.as_bytes() != protocol_version.as_bytes()
    {
        tracing::trace!(?version_msg_val, protocol_version, "ConnectionHandler: version mismatch");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Version mismatch.")
                }));
    }

    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn try_extract_client_id(
    request       : &ezsockets::Request,
    authenticator : &Authenticator
) -> Result<u128, Option<ezsockets::CloseFrame>>
{
    // extract auth msg
    let Some(auth_msg_val) = request.headers().get(AUTH_MSG_HEADER)
    else
    {
        tracing::trace!("ConnectionHandler: invalid auth message (not present)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Auth message missing.")
                }));
    };

    // deserialize
    let Ok(auth_request) = serde_json::de::from_slice::<AuthRequest>(auth_msg_val.as_bytes())
    else
    {
        tracing::trace!("ConnectionHandler: invalid auth message (deserialization)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Invalid,
                    reason : String::from("Auth message malformed.")
                }));
    };

    // validate
    if !authenticate(&auth_request, authenticator)
    {
        tracing::trace!("ConnectionHandler: invalid auth message (verification)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Auth message invalid.")
                }));
    }

    Ok(auth_request.client_id())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn try_extract_connect_msg<ConnectMsg>(
    request      : &ezsockets::Request,
    max_msg_size : u32
) -> Result<ConnectMsg, Option<ezsockets::CloseFrame>>
where
    ConnectMsg: for<'de> Deserialize<'de> + 'static,
{
    // extract connect msg
    let Some(connect_msg_val) = request.headers().get(CONNECT_MSG_HEADER)
    else
    {
        tracing::trace!("ConnectionHandler: invalid connect message (not present)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Connect message missing.")
                }));
    };

    // validate size
    // note: since connect messages are serialized as json, the actual deserialized message will be smaller
    //       however, we still limit connect msg sizes to 'max msg size' since the goal is constraining byte throughput
    //       at the network layer
    if connect_msg_val.as_bytes().len() > max_msg_size as usize
    {
        tracing::trace!("ConnectionHandler: invalid connect message (too large)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Size,
                    reason : String::from("Connect message too large.")
                }));
    }

    // deserialize
    let Ok(connect_msg) = serde_json::de::from_slice::<ConnectMsg>(connect_msg_val.as_bytes())
    else
    {
        tracing::trace!("ConnectionHandler: invalid connect message (deserialization)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Invalid,
                    reason : String::from("Connect message malformed.")
                }));
    };

    Ok(connect_msg)
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ConnectionHandler<ServerMsg, ClientMsg, ConnectMsg>
where
    ServerMsg: Clone + Debug + Send + Sync + Serialize,
    ClientMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de>,
    ConnectMsg: Clone + Debug + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    /// authenticator
    pub(crate) authenticator: Authenticator,
    /// protocol version
    pub(crate) protocol_version: &'static str,
    /// config: maximum message size (bytes)
    pub(crate) config: ConnectionConfig,

    /// sender endpoint for reporting connection events
    /// - receiver is in server owner
    pub(crate) connection_report_sender: crossbeam::channel::Sender<ConnectionReport<ConnectMsg>>,
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
        // reject connection if there is a protocol version mismatch
        let _ = check_protocol_version(&request, self.protocol_version)?;

        // reject connection if max connections reached
        if self.session_registry.len() >= self.config.max_connections as usize
        {
            tracing::warn!("ConnectionHandler: max connections reached, dropping connection request...");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Protocol,
                    reason : String::from("Max connections reached.")
                }));
        }

        // try to validate authentication
        let id = try_extract_client_id(&request, &self.authenticator)?;

        // reject connection if client id is already registered as a session
        if self.session_registry.contains_key(&id)
        {
            tracing::warn!(id, "ConnectionHandler: received connection request from already-connected client");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Protocol,
                    reason : String::from("Client is already connected.")
                }));
        }

        // try to extract connect message
        let connect_msg = try_extract_connect_msg(&request, self.config.max_msg_size)?;

        // make a session
        let socket_msg_sender = socket.sink.clone();
        let client_msg_sender = self.client_msg_sender.clone();
        let max_msg_size      = self.config.max_msg_size;
        let rate_limit_config = self.config.rate_limit_config.clone();

        let session = ezsockets::Session::create(
                move |_session_handle|
                {
                    SessionHandler::<ClientMsg>{
                            id,
                            socket_msg_sender,
                            client_msg_sender,
                            max_msg_size,
                            rate_limit_tracker: RateLimitTracker::new(rate_limit_config)
                        }
                },
                id,
                socket
            );

        // register the session
        self.session_registry.insert(id, session.clone());

        // report the new connection
        if let Err(err) = self.connection_report_sender.send(ConnectionReport::<ConnectMsg>::Connected(id, connect_msg))
        {
            tracing::error!(?err, "ConnectionHandler: forwarding connection report failed");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Error,
                    reason : String::from("Server internal error.")
                }));
        };

        Ok(session)
    }

    /// Responds to session disconnects.
    async fn on_disconnect(&mut self, id: SessionID) -> Result<(), ezsockets::Error>
    {
        // unregister session
        tracing::info!(id, "ConnectionHandler: unregistering session");
        self.session_registry.remove(&id);

        // send connection report
        if let Err(err) = self.connection_report_sender.send(ConnectionReport::<ConnectMsg>::Disconnected(id))
        {
            tracing::error!(?err, "ConnectionHandler: forwarding disconnect report failed");
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
            tracing::warn!(session_msg.id, "ConnectionHandler: dropping message sent to unknown session");
            return Ok(());
        };

        // handle input
        match session_msg.msg
        {
            SessionCommand::<ServerMsg>::SendMsg(msg_to_send) =>
            {
                // forward server message to target session
                //todo: .binary() potentially panics
                tracing::trace!(session_msg.id, "ConnectionHandler: sending message to session");
                let Ok(ser_msg) = bincode::serialize(&msg_to_send)
                else
                {
                    tracing::trace!(session_msg.id, "ConnectionHandler: serializing message failed");
                    return Err(Box::new(ConnectionError::SerializationError));
                };
                session.binary(ser_msg);
            }
            SessionCommand::Close =>
            {
                // force the target session to close
                //todo: .call() potentially panics
                tracing::info!(session_msg.id, "ConnectionHandler: closing session");
                session.call(());  //the on_call() handler forces closure
            }
        }

        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------------------------
