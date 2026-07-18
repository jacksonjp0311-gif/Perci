//! Replay learning + baseline comparisons (v0.8.6).
//!
//! Compare native engines on a fixed curriculum JSONL without promoting weights.
//! Promotion remains human-authorized and only when held-out candidate beats active.

use crate::compositional_world::CompositionalWorld;
use crate::entity_slot;
use crate::native_decoder;
use crate::reason_loop;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaselineRow {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub motif_a: String,
    #[serde(default)]
    pub motif_b: String,
    #[serde(default)]
    pub topic: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineResult {
    pub engine: String,
    pub topic_hit: bool,
    pub slot_pair_hit: bool,
    pub compositional_score: i64,
    pub answer_chars: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayReport {
    pub schema: String,
    pub n: usize,
    pub engines: Vec<EngineSummary>,
    pub promote_recommended: bool,
    pub promote_reason: String,
    pub claim_boundary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineSummary {
    pub engine: String,
    pub topic_binding_rate: f64,
    pub slot_pair_binding_rate: f64,
    pub mean_compositional_score: f64,
}

fn candidates_path() -> PathBuf {
    PathBuf::from("models/candidates")
}

/// Load curriculum rows from adversarial / emergence JSONL.
pub fn load_rows(path: &Path, limit: usize) -> std::io::Result<Vec<BaselineRow>> {
    let file = fs::File::open(path)?;
    let mut rows = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
            let prompt = v
                .get("prompt")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_owned();
            if prompt.is_empty() {
                continue;
            }
            rows.push(BaselineRow {
                id: v
                    .get("index")
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| rows.len().to_string()),
                prompt,
                family: v
                    .get("family_name")
                    .or_else(|| v.get("constraint"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_owned(),
                motif_a: v
                    .get("motif_a")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_owned(),
                motif_b: v
                    .get("motif_b")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_owned(),
                topic: v
                    .get("topic")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_owned(),
            });
            if rows.len() >= limit {
                break;
            }
        }
    }
    Ok(rows)
}

fn score_answer(row: &BaselineRow, answer: &str, world: &CompositionalWorld) -> EngineResult {
    let low = answer.to_ascii_lowercase();
    let topic = if !row.topic.is_empty() {
        row.topic.as_str()
    } else {
        row.motif_a.as_str()
    };
    let topic_hit = !topic.is_empty() && low.contains(&topic.to_ascii_lowercase());
    let slot_pair_hit = if !row.motif_a.is_empty() && !row.motif_b.is_empty() {
        entity_slot::slots_bound_in_speech(answer, &row.motif_a, &row.motif_b)
    } else {
        topic_hit
    };
    EngineResult {
        engine: String::new(),
        topic_hit,
        slot_pair_hit,
        compositional_score: world.score_speech(&row.prompt, answer),
        answer_chars: answer.chars().count(),
    }
}

fn answer_for_engine(engine: &str, prompt: &str) -> String {
    match engine {
        "entity-slot" => {
            if entity_slot::looks_entity_slot_transfer(prompt) {
                entity_slot::entity_slot_transfer_answer(prompt).answer
            } else {
                String::new()
            }
        }
        "native-decoder" => native_decoder::decode(prompt, None).text,
        "reason-loop" => reason_loop::run_loop(prompt).answer,
        "compositional" => {
            let w = CompositionalWorld::seed();
            if let Some(f) = entity_slot::extract_entity_slot_frame(prompt) {
                w.explain_pair(&f.slot_a, &f.slot_b)
            } else {
                "No slots parsed.".into()
            }
        }
        _ => String::new(),
    }
}

/// Compare engines on a curriculum; never promotes.
pub fn compare_baselines(path: &Path, limit: usize) -> std::io::Result<ReplayReport> {
    let rows = load_rows(path, limit)?;
    let world = CompositionalWorld::seed();
    let engines = ["entity-slot", "native-decoder", "reason-loop", "compositional"];
    let mut summaries = Vec::new();

    for eng in engines {
        let mut topic_hits = 0usize;
        let mut pair_hits = 0usize;
        let mut pair_n = 0usize;
        let mut score_sum = 0i64;
        let mut n = 0usize;
        for row in &rows {
            let ans = answer_for_engine(eng, &row.prompt);
            if ans.is_empty() {
                // entity-slot only applies to entity_swap family
                if eng == "entity-slot" && row.family != "entity_swap" {
                    continue;
                }
            }
            let mut r = score_answer(row, &ans, &world);
            r.engine = eng.into();
            n += 1;
            if r.topic_hit {
                topic_hits += 1;
            }
            if !row.motif_a.is_empty() && !row.motif_b.is_empty() {
                pair_n += 1;
                if r.slot_pair_hit {
                    pair_hits += 1;
                }
            }
            score_sum += r.compositional_score;
        }
        if n == 0 {
            continue;
        }
        summaries.push(EngineSummary {
            engine: eng.into(),
            topic_binding_rate: topic_hits as f64 / n as f64,
            slot_pair_binding_rate: if pair_n > 0 {
                pair_hits as f64 / pair_n as f64
            } else {
                0.0
            },
            mean_compositional_score: score_sum as f64 / n as f64,
        });
    }

    // Active baseline proxy: entity-slot on entity_swap; native-decoder overall.
    let active_topic = summaries
        .iter()
        .find(|s| s.engine == "native-decoder")
        .map(|s| s.topic_binding_rate)
        .unwrap_or(0.0);
    let best = summaries
        .iter()
        .max_by(|a, b| {
            a.slot_pair_binding_rate
                .partial_cmp(&b.slot_pair_binding_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|s| s.engine.clone())
        .unwrap_or_default();

    // Never auto-recommend promote of binary fields from this report alone.
    let promote_recommended = false;
    let promote_reason = format!(
        "best_engine={best}; active_native_decoder_topic={active_topic:.3}; \
human authorize required; binary field promote only if held-out beats active on full probe"
    );

    Ok(ReplayReport {
        schema: "perci.replay-baseline.v1".into(),
        n: rows.len(),
        engines: summaries,
        promote_recommended,
        promote_reason,
        claim_boundary:
            "engineering comparison only — not AGI; never auto-promote .pwgt / .bwm / .bphr"
                .into(),
    })
}

/// Write report JSON to models/candidates.
pub fn write_report(report: &ReplayReport) -> std::io::Result<PathBuf> {
    let dir = candidates_path();
    fs::create_dir_all(&dir)?;
    let path = dir.join("replay-baseline-v0.8.6.json");
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;
    writeln!(f, "{}", serde_json::to_string_pretty(report).unwrap_or_default())?;
    Ok(path)
}

pub fn format_report(r: &ReplayReport) -> String {
    let mut out = format!(
        "[Replay baselines · n={} · promote_recommended={}]\n{}\n\n",
        r.n, r.promote_recommended, r.promote_reason
    );
    for e in &r.engines {
        out.push_str(&format!(
            "  · {:<16} topic={:.1}% slot_pair={:.1}% mean_comp={:.1}\n",
            e.engine,
            e.topic_binding_rate * 100.0,
            e.slot_pair_binding_rate * 100.0,
            e.mean_compositional_score
        ));
    }
    out.push_str(&format!("\n{}\n", r.claim_boundary));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn replay_on_tiny_curriculum() {
        let dir = std::env::temp_dir().join(format!("perci-replay-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("tiny.jsonl");
        {
            let mut f = fs::File::create(&path).unwrap();
            writeln!(
                f,
                r#"{{"index":0,"family_name":"entity_swap","topic":"trust","motif_a":"trust","motif_b":"change","prompt":"An unfamiliar device called Quoril-7 has trust and change. Transfer one relation to it without treating the invented name as evidence."}}"#
            )
            .unwrap();
            writeln!(
                f,
                r#"{{"index":1,"family_name":"paraphrase","topic":"boundary","motif_a":"boundary","motif_b":"identity","prompt":"State how boundary relates to identity in new words."}}"#
            )
            .unwrap();
        }
        let report = compare_baselines(&path, 10).unwrap();
        assert!(report.n >= 1);
        assert!(!report.promote_recommended);
        let _ = fs::remove_dir_all(dir);
    }
}
