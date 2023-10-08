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
