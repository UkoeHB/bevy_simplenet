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

/// Configuration for accepting connections to the [`Server`]. Defaults to non-TLS.
pub enum AcceptorConfig
{
    Default,
    #[cfg(feature = "tls-rustls")]
    Rustls(axum_server::tls_rustls::RustlsConfig),
    #[cfg(feature = "tls-openssl")]
    OpenSSL(axum_server::tls_openssl::OpenSSLConfig),
}

//-------------------------------------------------------------------------------------------------------------------
