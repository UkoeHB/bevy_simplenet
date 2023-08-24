//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;
use serde::{Serialize, Deserialize};

//standard shortcuts
use core::fmt::Debug;
use std::borrow::Cow;
use std::collections::HashMap;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn check_protocol_version<'a>(
    query_element    : Option<(Cow<str>, Cow<str>)>,
    protocol_version : &'static str
) -> Result<(), Option<ezsockets::CloseFrame>>
{
    // get query element
    let Some((key, value)) = query_element
    else
    {
        tracing::trace!("invalid version message (not present)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Version message missing.")
                }));
    };

    // check key
    if key != VERSION_MSG_HEADER
    {
        tracing::trace!("invalid version message (not present)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Version message missing.")
                }));
    }

    // sanity check the version msg size so we can safely log the version if there is a mismatch
    if value.len() > 20
    {
        tracing::trace!("version too big");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Size,
                    reason : String::from("Version oversized.")
                }));
    }

    // check protocol version
    if value != protocol_version
    {
        tracing::trace!(?value, protocol_version, "version mismatch");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Version mismatch.")
                }));
    }

    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn try_extract_client_id<'a>(
    query_element : Option<(Cow<str>, Cow<str>)>,
    authenticator : &Authenticator
) -> Result<u128, Option<ezsockets::CloseFrame>>
{
    // extract auth msg
    let Some((key, value)) = query_element
    else
    {
        tracing::trace!("invalid auth message (not present)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Auth message missing.")
                }));
    };

    // check key
    if key != AUTH_MSG_HEADER
    {
        tracing::trace!("invalid auth message (not present)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Auth message missing.")
                }));
    }

    // deserialize
    let Ok(auth_request) = serde_json::de::from_str::<AuthRequest>(&value)
    else
    {
        tracing::trace!("invalid auth message (deserialization)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Invalid,
                    reason : String::from("Auth message malformed.")
                }));
    };

    // validate
    if !authenticate(&auth_request, authenticator)
    {
        tracing::trace!("invalid auth message (verification)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Auth message invalid.")
                }));
    }

    Ok(auth_request.client_id())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn try_extract_connect_msg<'a, ConnectMsg>(
    query_element : Option<(Cow<str>, Cow<str>)>,
    max_msg_size  : u32
) -> Result<ConnectMsg, Option<ezsockets::CloseFrame>>
where
    ConnectMsg: for<'de> Deserialize<'de> + 'static,
{
    // extract connect msg
    let Some((key, value)) = query_element
    else
    {
        tracing::trace!("invalid connect message (not present)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Connect message missing.")
                }));
    };

    // check key
    if key != CONNECT_MSG_HEADER
    {
        tracing::trace!("invalid connect message (not present)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Policy,
                    reason : String::from("Connect message missing.")
                }));
    }

    // validate size
    // note: since connect messages are serialized as json, the actual deserialized message will be smaller
    //       however, we still limit connect msg sizes to 'max msg size' since the goal is constraining byte throughput
    //       at the network layer
    if value.len() > max_msg_size as usize
    {
        tracing::trace!("invalid connect message (too large)");
        return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Size,
                    reason : String::from("Connect message too large.")
                }));
    }

    // deserialize
    let Ok(connect_msg) = serde_json::de::from_str::<ConnectMsg>(&value)
    else
    {
        tracing::trace!("invalid connect message (deserialization)");
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
    pub(crate) config: ServerConfig,

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
        // parse request query
        let Some(query) = request.uri().query()
        else
        {
            tracing::trace!("invalid uri query, dropping connection request...");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Protocol,
                    reason : String::from("Invalid query.")
                }));
        };
        let mut query_elements_iterator = form_urlencoded::parse(query.as_bytes()).map(|(k, v)| (k, v));

        // reject connection if there is a protocol version mismatch
        let _ = check_protocol_version(query_elements_iterator.next(), self.protocol_version)?;

        // reject connection if max connections reached
        if self.session_registry.len() >= self.config.max_connections as usize
        {
            tracing::trace!("max connections reached, dropping connection request...");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Protocol,
                    reason : String::from("Max connections reached.")
                }));
        }

        // try to validate authentication
        let id = try_extract_client_id(query_elements_iterator.next(), &self.authenticator)?;

        // reject connection if client id is already registered as a session
        if self.session_registry.contains_key(&id)
        {
            tracing::trace!(id, "received connection request from already-connected client");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Protocol,
                    reason : String::from("Client is already connected.")
                }));
        }

        // try to extract connect message
        let connect_msg = try_extract_connect_msg(query_elements_iterator.next(), self.config.max_msg_size)?;

        // make a session
        let client_msg_sender = self.client_msg_sender.clone();
        let max_msg_size      = self.config.max_msg_size;
        let rate_limit_config = self.config.rate_limit_config.clone();

        let session = ezsockets::Session::create(
                move | session |
                {
                    SessionHandler::<ClientMsg>{
                            id,
                            session,
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
        if let Err(err) = self.connection_report_sender.send(ServerReport::<ConnectMsg>::Connected(id, connect_msg))
        {
            tracing::error!(?err, "forwarding connection report failed");
            return Err(Some(ezsockets::CloseFrame{
                    code   : ezsockets::CloseCode::Error,
                    reason : String::from("Server internal error.")
                }));
        };

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
        self.session_registry.remove(&id);

        // send connection report
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
