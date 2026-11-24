//! maxima-rs webUI backend library.
//!
//! Exposes the request/response models, service functions, application router,
//! and server configuration so the HTTP binary, Lambda adapters, and end-to-end
//! tests share the exact same implementation.

pub mod api;
pub mod app;
pub mod config;
pub mod service;

pub use app::{AppState, StaticAssets, build_app};
pub use config::{Cli, ServerConfig};
