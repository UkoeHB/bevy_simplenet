//local shortcuts

//third-party shortcuts
use serde::{Serialize, Deserialize};
use serde_with::{Bytes, serde_as};
use wasm_timer::{SystemTime, UNIX_EPOCH};

//standard shortcuts
use core::fmt::Debug;
use std::time::Duration;

//-------------------------------------------------------------------------------------------------------------------

/// Secret size for the `Authenticator::Secret` authentication type.
pub const SECRET_AUTH_BYTES: usize = 16;

//-------------------------------------------------------------------------------------------------------------------

/// Byte length of a signature in [`AuthToken`].
pub const AUTH_TOKEN_SIGNATURE_BYTES: usize = 64;

//-------------------------------------------------------------------------------------------------------------------

/// Client id authenticated by auth key.
///
/// Can be validated by an `Authentication` struct.
//todo: consider including user data in the payload
#[serde_as]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct AuthToken
{
    /// The version of the protocol used to generate this auth token.
    pub protocol_version: u16,
    /// Expiration of the token in seconds since UNIX_EPOCH.
    ///
    /// The token is invalid when `current time >= UNIX_EPOCH + expiry`.
    pub expiry: u64,
    /// Client's id 
    pub client_id: u128,
    /// A signature authenticating the client id.
    #[serde_as(as = "Bytes")]
    pub signature: [u8; AUTH_TOKEN_SIGNATURE_BYTES],
}

impl AuthToken
{
    /// Checks if the token has expired.
    pub fn is_expired(&self) -> bool
    {
        self.time_until_expiry() == Duration::default()
    }

    /// Gets the time remaining before the token expires.
    pub fn time_until_expiry(&self) -> Duration
    {
        self.expiration_time()
            .saturating_sub(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
            )
    }

    /// Gets the duration after `UNIX_EPOCH` when the token will expire.
    pub fn expiration_time(&self) -> Duration
    {
        Duration::from_secs(self.expiry)
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// [`Client`](crate::Client) authentication for connecting to a [`Server`](crate::Server).
#[serde_as]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum AuthRequest
{
    None
    {
        client_id: u128
    },
    Secret
    {
        client_id: u128,
        #[serde_as(as = "Bytes")]
        secret: [u8; SECRET_AUTH_BYTES]
    },
    Token
    {
        token: AuthToken
    },
}

impl AuthRequest
{
    pub fn client_id(&self) -> u128
    {
        match self
        {
            AuthRequest::None{client_id}              => *client_id,
            AuthRequest::Secret{client_id, secret: _} => *client_id,
            AuthRequest::Token{token}                 => token.client_id,
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
