//! Tokito library — HTTP router and domain entrypoints.
#![forbid(unsafe_code)]

pub mod auth;
pub mod config;
pub mod config_provider;
pub mod connectivity;
pub mod db;
pub mod paths;
pub mod project_toml;
pub mod settings;
pub mod secrets;
pub mod user_messages;
pub mod error;
pub mod handlers;
pub mod models;
pub mod router;
pub mod server;
pub mod services;
pub mod store;

#[cfg(feature = "test-support")]
pub mod test_support;
