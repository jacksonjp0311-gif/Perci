//! Compositional typed world model — multi-hop edges (v0.8.6).
//!
//! PERCIWM1 stores bag-style S–R–O edges. This layer builds a **motif graph**
//! over known curriculum slots and scores multi-hop paths:
//!
//!   A —R1→ B —R2→ C  ⇒  compositional support for A … C
//!
//! It never invents facts about invented surface names; entity names stay
//! role-fillers. No weight auto-promote.

use crate::entity_slot;
use std::collections::{BTreeMap, BTreeSet};

/// Canonical motifs used as graph nodes.
const MOTIFS: &[&str] = &[
    "boundary", "memory", "evidence", "repair", "trust", "uncertainty", "scale",
    "identity", "signal", "learning", "entropy", "structure", "attention", "change",
    "mechanism", "relation", "transfer", "invariant", "observation", "feedback",
];

/// Default typed edges (relation label between motif slots).
fn seed_edges() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("boundary", "maintains", "identity"),
        ("evidence", "guides", "repair"),
        ("trust", "requires", "evidence"),
        ("signal", "reduces", "uncertainty"),
        ("attention", "selects", "structure"),
        ("entropy", "degrades", "memory"),
        ("scale", "modulates", "learning"),
        ("trust", "survives", "change"),
        ("memory", "records", "boundary"),
        ("observation", "tests", "relation"),
        ("feedback", "updates", "learning"),
        ("mechanism", "explains", "relation"),
        ("evidence", "supports", "trust"),
        ("structure", "constrains", "attention"),
        ("repair", "restores", "boundary"),
        ("learning", "requires", "feedback"),
        ("uncertainty", "limits", "trust"),
        ("identity", "depends", "memory"),
        ("transfer", "preserves", "relation"),
        ("invariant", "survives", "change"),
    ]
}

#[derive(Clone, Debug, Default)]
pub struct CompositionalWorld {
    /// adjacency: from → list of (relation, to)
    adj: BTreeMap<String, Vec<(String, String)>>,
    /// reverse: to → list of (relation, from)
    rev: BTreeMap<String, Vec<(String, String)>>,
}

impl CompositionalWorld {
    pub fn seed() -> Self {
        let mut w = Self::default();
        for (a, r, b) in seed_edges() {
            w.add_edge(a, r, b);
        }
        w
    }

    pub fn add_edge(&mut self, from: &str, relation: &str, to: &str) {
        let f = from.to_ascii_lowercase();
        let t = to.to_ascii_lowercase();
        let r = relation.to_ascii_lowercase();
        self.adj
            .entry(f.clone())
            .or_default()
            .push((r.clone(), t.clone()));
        self.rev.entry(t).or_default().push((r, f));
    }

    pub fn edge_count(&self) -> usize {
        self.adj.values().map(|v| v.len()).sum()
    }

    pub fn node_count(&self) -> usize {
        let mut nodes = BTreeSet::new();
        for (k, vs) in &self.adj {
            nodes.insert(k.clone());
            for (_, t) in vs {
                nodes.insert(t.clone());
            }
        }
        nodes.len()
    }

