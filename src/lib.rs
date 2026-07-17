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

pub mod agent;
pub mod backend;
pub mod branding;
pub mod bridge;
pub mod chat;
pub mod cognitive;
pub mod cortex;
pub mod daemon;
pub mod decision_trace;
pub mod deliberation;
pub mod emergence;
pub mod cognition_expand;
pub mod auto_repairs;
pub mod intel_packs;
pub mod learning;
pub mod memory;
pub mod operator_program;
pub mod personality;
pub mod reason;
pub mod reasoning;
pub mod reflex;
pub mod session;
pub mod voice;

pub use chat::{ChatEngine, ChatResponse};
pub use personality::Personality;
