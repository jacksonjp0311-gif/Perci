//! Warm recall latency for cited memory.
//!
//!   PERCI_AGENTSEARCH="python3 -m agentsearch" cargo run --release --example recall_bench -- 2000
//!
//! Writes N notes into a temporary journal, then measures `recall` p50/p95
//! for the AgentSearch backend (if available) and the substring fallback.
use perci::cited_memory::CitedMemory;
use std::time::Instant;

fn main() -> std::io::Result<()> {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);
    let dir = std::env::temp_dir().join(format!("perci-recall-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let journal = dir.join("perci.jsonl");
    let topics = [
        "dentist", "backup", "invoice", "router", "garden", "passport", "deploy", "keyboard",
    ];
    let mut writer = CitedMemory::new(&journal).without_agentsearch();
    let t = Instant::now();
    for i in 0..n {
        let topic = topics[i % topics.len()];
        writer.remember(
            "note",
            &format!("note {i}: the {topic} detail number {i} is item-{}", i * 7),
        )?;
    }
    println!("wrote {n} notes in {:?}", t.elapsed());
    let queries = [
        "where is the backup detail",
        "dentist",
        "invoice item",
        "router number",
        "nothing matches zebra",
    ];
    for (label, mut mem) in [
        ("agentsearch", CitedMemory::new(&journal)),
        (
            "substring",
            CitedMemory::new(&journal).without_agentsearch(),
        ),
    ] {
        let cold = Instant::now();
        let first = mem.recall("passport", 5)?;
        let cold = cold.elapsed();
        if label == "agentsearch" && first.backend != "agentsearch" {
            println!("agentsearch: unavailable ({:?})", first.warnings);
            continue;
        }
        let mut samples = Vec::new();
        for round in 0..40 {
            for q in &queries {
                let t = Instant::now();
                let r = mem.recall(q, 5)?;
                samples.push(t.elapsed().as_micros() as u64);
                if round == 0 && *q == "where is the backup detail" {
                    println!(
                        "  example: {} hits, first: {:?}",
                        r.hits.len(),
                        r.hits.first().map(|h| &h.text)
                    );
                }
            }
        }
        samples.sort_unstable();
        let p = |q: f64| samples[((samples.len() as f64 - 1.0) * q) as usize];
        println!(
            "{label:<12} n={n:<6} cold first recall {:>8.1} ms · warm p50 {:>6.2} ms · p95 {:>6.2} ms",
            cold.as_secs_f64() * 1e3,
            p(0.50) as f64 / 1e3,
            p(0.95) as f64 / 1e3
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
