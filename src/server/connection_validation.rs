//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::borrow::Cow;
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

#[derive(Debug, Clone)]
pub(crate) struct PendingCounter
{
    counter: Arc<AtomicU64>,
}

impl PendingCounter
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

impl Default for PendingCounter { fn default() -> Self { Self{ counter: Arc::new(AtomicU64::new(0u64)) } } }

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ConnectionPrevalidator
{
    pub(crate) protocol_version   : &'static str,
    pub(crate) max_pending        : u32,
    pub(crate) max_connections    : u32,
    pub(crate) heartbeat_interval : Duration,
    pub(crate) keepalive_timeout  : Duration,
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) fn prevalidate_connection_request(
    request         : &ezsockets::Request,
    num_pending     : &PendingCounter,
    num_connections : &ConnectionCounter,
    prevalidator    : &ConnectionPrevalidator,
) -> Result<EnvType, (axum::http::StatusCode, &'static str)>
{
    // check max connection counts
    // - this is an approximate test since the counters are updated async
    if num_pending.load() >= prevalidator.max_pending as u64
    {
        tracing::trace!("max pending connections reached, dropping request...");
        return Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, "Max pending connections."));
    }
    if num_connections.load() >= prevalidator.max_connections as u64
    {
        tracing::trace!("max connections reached, dropping request...");
        return Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, "Max connections."));
    }

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

    // there should be no more query elements
    let None = query_elements_iterator.next()
    else { return Err((axum::http::StatusCode::PAYLOAD_TOO_LARGE, "Excess query elements.")); };

    Ok(client_env_type)
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ConnectionInfo
{
    pub(crate) client_env_type: EnvType,
}

//-------------------------------------------------------------------------------------------------------------------

/// Extract connection info from a connection request.
///
/// Assumes the request has already been pre-validated.
pub(crate) fn extract_connection_info(
    request: &ezsockets::Request,
) -> Result<ConnectionInfo, Option<ezsockets::CloseFrame>>
{
    // parse request query
    let query = request.uri().query().ok_or(None)?;
    let mut query_elements_iterator = form_urlencoded::parse(query.as_bytes());

    // ignore protocol version
    query_elements_iterator.next();

    // get client's implementation type
    let client_env_type = try_extract_client_env(query_elements_iterator.next()).map_err(|_| None)?;

    Ok(ConnectionInfo{ client_env_type, })
}

//-------------------------------------------------------------------------------------------------------------------
