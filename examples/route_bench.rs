//! Bitwork classify latency and a fingerprint of every result, so a speed
//! change can be shown to leave answers bit-identical.
//!
//!   cargo run --release --example route_bench -- [v0.2|v0.3|PATH] [ROUNDS]
//!
//! Prompts: every `"prompt"` in training/*.jsonl (deduplicated, sorted) plus a
//! few fixed tool-style asks. Prints p50/p95/mean per classify and the
//! sha256 of the Debug rendering of all matches.

use perci::cognitive::CognitiveWeights;
use std::time::Instant;

fn prompts() -> Vec<String> {
    let mut out: Vec<String> = [
        "why is my rust build slow",
        "what is 15% of 80 plus 3",
        "deploy the service after the tests pass",
        "remember that the backup drive is in the left desk drawer",
        "explain how photosynthesis works",
        "hi there",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let dir = perci::paths::asset("training");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.sort();
    for f in files
        .iter()
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
    {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        for line in text.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(p) = v.get("prompt").and_then(|p| p.as_str()) {
                    out.push(p.to_owned());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pack = args.first().map(String::as_str).unwrap_or("v0.3");
    let rounds: usize = args.get(1).and_then(|r| r.parse().ok()).unwrap_or(3);
    let path = match pack {
        "v0.1" | "v0.2" | "v0.3" => {
            perci::paths::asset(format!("models/perci-cognitive-{pack}.pwgt"))
        }
        other => other.into(),
    };
    let w = CognitiveWeights::load(&path).expect("load pack");
    let ps = prompts();
    let mut fingerprint = String::new();
    for p in &ps {
        fingerprint.push_str(&format!("{p}\n{:?}\n", w.classify(p).expect("classify")));
    }
    let mut times = Vec::new();
    for _ in 0..rounds {
        for p in &ps {
            let t = Instant::now();
            let m = w.classify(p).expect("classify");
            times.push(t.elapsed().as_secs_f64() * 1e3);
            std::hint::black_box(m);
        }
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |q: f64| times[((times.len() - 1) as f64 * q) as usize];
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    println!(
        "pack {pack}: {} prompts x {rounds} rounds: p50 {:.3} ms, p95 {:.3} ms, mean {:.3} ms",
        ps.len(),
        pct(0.5),
        pct(0.95),
        mean
    );
    println!(
        "fingerprint sha256 {}",
        perci::doctor::sha256_hex(fingerprint.as_bytes())
    );
}
