#![forbid(unsafe_code)]
pub mod adapter;
pub mod config;
pub mod credentials;
pub mod definitions;
pub use adapter::GitHub;
pub use config::Config;
