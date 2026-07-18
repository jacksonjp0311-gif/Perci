//! Iterative reason / search / verify loop with receipts (v0.8.6).
//!
//! Flow:
//!   plan candidates → score (compositional + entity-slot) → verify → receipt
//!
//! Fail-closed: low scores produce unresolved receipts, never silent promote.

use crate::compositional_world::CompositionalWorld;
use crate::entity_slot;
use crate::native_decoder;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReasonReceipt {
    pub schema: String,
    pub status: ReasonStatus,
    pub user: String,
    pub steps: Vec<ReasonStep>,
    pub best_score: i64,
    pub answer: String,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasonStatus {
    Verified,
    BestEffort,
    Unresolved,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReasonStep {
    pub name: String,
    pub detail: String,
    pub score: i64,
    pub ok: bool,
}

/// Run a bounded reason/search/verify loop (max 4 candidates).
pub fn run_loop(user: &str) -> ReasonReceipt {
    let mut steps = Vec::new();
    let world = CompositionalWorld::seed();

    // Candidate 1: entity-slot operator
    let mut candidates: Vec<(String, String, i64)> = Vec::new();
    if entity_slot::looks_entity_slot_transfer(user) {
        let d = entity_slot::entity_slot_transfer_answer(user);
        let s = world.score_speech(user, &d.answer);
        candidates.push(("entity-slot".into(), d.answer, s));
        steps.push(ReasonStep {
            name: "search.entity_slot".into(),
            detail: "typed slot transfer candidate".into(),
            score: s,
            ok: s >= 20,
        });
    }

    // Candidate 2: native decoder
    let decoded = native_decoder::decode(user, None);
    let s2 = world.score_speech(user, &decoded.text);
    candidates.push(("native-decoder".into(), decoded.text.clone(), s2));
    steps.push(ReasonStep {
        name: "search.native_decoder".into(),
        detail: format!("layers={:?}", decoded.layers),
        score: s2,
        ok: s2 >= 12,
    });

    // Candidate 3: compositional prose only
    let motifs: Vec<String> = {
        if let Some(f) = entity_slot::extract_entity_slot_frame(user) {
            vec![f.slot_a, f.slot_b]
        } else {
            let lower = user.to_ascii_lowercase();
            ["trust", "evidence", "boundary", "identity", "repair", "memory"]
                .iter()
                .filter(|m| lower.contains(**m))
                .map(|m| (*m).to_owned())
                .collect()
        }
    };
    if motifs.len() >= 2 {
        let explain = world.explain_pair(&motifs[0], &motifs[1]);
        let s3 = world.score_speech(user, &explain);
        candidates.push(("compositional-only".into(), explain, s3));
        steps.push(ReasonStep {
            name: "search.compositional".into(),
            detail: format!("pair {}–{}", motifs[0], motifs[1]),
            score: s3,
            ok: s3 >= 8,
        });
    }

    // Verify: pick best score
    candidates.sort_by(|a, b| b.2.cmp(&a.2));
    let (name, answer, best) = candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            (
                "empty".into(),
                "Unresolved: no candidate produced checkable structure.".into(),
                0,
            )
        });

    steps.push(ReasonStep {
        name: "verify.select".into(),
        detail: format!("selected={name}"),
        score: best,
        ok: best >= 12,
    });

    // Verify slot binding when applicable
    let mut notes = Vec::new();
    if let Some(f) = entity_slot::extract_entity_slot_frame(user) {
        let bound = entity_slot::slots_bound_in_speech(&answer, &f.slot_a, &f.slot_b);
        steps.push(ReasonStep {
            name: "verify.slot_pair".into(),
            detail: format!("{} & {}", f.slot_a, f.slot_b),
            score: if bound { 1 } else { 0 },
            ok: bound,
        });
        if !bound {
            notes.push("slot_pair_binding failed".into());
        }
    }

    let status = if best >= 24 && notes.is_empty() {
        ReasonStatus::Verified
    } else if best >= 12 {
        ReasonStatus::BestEffort
    } else {
        ReasonStatus::Unresolved
    };

    notes.push("no weight promote from reason loop".into());

    ReasonReceipt {
        schema: "perci.reason-receipt.v1".into(),
        status,
        user: user.chars().take(200).collect(),
        steps,
        best_score: best,
        answer,
        notes,
    }
}

pub fn format_receipt(r: &ReasonReceipt) -> String {
    let mut out = format!(
        "[Reason loop · {:?} · score={}]\n",
        r.status, r.best_score
    );
    for s in &r.steps {
        out.push_str(&format!(
            "  {} {} score={} · {}\n",
            if s.ok { "✓" } else { "·" },
            s.name,
            s.score,
            s.detail
        ));
    }
    out.push_str("---\n");
    out.push_str(&r.answer);
    if !r.notes.is_empty() {
        out.push_str(&format!("\n\nnotes: {}", r.notes.join("; ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_loop_verifies_entity_slot() {
        let r = run_loop(
            "An unfamiliar device called Quoril-7 has trust and change. Transfer one relation to it without treating the invented name as evidence.",
        );
        assert!(matches!(
            r.status,
            ReasonStatus::Verified | ReasonStatus::BestEffort
        ));
        assert!(r.answer.to_ascii_lowercase().contains("trust"));
        assert!(r.steps.iter().any(|s| s.name.contains("verify")));
    }
}
