//local shortcuts

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::time::Duration;

//-------------------------------------------------------------------------------------------------------------------

/// Config for the [`Client`].
#[derive(Debug)]
pub struct ClientConfig
{
    /// Try to reconnect if the client is disconnected. Defaults to `true`.
    pub reconnect_on_disconnect: bool,
    /// Try to reconnect if the client is closed by the server. Defaults to `false`.
    pub reconnect_on_server_close: bool,
    /// Reconnect interval (delay between reconnect attempts). Defaults to 2 seconds.
    pub reconnect_interval: Duration,
    /// Maximum number of connection attempts when initially connecting. Defaults to infinite.
    pub max_initial_connect_attempts: usize,
    /// Maximum number of reconnect attempts when reconnecting. Defaults to infinite.
    pub max_reconnect_attempts: usize,
    /// Duration between socket heartbeat pings if the connection is inactive. Defaults to 5 seconds.
    pub heartbeat_interval: Duration,
    /// Duration after which a socket will shut down if the connection is inactive. Defaults to 10 seconds
    pub keepalive_timeout: Duration,
}

impl Default for ClientConfig
{
    fn default() -> ClientConfig
    {
        ClientConfig{
                reconnect_on_disconnect      : true,
                reconnect_on_server_close    : false,
                reconnect_interval           : Duration::from_secs(2),
                max_initial_connect_attempts : usize::MAX,
                max_reconnect_attempts       : usize::MAX,
                heartbeat_interval           : Duration::from_secs(5),
                keepalive_timeout            : Duration::from_secs(10),
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------
