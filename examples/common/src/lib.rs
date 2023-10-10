// path shortcuts
use serde::{Deserialize, Serialize};

/// Server messages
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DemoServerMsg
{
    AckSelect,
    Deselect,
}

/// Client messages
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DemoClientMsg
{
    Select
}

/// Package of messages.
#[derive(Debug, Clone)]
pub struct DemoMsgPack;
impl bevy_simplenet::MsgPack for DemoMsgPack
{
    type ConnectMsg = ();
    type ClientMsg = DemoClientMsg;
    type ClientRequest = ();
    type ServerMsg = DemoServerMsg;
    type ServerResponse = ();
}
