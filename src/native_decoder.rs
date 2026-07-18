//! Native generative semantic decoder beyond pure phrase transitions (v0.8.6).
//!
//! Layers:
//! 1. Operator / entity-slot seed (if any)
//! 2. Compositional multi-hop chain prose
//! 3. Optional PERCPHR1 phrase continuation as *tail* only
//! 4. Critic boundaries (no consciousness / auto-promote claims)
//!
//! This is not an LLM. It composes typed structure then optionally smooths.

use crate::binary_phrase::BinaryPhraseModel;
use crate::compositional_world::{self, CompositionalWorld};
use crate::entity_slot;
use crate::fabric::critic_accept_language;

#[derive(Clone, Debug)]
pub struct DecodeReceipt {
    pub schema: String,
    pub engine: String,
    pub layers: Vec<String>,
    pub text: String,
    pub ok: bool,
    pub notes: Vec<String>,
}

/// Decode a native answer under structure-first law.
pub fn decode(user: &str, seed: Option<&str>) -> DecodeReceipt {
    let mut layers = Vec::new();
    let mut notes = Vec::new();
    let mut body = String::new();

    // Layer A: entity-slot frame
    if entity_slot::looks_entity_slot_transfer(user) {
        let d = entity_slot::entity_slot_transfer_answer(user);
        body = d.answer;
        layers.push("entity-slot".into());
    } else if let Some(seed) = seed {
        if !seed.trim().is_empty() {
            body = seed.trim().to_owned();
            layers.push("seed".into());
        }
    }

    // Layer B: compositional multi-hop enrichment
    let world = CompositionalWorld::seed();
    let motifs = extract_motifs(user);
    if motifs.len() >= 2 {
        let explain = world.explain_pair(&motifs[0], &motifs[1]);
        let paths = world.paths(&motifs[0], &motifs[1], 2);
        if !paths.is_empty() {
            let chain = compositional_world::compose_chain_prose(&paths[0]);
            if body.is_empty() {
                body = format!(
                    "Native compositional decode:\n{explain}\n\nChain: {chain}\n\
Observation: perturb {} and predict a change in {}; fail closed if the link does not hold.",
                    motifs[0], motifs[1]
                );
                layers.push("compositional-primary".into());
            } else if !body.to_ascii_lowercase().contains("hop")
                && !body.to_ascii_lowercase().contains("→")
            {
                body.push_str(&format!("\n\nCompositional support: {chain}"));
                layers.push("compositional-enrich".into());
            }
        } else if body.is_empty() {
            body = format!(
                "Native decode for {} and {}: no seeded multi-hop path; state a testable hypothesis and a falsifier rather than inventing a bridge.",
                motifs[0], motifs[1]
            );
            layers.push("compositional-abstain".into());
            notes.push("no multi-hop path".into());
        }
    }

    // Layer C: optional phrase tail (never sole authority)
    match BinaryPhraseModel::discover() {
        Ok(Some(model)) if body.chars().count() < 280 => {
            let tail = model.generate_reply(user, "general", 180, 1);
            let tail = tail.trim();
            if tail.len() > 12 && !tail.eq_ignore_ascii_case(user) {
                body.push_str("\n\n");
                body.push_str(tail);
                layers.push("phrase-tail".into());
            }
        }
        Ok(None) => notes.push("PERCPHR1 absent — structure-only decode".into()),
        Err(e) => notes.push(format!("PERCPHR1 load: {e}")),
        _ => {}
    }

    if body.is_empty() {
        body = "Native decoder: insufficient structure to compose; ask for two motifs or a relation to transfer.".into();
        layers.push("empty-fallback".into());
        notes.push("empty".into());
    }

    // Critic
    let mut ok = true;
    if let Err(e) = critic_accept_language(&body, &["no consciousness claims".into(), "no weight auto-promote".into()]) {
        ok = false;
        notes.push(e);
        body.push_str("\n\n[Governor] Decode refused boundary violation.");
    }

    DecodeReceipt {
        schema: "perci.native-decode.v1".into(),
        engine: "native-decoder".into(),
        layers,
        text: body,
        ok,
        notes,
    }
}

fn extract_motifs(user: &str) -> Vec<String> {
    if let Some(f) = entity_slot::extract_entity_slot_frame(user) {
        return vec![f.slot_a, f.slot_b];
    }
    let lower = user.to_ascii_lowercase();
    const M: &[&str] = &[
        "boundary", "memory", "evidence", "repair", "trust", "uncertainty", "scale",
        "identity", "signal", "learning", "entropy", "structure", "attention", "change",
    ];
    M.iter()
        .filter(|m| lower.contains(**m))
        .map(|m| (*m).to_owned())
        .collect()
}

pub fn status_report() -> String {
    "[Native decoder · structure-first]\n\
layers: entity-slot → compositional multi-hop → optional phrase tail → critic\n\
law: phrase transitions never own truth; typed structure does\n"
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_entity_slot_binds_motifs() {
        let r = decode(
            "An unfamiliar device called Quoril-7 has trust and change. Transfer one relation to it without treating the invented name as evidence.",
            None,
        );
        assert!(r.ok);
        assert!(r.layers.iter().any(|l| l.contains("entity-slot")));
        let low = r.text.to_ascii_lowercase();
        assert!(low.contains("trust"));
        assert!(low.contains("change"));
    }

    #[test]
    fn decode_compositional_when_two_motifs() {
        let r = decode(
            "How does trust relate to evidence in a bounded system?",
            Some("Trust tracks evidence quality."),
        );
        assert!(r.ok);
        assert!(!r.text.is_empty());
    }
}
