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
pub mod binary_dialogue;
pub mod binary_language;
pub mod binary_phrase;
pub mod binary_relation;
pub mod binary_state;
pub mod binary_world;
pub mod branding;
pub mod bridge;
pub mod capability_router;
pub mod chat;
pub mod cognition_expand;
pub mod cognitive;
pub mod compositional_world;
pub mod context_card;
pub mod cortex;
pub mod daemon;
pub mod decision_trace;
pub mod deliberation;
pub mod dialogue_workspace;
pub mod discourse_plan;
pub mod emergence;
pub mod entity_slot;
pub mod fabric;
pub mod field_fold;
pub mod frontier_speech;
pub mod governed_will;
pub mod hydra_inject;
pub mod intel_packs;
pub mod knowledge_fabric;
pub mod language_realize;
pub mod language_sidecar;
pub mod learning;
pub mod low_bit;
pub mod memory;
pub mod native_decoder;
pub mod operator_program;
pub mod orchestrate;
pub mod pack_manifest;
pub mod personality;
pub mod proof_engine;
pub mod reason;
pub mod reason_loop;
pub mod reason_transition;
pub mod reasoning;
pub mod reasoning_controller;
pub mod reflex;
pub mod replay_learn;
pub mod semantic_eval;
pub mod semantic_field;
pub mod session;
pub mod text_normalize;
pub mod thought_plan;
pub mod voice;

pub use chat::{ChatEngine, ChatResponse};
pub use personality::Personality;
