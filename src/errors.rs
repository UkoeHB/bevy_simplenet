//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Errors emitted by `SessionHandler`
#[derive(Debug, Clone)]
pub(crate) enum SessionError
{
    ClosedByServer,
}

impl std::fmt::Display for SessionError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let _ = write!(f, "SessionError::");
        match self
        {
            SessionError::ClosedByServer => write!(f, "ClosedByServer"),
        }
    }
}
impl std::error::Error for SessionError {}

//-------------------------------------------------------------------------------------------------------------------

/// Errors emitted by `ConnectionHandler`
#[derive(Debug, Clone)]
pub(crate) enum ConnectionError
{
    SerializationError,
    SystemError,
}

impl std::fmt::Display for ConnectionError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let _ = write!(f, "ConnectionError::");
        match self
        {
            ConnectionError::SerializationError => write!(f, "SerializationError"),
            ConnectionError::SystemError        => write!(f, "SystemError"),
        }
    }
}
impl std::error::Error for ConnectionError {}

//-------------------------------------------------------------------------------------------------------------------

/// Errors emitted by `ClientHandler`
#[derive(Debug)]
pub(crate) enum ClientError
{
    //ClosedByServer,
    SendError
}

impl std::fmt::Display for ClientError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let _ = write!(f, "ClientError::");
        match self
        {
            //ClientError::ClosedByServer => write!(f, "ClosedByServer"),
            ClientError::SendError      => write!(f, "SendError"),
        }
    }
}
impl std::error::Error for ClientError {}

//-------------------------------------------------------------------------------------------------------------------
