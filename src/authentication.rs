//local shortcuts

//third-party shortcuts
use serde::{Serialize, Deserialize};
use serde_with::{Bytes, serde_as};

//standard shortcuts
use core::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn authenticate_none(request: &AuthRequest) -> bool
{
    let AuthRequest::None{client_id: _} = request else { return false; };
    return true;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn authenticate_secret(request: &AuthRequest, expected_secret: &[u8; SECRET_AUTH_BYTES]) -> bool
{
    let AuthRequest::Secret{client_id: _, secret} = request else { return false; };
    return *secret == *expected_secret;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn authenticate_token(request: &AuthRequest) -> bool
{
    let AuthRequest::Token{token: _} = request else { return false; };
    return true;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Secret size for the [`Authenticator::Secret`] authentication type.
pub const SECRET_AUTH_BYTES: usize = 16;

//-------------------------------------------------------------------------------------------------------------------

/// Client id authenticated by auth key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken
{
    client_id: u128,  //todo: derive from client key
    //todo
    //- prefix = sign[auth key](expiry, client key)
    //- verification = sign[client key](prefix)
    //todo: should sign entire connection http request?
}

//-------------------------------------------------------------------------------------------------------------------

/// Used by the [`Server`](crate::Server) to authenticate [`Client`](crate::Client) connections.
#[derive(Debug, Clone)]
pub enum Authenticator
{
    None,
    Secret
    {
        secret: [u8; SECRET_AUTH_BYTES]
    },
    Token
    {
        //todo
        //- auth pubkey
    },
}

//-------------------------------------------------------------------------------------------------------------------

/// [`Client`](crate::Client) authentication for connecting to a [`Server`](crate::Server).
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthRequest
{
    None
    {
        client_id: u128
    },
    /// **WARNING**
    ///
    /// Client connection requests paste the auth request into the request URL, which means secrets won't be encrypted.
    /// You should only use this auth type in controlled environments or if you are not concerned about secrets being
    /// intercepted.
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

/// Authenticates an auth request.
pub fn authenticate(request: &AuthRequest, authenticator: &Authenticator) -> bool
{
    match authenticator
    {
        Authenticator::None =>
        {
            return authenticate_none(request);
        }
        Authenticator::Secret{secret} =>
        {
            return authenticate_secret(request, secret);
        }
        Authenticator::Token{} =>
        {
            return authenticate_token(request);
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
