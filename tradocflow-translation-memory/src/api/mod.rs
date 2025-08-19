//! API module for REST server components
//!
//! This module contains authentication, middleware, and API models
//! used by the REST server binary.

pub mod auth;
pub mod handlers;
pub mod middleware_auth;
pub mod models;

// Re-export commonly used types for easier access
pub use models::*;
pub use middleware_auth::{AppState, auth_middleware};