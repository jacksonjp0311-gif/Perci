//! Perci core library.
//!
//! The runtime separates:
//! - explicit command routing,
//! - 4,096-bit associative cognition,
//! - deterministic exact tools,
//! - append-only JSONL memory,
//! - Cortex selective retrieval,
//! - optional external language generation,
//! - governance and authority boundaries.

pub mod backend;
pub mod chat;
pub mod cognitive;
pub mod cortex;
pub mod memory;
pub mod personality;
pub mod reasoning;
pub mod reflex;

pub use chat::{ChatEngine, ChatResponse};
pub use personality::Personality;
