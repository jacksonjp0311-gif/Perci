//! Perci core library.
//!
//! The runtime separates:
//! - explicit command routing,
//! - 4,096-bit associative cognition (Bitwork routing / geometry),
//! - deterministic exact tools,
//! - append-only JSONL memory,
//! - Cortex selective retrieval,
//! - Capability Fabric (language / knowledge / proof / code sidecars under governor),
//! - governance and authority boundaries.
//!
//! **Design law (v0.7.0):** do not stretch Bitwork to impersonate every capability.
//! Perci orchestrates; specialized engines perform specialized work.

pub mod agent;
pub mod auto_repairs;
pub mod backend;
pub mod branding;
pub mod bridge;
pub mod chat;
pub mod cognition_expand;
pub mod cognitive;
pub mod cortex;
pub mod daemon;
pub mod decision_trace;
pub mod deliberation;
pub mod emergence;
pub mod fabric;
pub mod intel_packs;
pub mod learning;
pub mod memory;
pub mod operator_program;
pub mod personality;
pub mod reason;
pub mod reasoning;
pub mod reflex;
pub mod semantic_eval;
pub mod session;
pub mod voice;

pub use chat::{ChatEngine, ChatResponse};
pub use personality::Personality;
