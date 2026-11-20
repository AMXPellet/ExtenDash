//! ExtenDash core: UI-agnostic business logic shared by every frontend.
//!
//! Milestone 1 (Foundations) provides the v1 data model and the workspace
//! folder layout. The actor, command/query/event API, merge engine and SQLite
//! cache arrive in Milestone 2.

pub mod error;
pub mod layout;
pub mod schema;

pub use error::{CoreError, Result};
pub use schema::SCHEMA_VERSION;
