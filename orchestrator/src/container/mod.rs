//! Container management module for CAGE Orchestrator
//!
//! Handles all interactions with Podman/Docker containers including:
//! - Creating and starting containers
//! - Executing code inside containers
//! - Managing container lifecycle
//! - Resource monitoring

mod manager;
mod executor;
pub mod session;

pub use manager::ContainerManager;
pub use executor::CodeExecutor;
pub use session::{Session, SessionState, SessionHandle};
