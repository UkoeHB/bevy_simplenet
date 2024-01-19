//features
#![cfg_attr(docsrs, feature(doc_cfg))]
#![feature(hash_extract_if)]

//documentation
#![doc = include_str!("../README.md")]
#[allow(unused_imports)]
use crate as bevy_simplenet;

//module tree
mod authentication;
mod common;
mod common_internal;
mod rate_limiter;
mod text_ping_pong;

#[cfg(feature = "client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
mod client;

#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
mod server;

//API exports
pub use crate::authentication::*;
pub use crate::common::*;
pub(crate) use crate::common_internal::*;
pub use crate::rate_limiter::*;
pub(crate) use crate::text_ping_pong::*;

#[cfg(feature = "client")]
pub use crate::client::*;

#[cfg(feature = "server")]
pub use crate::server::*;
