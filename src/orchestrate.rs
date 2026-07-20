//! Capability Fabric orchestrator — single entry for multi-engine answers.
//!
//! Flow:
//!   plan → exact/proof → operators (caller) → knowledge → native language → critic
//!
//! Perci remains governor of acceptance.

use crate::fabric::{self, FabricPlan};
use crate::knowledge_fabric;
use crate::language_sidecar;
use crate::proof_engine;

/// Enrich an operator/tool seed body with knowledge + governed language.
pub fn enrich_answer(user: &str, operator: &str, seed_body: &str) -> String {
    let plan = fabric::plan_for_prompt(user, "orchestrate");
    let mut body = seed_body.to_owned();
    let lower = user.to_ascii_lowercase();
    let explicit_evidence_request = [
        "evidence",
        "source",
        "provenance",
        "according to",
        "show support",
        "what justifies",
        "falsif",
    ]
    .iter()
    .any(|term| lower.contains(term));
    let world_fact_request = [
        "latest",
        "news",
        "documentation for",
        "rfc ",
        "when did",
        "who is",
    ]
    .iter()
    .any(|term| lower.contains(term));

    // Retrieval may still inform the internal language sidecar for a
    // cross-domain answer, but source blocks belong in the human-facing reply
    // only when the user asked for evidence or current/world facts.  Otherwise
    // the answer reads like a conversation instead of a debug receipt.
    if explicit_evidence_request || world_fact_request {
        let evidence = evidence_for_prompt(user, 4);
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
                block.push_str(
                    "Note: possible contradictions among sources; prefer higher authority.\n",
                );
            }
            body.push_str(&block);
        }
    }

    // Language sidecar under critic when plan includes it or user asks for explanation fluency.
    // A specialized operator owns its answer shape. The language sidecar may
    // polish open-ended prose, but it must not replace a measured separation
    // (especially learning/evidence) with a generic continuation.
    // Prefer fluency rewrite for operator speech so chat sounds collaborative,
    // not like a checklist dump. Keep ownership for evidence, multi-hop plans
    // (hardness requires Goal/Steps structure), and exact-tool style operators.
    let operator_owns_structure = matches!(
        operator,
        "learning-evidence"
            | "multi-hop-plan"
            | "math-explanation"
            | "causal-chain"
            | "hallucination-refusal"
            | "consciousness-claim-refusal"
            | "out-of-distribution-abstention"
            | "metaphysical-claim-abstention"
            | "session-situation"
            | "dialogue-workspace"
            // Keep human/authorize/refuse tokens; checklist strip used to drop them.
            | "governance-authority"
    );
    let want_fluency = !operator_owns_structure
        && (plan.language.is_some()
            || language_sidecar::should_invoke_language(user)
            || body.len() > 60
            || body.contains("**")
            || body.contains('\n'));
    if want_fluency {
        let wants_provenance = [
            "evidence",
            "source",
            "provenance",
            "according to",
            "show support",
        ]
        .iter()
        .any(|term| lower.contains(term));
        let evidence = if world_fact_request || wants_provenance {
            evidence_for_prompt(user, 3)
        } else {
            Vec::new()
        };
        let req = language_sidecar::request_from(user, operator, evidence);
        let resp = language_sidecar::generate(&req, &body);
        if resp.ok {
            body = resp.text;
        } else {
            // Even if critic rejects expansion, still demote checklist formatting.
            body = language_sidecar::fluent_rewrite(user, &body);
        }
    }

    body
}

/// Retrieve a small evidence bundle that gives each requested domain a chance
/// to contribute. A single mixed query often lets the strongest lexical domain
/// drown out the others; per-domain probes make coverage visible and bounded.
fn evidence_for_prompt(user: &str, limit: usize) -> Vec<crate::fabric::EvidenceRecord> {
    let Some(summary) = crate::deliberation::cross_domain_summary(user) else {
        return knowledge_fabric::retrieve_evidence(user, limit);
    };

    let axis = summary.shared_axis.as_deref().unwrap_or("structure");
    let per_domain = (limit / summary.terms.len().max(1)).max(1);
    let mut evidence = Vec::new();
    for term in &summary.terms {
        evidence.extend(knowledge_fabric::retrieve_evidence(
            &format!("{term} {axis}"),
            per_domain,
        ));
    }
    // Keep a mixed query as a fallback for pack cards that describe the bridge
    // rather than one domain. Deduplication preserves the bounded limit.
    evidence.extend(knowledge_fabric::retrieve_evidence(user, limit));
    evidence.sort_by(|left, right| {
        right
            .authority
            .partial_cmp(&left.authority)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .freshness
                    .partial_cmp(&left.freshness)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    evidence.dedup_by(|left, right| left.claim_id == right.claim_id);
    if let Some(banned) = banned_word(user) {
        evidence.retain(|item| {
            let blob = format!("{} {} {}", item.claim, item.source, item.supports.join(" "))
                .to_ascii_lowercase();
            !blob.contains(&banned)
        });
    }
    evidence.truncate(limit);
    evidence
}

fn banned_word(user: &str) -> Option<String> {
    let lower = user.to_ascii_lowercase();
    let marker = "without using the word";
    let start = lower.find(marker)? + marker.len();
    lower[start..]
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .find(|word| !word.is_empty())
        .map(str::to_owned)
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

    #[test]
    fn learning_evidence_operator_keeps_specific_answer_shape() {
        let out = enrich_answer(
            "What evidence supports the claim that Perci is learning?",
            "learning-evidence",
            "The smallest separating test is a fresh-process A/B run with unseen variants.",
        );
        assert!(out.contains("fresh-process A/B"));
        assert!(out.contains("unseen variants"));
    }

    #[test]
    fn evidence_map_respects_banned_word_requests() {
        let out = enrich_answer(
            "Connect markets, ecosystems, and immune systems without using the word boundary.",
            "cross-domain-synthesis",
            "Markets, ecosystems, and immune systems share selection under constraints.",
        );
        assert!(!out.to_ascii_lowercase().contains("boundary"));
        assert!(out.to_ascii_lowercase().contains("markets"));
    }

    #[test]
    fn conversational_cross_domain_answer_hides_receipt_until_requested() {
        let out = enrich_answer(
            "Connect geometry, language, life, and death in one coherent idea.",
            "cross-domain-synthesis",
            "A boundary can make structure and exchange visible.",
        );
        assert!(out.contains("geometry") || out.contains("boundary"));
        assert!(!out.contains("Source-bearing context:"));
        assert!(!out.contains("Evidence (source-bearing):"));
    }

    #[test]
    fn explicit_evidence_request_keeps_provenance_visible() {
        let out = enrich_answer(
            "What evidence supports the claim that a boundary enables exchange?",
            "evidence-design",
            "A selective interface admits some flows and rejects others.",
        );
        assert!(
            out.contains("Source-bearing context:") || out.contains("Evidence (source-bearing):")
        );
    }
}
