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
mod pending_result;
mod pending_result_defaults;
mod rate_limiter;
mod result_receiver;
mod runtime;
mod runtime_defaults;
mod server;
mod session_handler;

#[cfg(not(wasm))]
mod runtime_impl_native;

#[cfg(wasm)]
mod runtime_impl_wasm;

//API exports
pub use crate::authentication::*;
pub use crate::client::*;
pub(crate) use crate::client_handler::*;
pub use crate::common::*;
pub(crate) use crate::connection_handler::*;
pub(crate) use crate::errors::*;
pub use crate::pending_result::*;
pub use crate::pending_result_defaults::*;
pub use crate::rate_limiter::*;
pub use crate::result_receiver::*;
pub use crate::runtime::*;
pub use crate::runtime_defaults::*;
pub use crate::server::*;
pub(crate) use crate::session_handler::*;

#[cfg(not(wasm))]
pub use crate::runtime_impl_native::*;

#[cfg(wasm)]
pub use crate::runtime_impl_wasm::*;
