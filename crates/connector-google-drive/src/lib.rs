#![forbid(unsafe_code)]
mod adapter;
mod config;
mod credentials;
pub mod definitions;
pub mod managed;
pub(crate) mod native_scope;
pub mod oauth;
pub use adapter::Drive;
pub use config::Config;
