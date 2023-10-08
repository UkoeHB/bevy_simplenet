//documentation
#![doc = include_str!("../README.md")]

//features
#![allow(incomplete_features)]
#![feature(inherent_associated_types)]

//common
cfg_if::cfg_if! { if #[cfg(any(feature = "client", feature = "server"))] {
    mod authentication;
    mod common;
    mod rate_limiter;
    mod text_ping_pong;

    pub use crate::authentication::*;
    pub use crate::common::*;
    pub use crate::rate_limiter::*;
    pub(crate) use crate::text_ping_pong::*;
}}

//client
cfg_if::cfg_if! { if #[cfg(feature = "client")] {
    mod client;
    mod client_handler;

    pub use crate::client::*;
    pub(crate) use crate::client_handler::*;
}}

//server
cfg_if::cfg_if! { if #[cfg(feature = "server")] {
    mod connection_handler;
    mod connection_validation;
    mod server;
    mod session_handler;

    pub(crate) use crate::connection_handler::*;
    pub(crate) use crate::connection_validation::*;
    pub use crate::server::*;
    pub(crate) use crate::session_handler::*;
}}
