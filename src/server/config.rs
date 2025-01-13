//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::time::Duration;

//-------------------------------------------------------------------------------------------------------------------

/// Config for the [`Server`].
#[derive(Debug, Copy, Clone)]
pub struct ServerConfig
{
    /// Max number of pending client connections. Defaults to 11K.
    pub max_pending: u32,
    /// Max number of concurrent client connections. Defaults to 100K.
    ///
    /// In practice the number of connections can fluctuate up to `max_connections + max_pending` if connections
    /// arrive very quickly. This flexibility ensures if a session passes pre-validation and connects, then if its
    /// authentication is valid it won't be force-disconnected.
    pub max_connections: u32,
    /// Max message size allowed from clients (in bytes). Defaults to 1MB.
    pub max_msg_size: u32,
    /// Duration to wait for an authentication message after a session connects. Defaults to 3 seconds.
    ///
    /// Authentication is sent securly over websocket channels, so we have a 'waiting period' after a session
    /// initially connects to get its authentication. Sessions that don't authenticate will time out and be closed.
    pub auth_timeout: Duration,
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
                max_pending        : 10_000u32,
                max_connections    : 100_000u32,
                max_msg_size       : 1_000_000u32,
                auth_timeout       : Duration::from_secs(3),
                rate_limit_config  : RateLimitConfig::default(),
                heartbeat_interval : Duration::from_secs(5),
                keepalive_timeout  : Duration::from_secs(10),
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Configuration for accepting connections to the [`Server`]. Defaults to non-TLS.
#[derive(Default)]
pub enum AcceptorConfig
{
    #[default]
    Default,

    #[cfg(feature = "tls-rustls")]
    Rustls(axum_server::tls_rustls::RustlsConfig),

    #[cfg(feature = "tls-openssl")]
    OpenSSL(axum_server::tls_openssl::OpenSSLConfig),
}

//-------------------------------------------------------------------------------------------------------------------
