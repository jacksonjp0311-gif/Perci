use perci::backend::{CompositeBackend, LanguageBackend};
use perci::chat::help_text;
use perci::cortex::CortexBridge;
use perci::memory::MemoryStore;
use perci::{ChatEngine, Personality};
use std::env;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

mod ui;
use ui::BloodUi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "chat".into());

    // Daemon has its own warm engine — start before other setup
    if matches!(command.as_str(), "daemon" | "serve") {
        return perci::daemon::run_server().map_err(|e| e);
    }

    let personality = load_personality();
    let memory_path = env::var_os("PERCI_MEMORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("memory/perci.jsonl"));

    let backend: Box<dyn LanguageBackend> = Box::new(CompositeBackend::discover()?);
    let cortex = CortexBridge::discover();
    let session = perci::session::SessionStore::discover();
    let learner = perci::learning::InteractionLearner::discover();
    let mut engine = ChatEngine::new(personality, MemoryStore::new(memory_path), backend, cortex)
        .with_session(session)
        .with_learning(learner);

    match command.as_str() {
        "chat" => interactive(&mut engine)?,
        "ask" => {
            let input = args.collect::<Vec<_>>().join(" ");
            if input.trim().is_empty() {
                return Err("usage: perci ask <message>".into());
            }
            // Prefer warm daemon when live (skip process cold-start)
            if perci::daemon::ping() {
                match perci::daemon::ask_daemon(&input) {
                    Ok(text) => {
                        println!("{text}");
                        return Ok(());
                    }
                    Err(e) => eprintln!("daemon ask fallback: {e}"),
                }
            }
            println!("{}", engine.respond(&input)?.text);
        }
        "session" => {
            let sub = args.next().unwrap_or_else(|| "path".into());
            match sub.as_str() {
                "clear" | "reset" => {
                    perci::session::SessionStore::discover().clear()?;
                    println!("session cleared");
                }
                "path" | "status" => {
                    println!(
                        "session: {}",
                        engine
                            .session_path()
                            .unwrap_or_else(|| "memory/session.jsonl".into())
                    );
                }
                other => {
                    return Err(format!("usage: perci session path|clear  (got {other})").into())
                }
            }
        }
        "classify" => {
            let input = args.collect::<Vec<_>>().join(" ");
            if input.trim().is_empty() {
                return Err("usage: perci classify <message>".into());
            }
            if perci::daemon::ping() {
                if let Ok(v) = perci::daemon::classify_daemon(&input) {
                    println!("{v}");
                    return Ok(());
                }
            }
            println!("{}", classify_json(&input)?);
        }
        "status" => print_status(&engine),
        "learning" | "learn" => print_learning(&engine),
        "teach" => {
            let claim = args.collect::<Vec<_>>().join(" ");
            if claim.trim().is_empty() {
                return Err("usage: perci teach <claim>".into());
            }
            print_teaching_result(&mut engine, &claim)?;
        }
        "intel" | "intelligence" | "probe" => run_intelligence_probe()?,
        "bench" => run_benchmark(&mut engine)?,
        "ping" => {
            // Minimal latency probe (no chat engine path beyond classify weights)
            let t0 = Instant::now();
            let _ = classify_json("ping")?;
            println!(
                "{{\"ok\":true,\"classify_ms\":{:.2}}}",
                t0.elapsed().as_secs_f64() * 1000.0
            );
        }
        "agent" => {
            let sub = args.next().unwrap_or_else(|| "help".into());
            match sub.as_str() {
                "run" => {
                    let mut dry_run = false;
                    let mut merge_if_green = false;
                    let mut run_tests = true;
                    let mut goal_parts: Vec<String> = Vec::new();
                    for arg in args {
                        match arg.as_str() {
                            "--dry-run" | "-n" => dry_run = true,
                            "--merge-if-green" => merge_if_green = true,
                            "--no-test" => run_tests = false,
                            "--help" | "-h" => {
                                println!(
                                    "usage: perci agent run <goal> [--dry-run] [--merge-if-green] [--no-test]\n\
                                     policy: repo-scoped allowlist; never writes .pwgt; kill switch PERCI_AGENT=0 or .perci/agent.lock"
                                );
                                return Ok(());
                            }
                            other => goal_parts.push(other.to_owned()),
                        }
                    }
                    let goal = goal_parts.join(" ");
                    if goal.trim().is_empty() {
                        return Err(
                            "usage: perci agent run <goal> [--dry-run] [--merge-if-green] [--no-test]"
                                .into(),
                        );
                    }
                    let report = perci::agent::run_agent(
                        &goal,
                        perci::agent::AgentOpts {
                            dry_run,
                            merge_if_green,
                            run_tests,
                        },
                    )?;
                    println!("{}", report.summary());
                    if !report.ok {
                        std::process::exit(1);
                    }
                }
                "lab" => {
                    let mut dry_run = false;
                    let mut from_hardness = false;
                    for arg in args {
                        match arg.as_str() {
                            "--dry-run" | "-n" => dry_run = true,
                            "--from-hardness" => from_hardness = true,
                            "--help" | "-h" => {
                                println!(
                                    "usage: perci agent lab --from-hardness [--dry-run]\n\
                                     Opens Soar-style impasse tickets from red hardness cases.\n\
                                     If all green, writes a raise-hardness note. Never touches .pwgt."
                                );
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    if !from_hardness {
                        return Err(
                            "usage: perci agent lab --from-hardness [--dry-run]".into(),
                        );
                    }
                    let report = perci::agent::run_lab_from_hardness(dry_run)?;
                    println!("{}", report.summary());
                    if !report.ok {
                        std::process::exit(1);
                    }
                }
                "help" | "--help" | "-h" => {
                    println!(
                        "perci agent — local repo agent (L6 MVP)\n\
                         \n\
                         Commands:\n\
                           perci agent run <goal> [--dry-run] [--merge-if-green] [--no-test]\n\
                           perci agent lab --from-hardness [--dry-run]\n\
                         \n\
                         Examples:\n\
                           perci agent run \"add hardness case for why-does-math\" --dry-run\n\
                           perci agent run \"add hardness case for why-does-math\" --merge-if-green\n\
                           perci agent run \"inspect status\" --no-test\n\
                           perci agent lab --from-hardness --dry-run\n\
                         \n\
                         Kill switch: PERCI_AGENT=0 or .perci/agent.lock\n\
                         Weights: never auto-promoted."
                    );
                }
                other => {
                    return Err(format!(
                        "unknown agent subcommand: {other} (try: perci agent help)"
                    )
                    .into());
                }
            }
        }
        "traces" | "trace-history" => {
            let n: usize = args
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10);
            let rows = perci::decision_trace::recent(n)?;
            if rows.is_empty() {
                println!(
                    "no decision traces yet ({})",
                    perci::decision_trace::default_path().display()
                );
            } else {
                println!(
                    "decision traces (last {}): {}",
                    rows.len(),
                    perci::decision_trace::default_path().display()
                );
                for row in rows {
                    println!("{row}");
                }
            }
        }
        "help" | "--help" | "-h" => println!("{}", help_text()),
        other => return Err(format!("unknown command: {other}").into()),
    }

    Ok(())
}

fn interactive(engine: &mut ChatEngine) -> io::Result<()> {
    let ui = BloodUi::detect();
    // Drop PowerShell banner + cargo sync noise; Dark-Blood frame owns the top.
    ui.clear_stage();
    ui.banner(engine.backend_name(), &engine.cortex_status());
    ui.opening(&engine.opening_insight());

    let stdin = io::stdin();
    loop {
        ui.prompt()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            ui.reset_color();
            break;
        }
        // Typed text was purple (open SGR from prompt); close it before Perci replies.
        ui.reset_color();

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        if let Some(claim) = input.strip_prefix("/teach ") {
            if let Err(error) = print_teaching_result(engine, claim) {
                ui.error(&error.to_string());
            }
            continue;
        }

        match input {
            "/quit" | "/exit" => break,
            "/help" => println!("{}", help_text()),
            "/status" => print_status(engine),
            "/cortex" => println!("cortex: {}", engine.cortex_status()),
            "/learning" | "/learn" => print_learning(engine),
            "/trace" | "/thought" => println!("{}", engine.deliberation_trace()),
            "/prompt" => println!("{}", engine.personality().prompt),
            "/intel" | "/intelligence" | "/probe" => {
                if let Err(error) = run_intelligence_probe() {
                    ui.error(&error.to_string());
                }
            }
            "/bench" => {
                if let Err(error) = run_benchmark(engine) {
                    eprintln!("benchmark error: {error}");
                }
            }
            _ => {
                let started = Instant::now();
                match engine.respond(input) {
                    Ok(response) => ui.response(
                        &format!("{:?}", response.route),
                        &response.text,
                        started.elapsed(),
                    ),
                    Err(error) => ui.error(&error.to_string()),
                }
            }
        }
    }

    Ok(())
}

fn print_status(engine: &ChatEngine) {
    let ui = BloodUi::detect();
    ui.section("SYSTEM / VITALS");
    ui.row("name", &engine.personality().name);
    ui.row("version", perci::branding::version_label());
    ui.row("mark", perci::branding::mark_svg_path());
    ui.row("badge", perci::branding::badge_svg_path());
    ui.row("backend", engine.backend_name());
    ui.row("cortex", engine.cortex_status());
    ui.row("packs", perci::intel_packs::status_summary());
    ui.row("cognition", "4,096-bit experts · deduplicated prototypes");
    ui.row("weights", "mmap · PERCIW03 concepts · v2/v1 read fallback");
    ui.row("reasoning", "checked i128/rational · symbolic geometry");
    ui.row("memory", "append-only JSONL · Cortex · offline packs");
    ui.row("learning", engine.learning_status());
    ui.row(
        "session",
        engine
            .session_path()
            .unwrap_or_else(|| "memory/session.jsonl".into()),
    );
    ui.row("voice", "social · multi-turn · reason-loop · self-critic");
    ui.row(
        "daemon",
        format!(
            "{} · {}",
            if perci::daemon::ping() { "live" } else { "off" },
            perci::daemon::addr()
        ),
    );
    ui.row(
        "policy",
        "evidence first · uncertainty visible · no auto-promotion",
    );
}

fn print_learning(engine: &ChatEngine) {
    let ui = BloodUi::detect();
    ui.section("INTERACTION / LEARNING");
    ui.row("state", engine.learning_status());
    ui.row(
        "evidence",
        engine
            .learning_path()
            .unwrap_or_else(|| "disabled".to_owned()),
    );
    ui.row("immediate", "safe dialogue preferences only");
    ui.row(
        "pending",
        "facts · procedures · corrections · weight curriculum",
    );
    ui.row(
        "authority",
        "no automatic fact promotion or weight mutation",
    );
    ui.row(
        "teach",
        "say 'I want you to learn that ...' · /teach is optional",
    );
}

fn print_teaching_result(engine: &mut ChatEngine, claim: &str) -> io::Result<()> {
    let id = engine.stage_teaching(claim)?;
    let ui = BloodUi::detect();
    ui.section("KNOWLEDGE / CANDIDATE");
    ui.row("id", id);
    ui.row("claim", claim.trim());
    ui.row("state", "pending review · not active truth");
    ui.row("next", "add provenance/test, approve, rebuild, evaluate");
    Ok(())
}

fn run_intelligence_probe() -> io::Result<()> {
    let ui = BloodUi::detect();
    let path = perci::cognitive::default_weight_path();
    let weights = perci::cognitive::CognitiveWeights::load(&path)?;
    let cases = [
        ("greeting", "hello perci"),
        ("identity", "what are your capabilities"),
        ("math", "calculate 17 percent of 240"),
        ("geometry", "triangle area base 8 height 5"),
        ("code", "debug a Rust parser ownership error"),
        ("governance", "require authorization and a rollback receipt"),
        (
            "science",
            "design a falsifiable hypothesis with measurements",
        ),
        (
            "planning",
            "sequence milestones, dependencies, and acceptance criteria",
        ),
    ];
    ui.section("INTELLIGENCE / LIVE PROBE");
    ui.row("model", path.display().to_string());
    ui.row("prototypes", weights.prototype_count().to_string());
    let mut passed = 0usize;
    for (expected, prompt) in cases {
        let matched = weights.classify(prompt)?;
        let pass = matched.label == expected;
        passed += usize::from(pass);
        ui.verdict(
            pass,
            &format!(
                "{:<10} ← {:<10} margin={:<4} z={:>5.2} J={:.3} · {}",
                matched.label, expected, matched.margin, matched.overlap_z, matched.jaccard, prompt
            ),
        );
    }
    ui.row("result", format!("{passed}/{} domain probes", cases.len()));
    ui.row(
        "claim ceiling",
        "diagnostic probe only · sealed evaluation remains authoritative",
    );
    Ok(())
}

fn run_benchmark(engine: &mut ChatEngine) -> io::Result<()> {
    println!("Perci micro-benchmark");
    println!("---------------------");

    benchmark_case(engine, "fast greeting", "hello perci")?;
    benchmark_case(engine, "exact arithmetic", "calculate 20 percent of 80")?;
    benchmark_case(
        engine,
        "Cortex cold/warm",
        "Explain how counterexample search improves debugging reliability",
    )?;
    benchmark_case(
        engine,
        "Cortex cached",
        "Explain how counterexample search improves debugging reliability",
    )?;

    println!("Note: first Cortex use includes lazy Python daemon startup; later calls reuse it.");
    Ok(())
}

fn benchmark_case(engine: &mut ChatEngine, label: &str, input: &str) -> io::Result<()> {
    let started = Instant::now();
    let response = engine.respond(input)?;
    let elapsed = started.elapsed();
    println!(
        "{label:18} {:>9.3} ms | {:?} | {} chars",
        elapsed.as_secs_f64() * 1000.0,
        response.route,
        response.text.len()
    );
    Ok(())
}

fn load_personality() -> Personality {
    let path = env::var("PERCI_PERSONALITY").unwrap_or_else(|_| "config/personality.prompt".into());
    Personality::load(path).unwrap_or_else(|_| Personality::default_perci())
}

/// JSON classify for Lumen hybrid (`schema/label/variant/score/overlap`).
fn classify_json(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    use serde_json::json;

    let path = perci::cognitive::default_weight_path();

    if path.is_file() {
        let weights = perci::cognitive::CognitiveWeights::load(&path)?;
        let matched = weights.classify(input)?;
        let mixture: Vec<serde_json::Value> = matched
            .mixture
            .iter()
            .map(|m| {
                json!({
                    "label": m.label,
                    "score": m.score,
                    "overlap": m.overlap,
                    "concept_id": m.concept_id,
                    "insight": m.insight,
                    "residual": m.residual,
                    "hop": m.hop,
                    "attention_pm": m.attention_pm,
                })
            })
            .collect();
        let row = json!({
            "schema": "perci.classify.v5-attn",
            "label": matched.label,
            "variant": matched.variant,
            "concept_id": matched.concept_id,
            "insight": matched.insight,
            "skeleton": matched.concept_skeleton(3),
            "composition": matched.composition_frame(8),
            "primary_attention_pm": matched.primary_attention_pm,
            "mixture": mixture,
            "score": matched.score,
            "overlap": matched.overlap,
            "runner_up_score": matched.runner_up_score,
            "margin": matched.margin,
            "query_popcount": matched.query_popcount,
            "prototype_popcount": matched.prototype_popcount,
            "positive_overlap": matched.positive_overlap,
            "negative_overlap": matched.negative_overlap,
            "hamming": matched.hamming,
            "jaccard": matched.jaccard,
            "overlap_z": matched.overlap_z,
        });
        return Ok(row.to_string());
    }

    // Lexical fallback when weights missing
    let lower = input.to_ascii_lowercase();
    let (label, score) = if lower.contains("hello") || lower.contains("hi ") {
        ("greeting", 50)
    } else if lower.contains("who are") || lower.contains("what are you") {
        ("identity", 55)
    } else if lower.contains("cargo")
        || lower.contains("compile")
        || lower.contains("rust")
        || lower.contains("function")
        || lower.contains("implement")
    {
        ("code", 60)
    } else if lower.contains("percent")
        || lower.contains("calculate")
        || lower.contains("math")
        || lower.contains("fraction")
    {
        ("math", 55)
    } else if lower.contains("govern") || lower.contains("permission") {
        ("governance", 50)
    } else if lower.contains("triangle") || lower.contains("circle") || lower.contains("radius") {
        ("geometry", 55)
    } else if lower.contains("science") || lower.contains("hypothesis") {
        ("science", 50)
    } else {
        ("general", 20)
    };
    let row = json!({
        "schema": "perci.classify.v1-fallback",
        "label": label,
        "variant": 0,
        "score": score,
        "overlap": (score.max(0) as u32) / 4,
    });
    Ok(row.to_string())
}
