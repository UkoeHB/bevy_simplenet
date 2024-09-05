//local shortcuts
use crate::*;

//third-party shortcuts
use ed25519_dalek::{Signer, Signature, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use wasm_timer::{SystemTime, UNIX_EPOCH};

//standard shortcuts
use core::fmt::Debug;
use std::{io::{self, Cursor}, time::Duration};

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn format_date(time: Duration) -> String
{
    let secs = time.as_secs() % 60;
    let mins = (time.as_secs() / 60) % 60;
    let hrs = ((time.as_secs() / 60) / 60) % 24;
    let days = (((time.as_secs() / 60) / 60) / 24) % 365;
    let year = (((time.as_secs() / 60) / 60) / 24) / 365 + 1970;
    format!("{}:{}:{:0>2}:{:0>2}:{:0>2}", year, days, hrs, mins, secs)
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn write_auth_token_payload(
    writer: &mut impl io::Write,
    expiry: u64,
    client_id: u128,
) -> Result<(), io::Error>
{
    writer.write_all(AUTH_TOKEN_DOMAIN_SEPARATOR)?;
    writer.write_all(&AUTH_TOKEN_PROTOCOL_VERSION.to_le_bytes())?;
    writer.write_all(&expiry.to_le_bytes())?;
    writer.write_all(&client_id.to_le_bytes())?;
    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn auth_token_payload(expiry: u64, client_id: u128) -> [u8; AUTH_TOKEN_PAYLOAD_BYTES]
{
    let mut payload = [0u8; AUTH_TOKEN_PAYLOAD_BYTES];
    write_auth_token_payload(&mut Cursor::new(&mut payload[..]), expiry, client_id).expect("write should succeed");
    payload
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn authenticate_none(_request: &AuthRequest) -> bool
{
    // We allow any kind of auth request.
    true
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn authenticate_secret(expected_secret: &[u8; SECRET_AUTH_BYTES], request: &AuthRequest) -> bool
{
    let AuthRequest::Secret{client_id: _, secret} = request else { return false; };
    *secret == *expected_secret
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn authenticate_token(pubkey: &[u8; AUTH_PUBKEY_BYTES], request: &AuthRequest) -> bool
{
    let AuthRequest::Token{token} = request else { return false; };
    let AuthToken{ protocol_version, expiry, client_id, signature } = token;

    // Check token expiration.
    if token.is_expired() {
        tracing::debug!("failed verifying auth request {request:?}, token is expired \
            (current time: {:?}, expiration time: {:?}",
            format_date(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()),
            format_date(token.expiration_time()),
        );
    }

    // Pre-check the protocol version so it can be logged on mismatch.
    if *protocol_version != AUTH_TOKEN_PROTOCOL_VERSION
    {
        tracing::debug!("failed verifying auth request {request:?}, protocol version mismatch \
            (verifier version: {})", AUTH_TOKEN_PROTOCOL_VERSION);
        return false;
    }

    // Verify the signature.
    let verifier = match VerifyingKey::from_bytes(pubkey)
    {
        Ok(verifier) => verifier,
        Err(err) =>
        {
            tracing::error!("failed authenticating auth request {request:?}, verifier key is invalid: {err:?}");
            return false;
        }
    };

    let payload = auth_token_payload(*expiry, *client_id);
    let signature = Signature::from_bytes(signature);

    match verifier.verify(&payload, &signature)
    {
        Ok(()) => true,
        Err(err) =>
        {
            tracing::debug!("failed verifying auth request {request:?}, signature invalid: {err:?}");
            false
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// The domain separator used when signing auth tokens.
const AUTH_TOKEN_DOMAIN_SEPARATOR: &[u8; 22] = b"BevySimplenetAuthToken";

/// Payload: domain_sep | protocol_version | expiry_secs | client_id
const AUTH_TOKEN_PAYLOAD_BYTES: usize = 22 + 2 + 8 + 16;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// The current protocol version for [`AuthToken`] construction and validation.
pub const AUTH_TOKEN_PROTOCOL_VERSION: u16 = 0;

//-------------------------------------------------------------------------------------------------------------------

/// Byte length of [`AuthToken`] authentication privkeys.
pub const AUTH_PRIVKEY_BYTES: usize = 32;

/// Byte length of [`AuthToken`] authentication pubkeys.
///
/// See [`Authenticator::Token`].
pub const AUTH_PUBKEY_BYTES: usize = 32;

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
        pubkey: [u8; AUTH_PUBKEY_BYTES]
    },
}

impl Authenticator
{
    /// Authenticates an auth request.
    pub fn authenticate(&self, request: &AuthRequest) -> bool
    {
        match self
        {
            Authenticator::None =>
            {
                authenticate_none(request)
            }
            Authenticator::Secret{secret} =>
            {
                authenticate_secret(secret, request)
            }
            Authenticator::Token{pubkey} =>
            {
                authenticate_token(pubkey, request)
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Generates a privkey/pubkey pair for use in creating and verifying [`AuthTokens`](AuthToken).
///
/// The private key is a security-critical secret that should not be exposed to clients. For architectural
/// robustness it is recommended to only store the private key in servers that produce auth tokens, and use
/// the pubkey in servers that verify auth tokens.
pub fn generate_auth_token_keys() -> ([u8; AUTH_PRIVKEY_BYTES], [u8; AUTH_PUBKEY_BYTES])
{
    let mut csprng = OsRng;
    let privkey: SigningKey = SigningKey::generate(&mut csprng);
    let pubkey = privkey.verifying_key();

    (privkey.to_bytes(), pubkey.to_bytes())
}

//-------------------------------------------------------------------------------------------------------------------

/// Makes an [`AuthToken`] from a token lifetime.
///
/// The token will expire at `current time + lifetime`.
pub fn make_auth_token_from_lifetime(
    privkey: &[u8; AUTH_PRIVKEY_BYTES],
    token_lifetime_secs: u64,
    client_id: u128,
) -> AuthToken
{
    let expiry = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()
        + Duration::from_secs(token_lifetime_secs);
    make_auth_token_from_expiry(privkey, expiry.as_secs(), client_id)
}

//-------------------------------------------------------------------------------------------------------------------

/// Makes an [`AuthToken`] from an expiration time in seconds since `UNIX_EPOCH`.
pub fn make_auth_token_from_expiry(
    privkey: &[u8; AUTH_PRIVKEY_BYTES],
    expiry: u64,
    client_id: u128,
) -> AuthToken
{
    let payload = auth_token_payload(expiry, client_id);

    let signer: SigningKey = SigningKey::from_bytes(privkey);
    let signature = signer.sign(&payload);

    AuthToken{
        protocol_version: AUTH_TOKEN_PROTOCOL_VERSION,
        expiry,
        client_id,
        signature: signature.to_bytes(),
    }
}

//-------------------------------------------------------------------------------------------------------------------
