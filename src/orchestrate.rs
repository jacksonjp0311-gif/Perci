//! Capability Fabric orchestrator — single entry for multi-engine answers.
//!
//! Flow:
//!   plan → exact/proof → operators (caller) → knowledge → native language → critic
//!
//! Perci remains governor of acceptance.

use crate::fabric::{self, FabricEngine, FabricPlan};
use crate::knowledge_fabric;
use crate::language_sidecar;
use crate::proof_engine;

/// Enrich an operator/tool seed body with knowledge + governed language.
pub fn enrich_answer(user: &str, operator: &str, seed_body: &str) -> String {
    let plan = fabric::plan_for_prompt(user, "orchestrate");
    let mut body = seed_body.to_owned();

    // Knowledge fabric when plan says facts needed, or always light pack context for long asks.
    if plan.needs_external_facts || plan.engines.contains(&FabricEngine::KnowledgeFabric) {
        let evidence = knowledge_fabric::retrieve_evidence(user, 4);
        if !evidence.is_empty() {
            let mut block = String::from("\n\nSource-bearing context:\n");
            for e in evidence.iter().take(3) {
                block.push_str(&format!(
                    "- ({}) {}\n",
                    e.source_type,
                    e.claim.chars().take(160).collect::<String>()
                ));
            }
            let contra = knowledge_fabric::find_contradictions(&evidence);
            if !contra.is_empty() {
                block.push_str("Note: possible contradictions among sources; prefer higher authority.\n");
            }
            body.push_str(&block);
        }
    }

    // Language sidecar under critic when plan includes it or user asks for explanation fluency.
    if plan.language.is_some() || language_sidecar::should_invoke_language(user) {
        let lower = user.to_ascii_lowercase();
        let wants_provenance = ["evidence", "source", "provenance", "according to", "show support"]
            .iter()
            .any(|term| lower.contains(term));
        let evidence = if plan.needs_external_facts || wants_provenance {
            knowledge_fabric::retrieve_evidence(user, 3)
        } else {
            Vec::new()
        };
        let req = language_sidecar::request_from(user, operator, evidence);
        let resp = language_sidecar::generate(&req, &body);
        if resp.ok {
            body = resp.text;
        }
    }

    body
}

/// Proof/exact path before free chat.
pub fn try_proof_or_exact(user: &str) -> Option<String> {
    proof_engine::try_prove_or_compute(user).map(|r| proof_engine::format_receipt(&r))
}

/// Public plan snapshot for AI agents entering the repo.
pub fn plan_json(user: &str) -> String {
    let plan: FabricPlan = fabric::plan_for_prompt(user, "ai-entry");
    serde_json::to_string_pretty(&plan).unwrap_or_else(|_| "{}".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrich_keeps_seed() {
        let out = enrich_answer(
            "explain trust under lag briefly",
            "trust-systems",
            "Retries must be idempotent under lag.",
        );
        assert!(out.to_ascii_lowercase().contains("idempotent"));
    }
}
