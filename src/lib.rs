//documentation
#![doc = include_str!("../README.md")]

//features
#![allow(incomplete_features)]
#![feature(inherent_associated_types)]

//module tree
mod authentication;
mod client;
mod client_handler;
mod common;
mod connection_handler;
mod errors;
mod rate_limiter;
mod server;
mod session_handler;

//API exports
pub use crate::authentication::*;
pub use crate::client::*;
pub(crate) use crate::client_handler::*;
pub use crate::common::*;
pub(crate) use crate::connection_handler::*;
pub(crate) use crate::errors::*;
pub use crate::rate_limiter::*;
pub use crate::server::*;
pub(crate) use crate::session_handler::*;
