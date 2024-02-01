//local shortcuts
use crate::*;

//third-party shortcuts
use serde::Deserialize;

//standard shortcuts
use core::fmt::Debug;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn check_protocol_version<'a>(
    query_element    : Option<(Cow<str>, Cow<str>)>,
    protocol_version : &'static str
) -> Result<(), &'static str>
{
    // get query element
    let Some((key, value)) = query_element
    else { tracing::trace!("invalid version message (not present)"); return Err("Version message missing."); };

    // check key
    if key != VERSION_MSG_KEY
    { tracing::trace!("invalid version message (not present)"); return Err("Version message missing."); };

    // sanity check the version msg size so we can safely log the version if there is a mismatch
    if value.len() > 20
    { tracing::trace!("version too big"); return Err("Version oversized."); };

    // check protocol version
    if value != protocol_version
    { tracing::trace!(?value, protocol_version, "version mismatch"); return Err("Version mismatch."); };

    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn try_extract_client_env<'a>(
    query_element: Option<(Cow<str>, Cow<str>)>,
) -> Result<EnvType, &'static str>
{
    // extract env type
    let Some((key, value)) = query_element
    else { tracing::trace!("invalid env type (not present)"); return Err("Env type missing."); };

    // check key
    if key != TYPE_MSG_KEY
    { tracing::trace!("invalid env type (not present)"); return Err("Env type missing."); };

    // get value
    let Some(env_type) = env_type_from_str(&value)
    else { tracing::trace!("invalid env type (unknown)"); return Err("Unknown env type."); };

    Ok(env_type)
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn deserialize_authentication<'a>(
    query_element: &Option<(Cow<str>, Cow<str>)>,
) -> Result<AuthRequest, &'static str>
{
    // extract auth msg
    let Some((key, value)) = query_element
    else { tracing::trace!("invalid auth message (not present)"); return Err("Auth message missing."); };

    // check key
    if key != AUTH_MSG_KEY
    { tracing::trace!("invalid auth message (not present)"); return Err("Auth message missing."); };

    // deserialize
    let Ok(auth_request) = serde_json::de::from_str::<AuthRequest>(&value)
    else { tracing::trace!("invalid auth message (deserialization)"); return Err("Auth message malformed."); };

    Ok(auth_request)
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn validate_authentication<'a>(
    query_element : Option<(Cow<str>, Cow<str>)>,
    authenticator : &Authenticator
) -> Result<(), &'static str>
{
    // deserialize
    let auth_request = deserialize_authentication(&query_element)?;

    // validate
    if !authenticate(&auth_request, authenticator)
    { tracing::trace!("invalid auth message (verification)"); return Err("Auth message invalid."); };

    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn try_extract_client_id<'a>(
    query_element: Option<(Cow<str>, Cow<str>)>,
) -> Result<u128, &'static str>
{
    let auth_request = deserialize_authentication(&query_element)?;
    Ok(auth_request.client_id())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn check_connect_message_size<'a>(
    query_element : Option<(Cow<str>, Cow<str>)>,
    max_msg_size  : u32
) -> Result<(), &'static str>
{
    // extract connect msg
    let Some((key, value)) = query_element
    else { tracing::trace!("invalid connect message (not present)"); return Err("Connect message missing."); };

    // check key
    if key != CONNECT_MSG_KEY
    { tracing::trace!("invalid connect message (not present)"); return Err("Connect message missing."); };

    // validate size
    // note: since connect messages are serialized as json, the actual deserialized message will be smaller
    //       however, we still limit connect msg sizes to 'max msg size' since the goal is constraining byte-throughput
    //       at the network layer
    if value.len() > max_msg_size as usize
    { tracing::trace!("invalid connect message (too large)"); return Err("Connect message too large."); };

    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn try_extract_connect_msg<'a, ConnectMsg>(
    query_element : Option<(Cow<str>, Cow<str>)>,
) -> Result<ConnectMsg, &'static str>
where
    ConnectMsg: for<'de> Deserialize<'de> + 'static,
{
    // extract connect msg
    let Some((key, value)) = query_element
    else { tracing::trace!("invalid connect message (not present)"); return Err("Connect message missing."); };

    // check key
    if key != CONNECT_MSG_KEY
    { tracing::trace!("invalid connect message (not present)"); return Err("Connect message missing."); };

    // deserialize
    let Ok(connect_msg) = serde_json::de::from_str::<ConnectMsg>(&value)
    else { tracing::trace!("invalid connect message (deserialization)"); return Err("Connect message malformed."); };

    Ok(connect_msg)
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct ConnectionCounter
{
    counter: Arc<AtomicU64>,
}

impl ConnectionCounter
{
    pub(crate) fn load(&self) -> u64
    {
        self.counter.load(Ordering::Relaxed)
    }

    pub(crate) fn increment(&self)
    {
        self.counter.fetch_add(1u64, Ordering::Release);
    }

    pub(crate) fn decrement(&self)
    {
        // negate the decrement if we go below zero
        if self.counter.fetch_sub(1u64, Ordering::Release) == u64::MAX
        {
            self.increment();
        }
    }
}

impl Default for ConnectionCounter { fn default() -> Self { Self{ counter: Arc::new(AtomicU64::new(0u64)) } } }

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ConnectionPrevalidator
{
    pub(crate) protocol_version   : &'static str,
    pub(crate) authenticator      : Authenticator,
    pub(crate) max_connections    : u32,
    pub(crate) max_msg_size       : u32,
    pub(crate) heartbeat_interval : Duration,
    pub(crate) keepalive_timeout  : Duration,
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) fn prevalidate_connection_request(
    request         : &ezsockets::Request,
    num_connections : &ConnectionCounter,
    prevalidator    : &ConnectionPrevalidator,
) -> Result<EnvType, (axum::http::StatusCode, &'static str)>
{
    // check max connection count
    // - this is an approximate test since the counter is updated async
    if num_connections.load() >= prevalidator.max_connections as u64
    {
        tracing::trace!("max connections reached, dropping request...");
        return Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, "Max connections reached."));
    };

    // parse request query
    let Some(query) = request.uri().query()
    else
    {
        tracing::trace!("invalid uri query, dropping connection request...");
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid query."));
    };
    let mut query_elements_iterator = form_urlencoded::parse(query.as_bytes());

    // check if there is a protocol version mismatch
    let _ = check_protocol_version(query_elements_iterator.next(), prevalidator.protocol_version)
        .map_err(|reason| (axum::http::StatusCode::BAD_REQUEST, reason))?;

    // check that client env type is present
    let client_env_type = try_extract_client_env(query_elements_iterator.next())
        .map_err(|reason| (axum::http::StatusCode::BAD_REQUEST, reason))?;

    // validate authentication
    validate_authentication(query_elements_iterator.next(), &prevalidator.authenticator)
        .map_err(|reason| (axum::http::StatusCode::BAD_REQUEST, reason))?;

    // validate size of connect message
    // - don't check if deserializable (too expensive for valid connections)
    check_connect_message_size(query_elements_iterator.next(), prevalidator.max_msg_size)
        .map_err(|reason| (axum::http::StatusCode::BAD_REQUEST, reason))?;

    // there should be no more query elements
    let None = query_elements_iterator.next()
    else { return Err((axum::http::StatusCode::PAYLOAD_TOO_LARGE, "Excess query elements.")); };

    Ok(client_env_type)
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ConnectionInfo<ConnectMsg>
{
    pub(crate) client_env_type : EnvType,
    pub(crate) id              : u128,
    pub(crate) connect_msg     : ConnectMsg,
}

//-------------------------------------------------------------------------------------------------------------------

/// Extract connection info from a connection request.
///
/// Assumes the request has already been pre-validated.
pub(crate) fn extract_connection_info<ConnectMsg>(
    request          : &ezsockets::Request,
    session_registry : &HashMap<SessionId, (u64, ezsockets::Session<SessionId, ()>)>,
) -> Result<ConnectionInfo<ConnectMsg>, Option<ezsockets::CloseFrame>>
where
    ConnectMsg: for<'de> Deserialize<'de> + 'static,
{
    // parse request query
    let query = request.uri().query().ok_or(None)?;
    let mut query_elements_iterator = form_urlencoded::parse(query.as_bytes());

    // ignore protocol version
    query_elements_iterator.next();

    // get client's implementation type
    let client_env_type = try_extract_client_env(query_elements_iterator.next()).map_err(|_| None)?;

    // try to get client id
    let id = try_extract_client_id(query_elements_iterator.next()).map_err(|_| None)?;

    // reject connection if client id is already registered as a session
    if session_registry.contains_key(&id)
    {
        tracing::trace!(id, "received connection request from already-connected client");
        return Err(Some(ezsockets::CloseFrame{
                code   : ezsockets::CloseCode::Protocol,
                reason : String::from("Client is already connected.")
            }));
    }

    // try to extract connect message
    let connect_msg = try_extract_connect_msg(query_elements_iterator.next())
        .map_err(
            |reason|
            Some(ezsockets::CloseFrame{
                code   : ezsockets::CloseCode::Protocol,
                reason : String::from(reason)
            })
        )?;

    Ok(ConnectionInfo{
            client_env_type,
            id,
            connect_msg,
        })
}

//-------------------------------------------------------------------------------------------------------------------
