//local shortcuts

//third-party shortcuts

//standard shortcuts
use core::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

/// Errors emitted by the internal connection handler.
#[derive(Debug, Clone)]
pub enum ConnectionError
{
    SystemError,
}

impl std::fmt::Display for ConnectionError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let _ = write!(f, "ConnectionError::");
        match self
        {
            ConnectionError::SystemError => write!(f, "SystemError"),
        }
    }
}
impl std::error::Error for ConnectionError {}

//-------------------------------------------------------------------------------------------------------------------