    /// One-hop neighbors of a motif.
    pub fn neighbors(&self, node: &str) -> Vec<(String, String)> {
        self.adj
            .get(&node.to_ascii_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    /// Multi-hop paths of length 1 or 2 between two motifs.
    pub fn paths(&self, from: &str, to: &str, max_hops: u8) -> Vec<Vec<String>> {
        let start = from.to_ascii_lowercase();
        let goal = to.to_ascii_lowercase();
        let mut out = Vec::new();
        if start == goal {
            return out;
        }
        // hop-1
        if let Some(edges) = self.adj.get(&start) {
            for (rel, nxt) in edges {
                if *nxt == goal {
                    out.push(vec![start.clone(), rel.clone(), goal.clone()]);
                }
            }
        }
        if max_hops < 2 {
            return out;
        }
        // hop-2
        if let Some(edges) = self.adj.get(&start) {
            for (rel1, mid) in edges {
                if let Some(edges2) = self.adj.get(mid) {
                    for (rel2, nxt) in edges2 {
                        if *nxt == goal {
                            out.push(vec![
                                start.clone(),
                                rel1.clone(),
                                mid.clone(),
                                rel2.clone(),
                                goal.clone(),
                            ]);
                        }
                    }
                }
            }
        }
        out
    }

    /// Score how well a response preserves multi-hop structure for a prompt.
    pub fn score_speech(&self, user: &str, response: &str) -> i64 {
        let lower_u = user.to_ascii_lowercase();
        let lower_r = response.to_ascii_lowercase();
        let slots = present_motifs(&lower_u);
        if slots.len() < 2 {
            // still reward if response carries multi-hop vocabulary from seed
            return if lower_r.contains("hop") || lower_r.contains("compose") {
                2
            } else {
                0
            };
        }
        let mut score = 0i64;
        // Direct edge presence
        for i in 0..slots.len() {
            for j in (i + 1)..slots.len() {
                let paths = self.paths(&slots[i], &slots[j], 2);
                if paths.is_empty() {
                    continue;
                }
                // Both slots must appear in speech for credit (relation transfer law).
                if !(lower_r.contains(&slots[i]) && lower_r.contains(&slots[j])) {
                    continue;
                }
                score += 12; // both slots bound
                for path in &paths {
                    // Reward relation labels that appear
                    for token in path {
                        if token.len() >= 4 && lower_r.contains(token) {
                            score += 4;
                        }
                    }
                    if path.len() >= 5 {
                        score += 8; // multi-hop bonus
                    }
                }
            }
        }
        // Entity-slot prompts: never reward invented name as evidence alone
        if entity_slot::looks_entity_slot_transfer(user) {
            if let Some(frame) = entity_slot::extract_entity_slot_frame(user) {
                if entity_slot::slots_bound_in_speech(response, &frame.slot_a, &frame.slot_b) {
                    score += 20;
                }
            }
        }
        score.min(255)
    }

    /// Human-readable compositional explanation for two slots.
    pub fn explain_pair(&self, a: &str, b: &str) -> String {
        let paths = self.paths(a, b, 2);
        if paths.is_empty() {
            // try reverse
            let rev = self.paths(b, a, 2);
            if rev.is_empty() {
                return format!(
                    "No seeded multi-hop path between {a} and {b}; treat the link as a hypothesis to test."
                );
            }
            return format_paths(&rev);
        }
        format_paths(&paths)
    }

    pub fn status_report() -> String {
        let w = Self::seed();
        format!(
            "[Compositional world · multi-hop]\n\
nodes={} edges={} (seed motif graph)\n\
law: surface entity names are role-fillers; hops compose typed relations\n\
promote: never auto — candidate scores only\n",
            w.node_count(),
            w.edge_count()
        )
    }
}

fn present_motifs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for m in MOTIFS {
        if text.contains(m) {
            out.push((*m).to_owned());
        }
    }
    out
}

fn format_paths(paths: &[Vec<String>]) -> String {
    let mut lines = Vec::new();
    for p in paths.iter().take(3) {
        lines.push(p.join(" → "));
    }
    format!("Compositional paths:\n  · {}", lines.join("\n  · "))
}

/// Decode a multi-hop chain into native prose (feeds generative decoder).
pub fn compose_chain_prose(path: &[String]) -> String {
    if path.len() == 3 {
        return format!(
            "{} {} {} under a checkable intermediate; verify by holding other factors fixed.",
            path[0], path[1], path[2]
        );
    }
    if path.len() >= 5 {
        return format!(
            "{} {} {} then {} {} — a two-hop composition. The surface name of a device does not add evidence; only the chain of checkable links does.",
            path[0], path[1], path[2], path[3], path[4]
        );
    }
    path.join(" → ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_hop_trust_to_repair() {
        let w = CompositionalWorld::seed();
        // trust → evidence → repair (2 hops)
        let paths = w.paths("trust", "repair", 2);
        assert!(
            !paths.is_empty(),
            "expected multi-hop trust→…→repair, got {paths:?}"
        );
        assert!(paths.iter().any(|p| p.len() >= 5));
    }

    #[test]
    fn score_rewards_slot_pair_in_speech() {
        let w = CompositionalWorld::seed();
        let user = "An unfamiliar device called Quoril-7 has trust and evidence. Transfer one relation to it without treating the invented name as evidence.";
        let good = "Slots trust and evidence: trust requires evidence; the name Quoril-7 is not data.";
        let bad = "A switchyard of sparse tracks routes trains.";
        assert!(w.score_speech(user, good) > w.score_speech(user, bad));
    }

    #[test]
    fn explain_boundary_identity() {
        let w = CompositionalWorld::seed();
        let e = w.explain_pair("boundary", "identity");
        assert!(e.to_ascii_lowercase().contains("boundary"));
    }
}
