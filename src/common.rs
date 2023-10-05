//local shortcuts

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;
use std::net::SocketAddr;

//-------------------------------------------------------------------------------------------------------------------

pub type SessionID = u128;

//-------------------------------------------------------------------------------------------------------------------

pub(crate) const VERSION_MSG_KEY : &'static str = "v";
pub(crate) const TYPE_MSG_KEY    : &'static str = "t";
pub(crate) const AUTH_MSG_KEY    : &'static str = "a";
pub(crate) const CONNECT_MSG_KEY : &'static str = "c";

//-------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EnvType
{
    Native,
    Wasm,
}

pub(crate) fn env_type() -> EnvType
{
    #[cfg(not(target_family = "wasm"))]
    { EnvType::Native }

    #[cfg(target_family = "wasm")]
    { EnvType::Wasm }
}

pub(crate) fn env_type_as_str(env_type: EnvType) -> &'static str
{
    match env_type
    {
        EnvType::Native => "0",
        EnvType::Wasm   => "1",
    }
}

pub(crate) fn env_type_from_str(env_type: &str) -> Option<EnvType>
{
    match env_type
    {
        "0" => Some(EnvType::Native),
        "1" => Some(EnvType::Wasm),
        _   => None
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Make a websocket url: {ws, wss}://[ip:port]/ws.
pub fn make_websocket_url(with_tls: bool, address: SocketAddr) -> Result<url::Url, ()>
{
    let mut url = url::Url::parse("https://example.net").map_err(|_| ())?;
    let scheme = match with_tls { true => "wss", false => "ws" };
    url.set_scheme(scheme)?;
    url.set_ip_host(address.ip())?;
    url.set_port(Some(address.port()))?;
    url.set_path("/ws");
    Ok(url)
}

//-------------------------------------------------------------------------------------------------------------------
