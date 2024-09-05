//documentation
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![allow(rustdoc::redundant_explicit_links)]
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
mod client;

#[cfg(feature = "server")]
mod server;

//API exports
pub use authentication::*;
pub use common::*;
pub(crate) use common_internal::*;
pub use rate_limiter::*;
pub(crate) use text_ping_pong::*;

#[cfg(feature = "client")]
pub use client::*;

#[cfg(feature = "server")]
pub use server::*;
