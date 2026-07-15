//! Perci core library.
//!
//! The runtime deliberately separates four concerns:
//! - language generation (`backend`),
//! - deterministic reasoning (`reasoning`),
//! - compact bit-level routing (`reflex`), and
//! - durable local memory (`memory`).
//!
//! This makes the system useful before a trained model is available and keeps
//! mathematical answers and governance decisions inspectable.

pub mod backend;
pub mod chat;
pub mod cognitive;
pub mod memory;
pub mod personality;
pub mod reasoning;
pub mod reflex;

pub use chat::{ChatEngine, ChatResponse};
pub use personality::Personality;
