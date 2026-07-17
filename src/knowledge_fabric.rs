//! Knowledge fabric — source-bearing retrieval (not pack weight knowledge).
//!
//! Phase 2: hybrid-ready ranking over local intelligence packs + optional
//! evidence ledger. Network retrieval is capability-gated (default off).

use crate::fabric::EvidenceRecord;
use crate::intel_packs::{self, PackHit};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Retrieve evidence for a query; returns typed EvidenceRecords.
pub fn retrieve_evidence(query: &str, limit: usize) -> Vec<EvidenceRecord> {
    let mut out = Vec::new();
    let hits = intel_packs::retrieve(query, limit).unwrap_or_default();
    let now = now_iso();
    for (i, h) in hits.into_iter().enumerate() {
        out.push(hit_to_evidence(i, h, &now));
    }
    // Optional external evidence ledger (append-only, never weights).
    if let Ok(extra) = load_ledger_evidence(query, limit.saturating_sub(out.len())) {
        out.extend(extra);
    }
    // Rank: authority * freshness * lexical overlap with the query (hybrid-ready).
    let q_terms: Vec<String> = query
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(|s| s.to_owned())
        .take(12)
        .collect();
    out.sort_by(|a, b| {
        let sa = rank_evidence(a, &q_terms);
        let sb = rank_evidence(b, &q_terms);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(limit);
    out
}

fn rank_evidence(e: &EvidenceRecord, q_terms: &[String]) -> f64 {
    let blob = format!("{} {} {}", e.claim, e.source, e.supports.join(" ")).to_ascii_lowercase();
    let overlap = if q_terms.is_empty() {
        0.5
    } else {
        let hits = q_terms.iter().filter(|t| blob.contains(t.as_str())).count();
        hits as f64 / q_terms.len() as f64
    };
    e.authority * e.freshness.max(0.1) * (0.35 + 0.65 * overlap)
}

fn hit_to_evidence(i: usize, h: PackHit, now: &str) -> EvidenceRecord {
    let auth = (0.55 + (h.score as f64 * 0.02).min(0.4)).min(0.95);
    EvidenceRecord {
        claim_id: format!("pack-{}-{}", i, short_hash(&h.path)),
        claim: h.excerpt.chars().take(280).collect(),
        source_type: "intelligence_pack".into(),
        source: h.path,
        retrieved_at: now.to_owned(),
        published_at: String::new(),
        content_hash: short_hash(&h.excerpt),
        authority: auth,
        freshness: 0.85,
        supports: vec![h.title],
        contradicts: Vec::new(),
    }
}

fn evidence_ledger_path() -> PathBuf {
    PathBuf::from("models/candidates/knowledge-evidence.jsonl")
}

fn load_ledger_evidence(query: &str, limit: usize) -> std::io::Result<Vec<EvidenceRecord>> {
    let path = evidence_ledger_path();
    if !path.is_file() || limit == 0 {
        return Ok(Vec::new());
    }
    let terms: Vec<String> = query
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 4)
        .map(|s| s.to_owned())
        .take(8)
        .collect();
    let text = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        if let Ok(ev) = serde_json::from_str::<EvidenceRecord>(line) {
            let blob = format!("{} {}", ev.claim, ev.source).to_ascii_lowercase();
            let hits = terms.iter().filter(|t| blob.contains(t.as_str())).count();
            if hits > 0 {
                out.push(ev);
            }
        }
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

/// Stage an evidence record (human/AI may append; never auto-promotes weights).
pub fn stage_evidence(ev: &EvidenceRecord) -> std::io::Result<()> {
    let path = evidence_ledger_path();
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{}", serde_json::to_string(ev).unwrap_or_default())?;
    Ok(())
}

/// Detect simple contradictions among evidence claims (keyword negation).
pub fn find_contradictions(records: &[EvidenceRecord]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for i in 0..records.len() {
        for j in (i + 1)..records.len() {
            let a = records[i].claim.to_ascii_lowercase();
            let b = records[j].claim.to_ascii_lowercase();
            if (a.contains(" always ") && b.contains(" never "))
                || (a.contains(" never ") && b.contains(" always "))
                || (a.contains("safe") && b.contains("unsafe"))
            {
                pairs.push((records[i].claim_id.clone(), records[j].claim_id.clone()));
            }
            for c in &records[i].contradicts {
                if b.contains(&c.to_ascii_lowercase()) {
                    pairs.push((records[i].claim_id.clone(), records[j].claim_id.clone()));
                }
            }
        }
    }
    pairs
}

fn now_iso() -> String {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{t}")
}

fn short_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:x}", h.finish() & 0xffff_ffff)
}

/// Hybrid retrieval report for CLI / fabric.
pub fn status_report(query: &str) -> String {
    let ev = retrieve_evidence(query, 5);
    let contra = find_contradictions(&ev);
    let mut out = format!(
        "[Knowledge fabric] query={:?} hits={}\n",
        query.chars().take(60).collect::<String>(),
        ev.len()
    );
    for e in &ev {
        out.push_str(&format!(
            "  · {} auth={:.2} fresh={:.2} src={}\n    {}\n",
            e.claim_id,
            e.authority,
            e.freshness,
            e.source,
            e.claim.chars().take(120).collect::<String>()
        ));
    }
    if !contra.is_empty() {
        out.push_str(&format!("contradictions: {contra:?}\n"));
    }
    out.push_str("law: facts via retrieval+provenance — never bake into .pwgt without authorize\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieve_does_not_panic() {
        let _ = retrieve_evidence("trust lag timeout bitwork", 3);
    }
}
