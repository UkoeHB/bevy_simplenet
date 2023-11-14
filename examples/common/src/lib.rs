//local shortcuts

//third-party shortcuts
use serde::{Deserialize, Serialize};

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DemoServerMsg
{
    /// Current owner.
    Current(Option<u128>),
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DemoClientRequest
{
    /// Select the button.
    ///
    /// Will be acked by the server.
    Select,
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DemoChannel;
impl bevy_simplenet::ChannelPack for DemoChannel
{
    type ConnectMsg = ();
    type ClientMsg = ();
    type ClientRequest = DemoClientRequest;
    type ServerMsg = DemoServerMsg;
    type ServerResponse = ();
}

//-------------------------------------------------------------------------------------------------------------------
