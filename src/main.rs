use perci::backend::{CompositeBackend, LanguageBackend};
use perci::chat::help_text;
use perci::cortex::CortexBridge;
use perci::deliberation;
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

    // Identity only — no pack load. Launch-Perci and scripts use this to prove build.
    if matches!(
        command.as_str(),
        "--version" | "-V" | "version" | "build-id" | "build_id"
    ) {
        println!("Perci {}", perci::branding::build_id());
        return Ok(());
    }

    // Daemon has its own warm engine — start before other setup
    if matches!(command.as_str(), "daemon" | "serve") {
        return perci::daemon::run_server().map_err(|e| e);
    }
    // Language weight maintenance must not require loading the active chat
    // backend; this also lets a rebuild replace an older binary format.
    if matches!(command.as_str(), "language" | "lang") {
        return run_language_command(&mut args);
    }
    // Layered low-bit maintenance/probes do not require the chat backend.
    if matches!(command.as_str(), "lowbit" | "low-bit" | "bitlayer") {
        return run_lowbit_command(&mut args);
    }
    // Modular cognition: pack route, ThoughtPlan, field-fold experiment (no chat backend).
    if matches!(
        command.as_str(),
        "modular" | "packs" | "thought" | "fold-exp" | "field-fold"
    ) {
        return run_modular_command(&command, &mut args);
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
            // Prefer warm daemon only when its build_id matches this binary.
            // Stale daemons after source fixes silently broke chat — never again.
            if perci::daemon::ping_current() {
                match perci::daemon::ask_daemon(&input) {
                    Ok(text) => {
                        println!("{text}");
                        return Ok(());
                    }
                    Err(e) => eprintln!("daemon ask fallback: {e}"),
                }
            } else if perci::daemon::ping() {
                eprintln!(
                    "daemon stale (build_id mismatch) — using local binary {}",
                    perci::branding::build_id()
                );
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
        "language" | "lang" => run_language_command(&mut args)?,
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
        "transfer" | "xfer" => {
            let base = args.collect::<Vec<_>>().join(" ");
            if base.trim().is_empty() {
                return Err(
                    "usage: perci transfer \"<base prompt>\" | perci transfer-suite\n\
                     Runs base + paraphrase + novel-noun transfer gate on operator speech."
                        .into(),
                );
            }
            if base.trim() == "suite" || base.trim() == "--suite" {
                let (ok, report) = perci::emergence::run_transfer_suite();
                print!("{report}");
                if !ok {
                    std::process::exit(1);
                }
                return Ok(());
            }
            let report = perci::emergence::run_operator_transfer(base.trim());
            print!("{report}");
            if report.contains("pass=false") {
                std::process::exit(1);
            }
        }
        "transfer-suite" | "xfer-suite" => {
            let (ok, report) = perci::emergence::run_transfer_suite();
            print!("{report}");
            let (ok2, report2) = perci::emergence::run_softcascade_trust_transfer();
            print!("{report2}");
            if !ok || !ok2 {
                std::process::exit(1);
            }
        }
        "hydra" | "inject" => run_hydra_command(&mut args)?,
        "fabric" => {
            let sub = args.next().unwrap_or_else(|| "status".into());
            match sub.as_str() {
                "status" | "help" | "--help" | "-h" => {
                    println!("{}", perci::fabric::status_report());
                }
                "plan" => {
                    let prompt = args.collect::<Vec<_>>().join(" ");
                    if prompt.trim().is_empty() {
                        return Err("usage: perci fabric plan <prompt>".into());
                    }
                    let plan = perci::fabric::plan_for_prompt(prompt.trim(), "cli");
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                }
                "knowledge" => {
                    let q = args.collect::<Vec<_>>().join(" ");
                    println!("{}", perci::knowledge_fabric::status_report(q.trim()));
                }
                "orchestrate" => {
                    let prompt = args.collect::<Vec<_>>().join(" ");
                    if prompt.trim().is_empty() {
                        return Err("usage: perci fabric orchestrate <prompt>".into());
                    }
                    println!("{}", perci::orchestrate::plan_json(prompt.trim()));
                    let seed = deliberation::try_deliberate(prompt.trim(), &[], &[])
                        .map(|d| d.answer)
                        .unwrap_or_else(|| {
                            "No operator match; SoftCascade/pack path applies.".into()
                        });
                    let out = perci::orchestrate::enrich_answer(prompt.trim(), "fabric-cli", &seed);
                    println!("---\n{out}");
                }
                "handoff" | "entry" => {
                    let prompt = args.collect::<Vec<_>>().join(" ");
                    let task = if prompt.trim().is_empty() {
                        "general evolution — read lab queue and improve next gap".to_owned()
                    } else {
                        prompt.trim().to_owned()
                    };
                    let packet = perci::fabric::build_handoff(&task);
                    match perci::fabric::write_handoff_latest(&packet) {
                        Ok(p) => eprintln!("wrote {}", p.display()),
                        Err(e) => eprintln!("handoff persist warning: {e}"),
                    }
                    println!("{}", serde_json::to_string_pretty(&packet)?);
                }
                "evolve" | "loop" => {
                    println!("{}", perci::fabric::evolve_loop_report());
                }
                "next" | "work" => {
                    println!("{}", perci::emergence::next_work_report());
                    let items = perci::emergence::open_work_items();
                    if !items.is_empty() {
                        println!(
                            "---\njson:\n{}",
                            serde_json::to_string_pretty(&items).unwrap_or_default()
                        );
                    }
                }
                "regress" | "regression" => {
                    println!("{}", perci::fabric::regress_report());
                }
                "decode" => {
                    let prompt = args.collect::<Vec<_>>().join(" ");
                    if prompt.trim().is_empty() {
                        return Err("usage: perci fabric decode <prompt>".into());
                    }
                    let r = perci::native_decoder::decode(prompt.trim(), None);
                    println!("layers={:?} ok={}\n{}", r.layers, r.ok, r.text);
                }
                "reason" => {
                    let prompt = args.collect::<Vec<_>>().join(" ");
                    if prompt.trim().is_empty() {
                        return Err("usage: perci fabric reason <prompt>".into());
                    }
                    let r = perci::reason_loop::run_loop(prompt.trim());
                    println!("{}", perci::reason_loop::format_receipt(&r));
                }
                "replay" | "baselines" => {
                    let path = args.next().unwrap_or_else(|| {
                        "models/candidates/adversarial-v0.8.4-heldout.jsonl".into()
                    });
                    let limit = args
                        .next()
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(120);
                    let report =
                        perci::replay_learn::compare_baselines(std::path::Path::new(&path), limit)?;
                    if let Ok(p) = perci::replay_learn::write_report(&report) {
                        eprintln!("wrote {}", p.display());
                    }
                    println!("{}", perci::replay_learn::format_report(&report));
                }
                "compose" | "world-compose" => {
                    println!(
                        "{}",
                        perci::compositional_world::CompositionalWorld::status_report()
                    );
                    let prompt = args.collect::<Vec<_>>().join(" ");
                    if !prompt.trim().is_empty() {
                        let w = perci::compositional_world::CompositionalWorld::seed();
                        if let Some(f) =
                            perci::entity_slot::extract_entity_slot_frame(prompt.trim())
                        {
                            println!("{}", w.explain_pair(&f.slot_a, &f.slot_b));
                        } else {
                            println!("(pass entity-slot style prompt to see multi-hop paths)");
                        }
                    }
                }
                other => {
                    return Err(format!(
                        "unknown fabric subcommand: {other} (try: status|plan|knowledge|orchestrate|handoff|next|regress|evolve|decode|reason|replay|compose)"
                    )
                    .into());
                }
            }
        }
        "lab" => {
            let sub = args.next().unwrap_or_else(|| "queue".into());
            match sub.as_str() {
                "queue" | "next" | "status" => {
                    println!("{}", perci::emergence::lab_report());
                    println!("{}", perci::emergence::next_queue_item());
                }
                "unified" | "world" => {
                    println!("{}", perci::emergence::unified_queue_report());
                }
                "curriculum" | "cluster" => {
                    println!("{}", perci::emergence::curriculum_cluster_report());
                }
                "patterns" | "pattern" | "intel-patterns" => {
                    println!("{}", perci::emergence::pattern_intelligence_report());
                }
                "feed" | "channels" => {
                    println!("{}", perci::emergence::feed_all_channels_report());
                }
                "hygiene" => {
                    println!("{}", perci::emergence::hygiene_dual_tickets());
                }
                "field" => {
                    println!("{}", perci::emergence::status_report(32));
                }
                "close" => {
                    let id = args
                        .next()
                        .ok_or("usage: perci lab close <ticket-id> --reason \"…\"")?;
                    let mut reason = String::from("resolved");
                    let rest: Vec<String> = args.collect();
                    let mut i = 0;
                    while i < rest.len() {
                        if rest[i] == "--reason" || rest[i] == "-r" {
                            reason = rest[i + 1..].join(" ");
                            break;
                        }
                        i += 1;
                    }
                    if reason == "resolved" && !rest.is_empty() && rest[0] != "--reason" {
                        reason = rest.join(" ");
                    }
                    println!("{}", perci::emergence::close_ticket(&id, &reason)?);
                }
                "help" | "--help" | "-h" => {
                    println!(
                        "perci lab — emergence self-improve queue (L8)\n\
                         \n\
                         Commands:\n\
                           perci lab queue                 open tickets + next work item\n\
                           perci lab unified               hardness + emergence + curriculum\n\
                           perci lab curriculum            pack-debt cluster by label\n\
                           perci lab patterns              emergent laws from ledger\n\
                           perci lab feed                  all five intelligence channels\n\
                           perci lab hygiene               drop open tickets if closed exists\n\
                           perci lab field                 geometry (curriculum view)\n\
                           perci lab close <id> --reason   resolve ticket\n\
                           perci transfer \"<prompt>\"       single transfer gate\n\
                           perci transfer-suite            full transfer law suite\n\
                         \n\
                         Agent:\n\
                           perci agent lab --from-emergence [--repair] [--dry-run]\n\
                           perci agent lab --full [--repair] [--dry-run]\n\
                           perci agent lab --from-hardness [--dry-run]\n\
                         \n\
                         Release: python scripts/release_gates.py\n\
                         Never auto-promotes .pwgt weights."
                    );
                }
                other => {
                    return Err(
                        format!("unknown lab subcommand: {other} (try: perci lab help)").into(),
                    );
                }
            }
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
                            budget: perci::agent::ExecutionBudget::default(),
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
                    let mut from_emergence = false;
                    let mut full = false;
                    let mut repair = false;
                    let mut repair_hardness = false;
                    for arg in args {
                        match arg.as_str() {
                            "--dry-run" | "-n" => dry_run = true,
                            "--from-hardness" => from_hardness = true,
                            "--from-emergence" => from_emergence = true,
                            "--full" => full = true,
                            "--repair" => repair = true,
                            "--repair-hardness" => repair_hardness = true,
                            "--help" | "-h" => {
                                println!(
                                    "usage: perci agent lab --from-hardness|--from-emergence|--full|--repair-hardness [--repair] [--dry-run]\n\
                                     --from-hardness     impasse tickets from red hardness cases\n\
                                     --from-emergence    transfer suite + close open primary-fix tickets\n\
                                     --repair            on transfer fail, stage hardness locks + re-gate\n\
                                     --repair-hardness   hardness fail → auto-repairs.jsonl (runtime operator catalog)\n\
                                     --full              hardness + emergence world loop\n\
                                     Never touches .pwgt."
                                );
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    if repair_hardness {
                        let report = perci::agent::run_repair_from_hardness(dry_run)?;
                        println!("{}", report.summary());
                        if !report.ok {
                            std::process::exit(1);
                        }
                    } else if full {
                        let report = perci::agent::run_lab_full(dry_run, repair)?;
                        println!("{}", report.summary());
                        if !report.ok {
                            std::process::exit(1);
                        }
                    } else if from_emergence {
                        let report = perci::agent::run_lab_from_emergence_opts(
                            perci::agent::LabFromEmergenceOpts { dry_run, repair },
                        )?;
                        println!("{}", report.summary());
                        if !report.ok {
                            std::process::exit(1);
                        }
                    } else if from_hardness {
                        let report = perci::agent::run_lab_from_hardness(dry_run)?;
                        println!("{}", report.summary());
                        if !report.ok {
                            std::process::exit(1);
                        }
                    } else {
                        return Err(
                            "usage: perci agent lab --from-hardness|--from-emergence|--full [--repair] [--dry-run]"
                                .into(),
                        );
                    }
                }
                "help" | "--help" | "-h" => {
                    println!(
                        "perci agent — local repo agent (L6/L8)\n\
                         \n\
                         Commands:\n\
                           perci agent run <goal> [--dry-run] [--merge-if-green] [--no-test]\n\
                           perci agent lab --from-hardness [--dry-run]\n\
                           perci agent lab --from-emergence [--repair] [--dry-run]\n\
                           perci agent lab --full [--repair] [--dry-run]\n\
                         \n\
                         Kill switch: PERCI_AGENT=0 or .perci/agent.lock\n\
                         Weights: never auto-promoted.\n\
                         Release: python scripts/release_gates.py"
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
            let n: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(10);
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
            "/language" | "/lang" => println!(
                "{}\n\n{}\n\n{}",
                perci::binary_language::status_report(),
                perci::binary_phrase::status_report(),
                perci::binary_relation::status_report()
            ),
            "/trace" | "/thought" => println!("{}", engine.deliberation_trace()),
            "/field" | "/emergence" | "/geometry" => {
                println!("{}", perci::emergence::status_report(24));
            }
            "/lab" | "/tickets" => {
                println!("{}", perci::emergence::lab_report());
                println!("{}", perci::emergence::next_queue_item());
            }
            "/patterns" => {
                println!("{}", perci::emergence::pattern_intelligence_report());
            }
            "/feed" | "/channels" => {
                println!("{}", perci::emergence::feed_all_channels_report());
            }
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
            other if other == "/think" || other.starts_with("/think ") => {
                let arg = other.strip_prefix("/think").unwrap_or("").trim();
                match arg {
                    "on" => {
                        engine.set_verbose_cognition(true);
                        println!(
                            "deep backend plans ON — chat stays clean; /think shows richer geometry"
                        );
                    }
                    "off" => {
                        engine.set_verbose_cognition(false);
                        println!("deep backend plans OFF — /think still shows last plan");
                    }
                    "" => println!("{}", engine.cognition_think()),
                    _ => println!(
                        "usage: /think | /think on | /think off\n(chat never shows cognition traces)\n{}",
                        engine.cognition_think()
                    ),
                }
            }
            "/concise" | "/short" | "/brief" => match engine.set_style_depth("concise") {
                Ok(msg) => println!("{msg}"),
                Err(e) => ui.error(&e.to_string()),
            },
            "/deep" | "/detailed" | "/thorough" => match engine.set_style_depth("deep") {
                Ok(msg) => println!("{msg}"),
                Err(e) => ui.error(&e.to_string()),
            },
            "/balanced" | "/natural" => match engine.set_style_depth("balanced") {
                Ok(msg) => println!("{msg}"),
                Err(e) => ui.error(&e.to_string()),
            },
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
    ui.row(
        "low-bit",
        "PERCLBW1 sidecar · ternary blocks · residual planes · INT4 escape lane",
    );
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

fn run_hydra_command<I>(args: &mut I) -> Result<(), Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    use perci::hydra_inject::{
        apply_code_injection, brpc_stress_field, discover_markers, load_brpc_factor_values,
        plan_code_injection, residual_field, write_json_pretty, CodeInjectSpec, FieldConfig,
    };

    let sub = args.next().unwrap_or_else(|| "status".into());
    let root = std::env::current_dir()?;
    let out_dir = root.join("models/candidates/hydra-bridge");
    std::fs::create_dir_all(&out_dir)?;

    match sub.as_str() {
        "help" | "--help" | "-h" => {
            println!(
                "perci hydra — governed inject (pure Rust; no external HYDRA install)\n\
                 \n\
                 Law: anchor → inject → retract → seal · never auto-promote .pwgt\n\
                 \n\
                 perci hydra status\n\
                 perci hydra markers [--slots-only]\n\
                 perci hydra field              # BRPC factors → residual seal (if receipt present)\n\
                 perci hydra plan <spec.json>   # plan codeweave diff\n\
                 perci hydra apply <spec.json> [--write]  # default dry-run\n\
                 \n\
                 Spec JSON fields: target_file, marker, code, name?, mode?, root?, max_bytes?, rationale?, profile?"
            );
        }
        "status" => {
            let markers = discover_markers(&root)?;
            let slots = markers.iter().filter(|m| m.is_slot).count();
            let brpc = root.join("models/candidates/brpc-perci-receipt-latest.json");
            println!("Perci HYDRA inject (native Rust)");
            println!("  root: {}", root.display());
            println!("  markers: {} (slots: {slots})", markers.len());
            println!(
                "  BRPC receipt: {}",
                if brpc.is_file() { "present" } else { "missing" }
            );
            println!("  claim: never auto-promote .pwgt; apply defaults to dry-run");
        }
        "markers" => {
            let slots_only = args.any(|a| a == "--slots-only");
            let markers = discover_markers(&root)?;
            let list: Vec<_> = markers
                .into_iter()
                .filter(|m| !slots_only || m.is_slot)
                .collect();
            println!("{}", serde_json::to_string_pretty(&list)?);
        }
        "field" => {
            let brpc_path = root.join("models/candidates/brpc-perci-receipt-latest.json");
            let values = load_brpc_factor_values(&brpc_path).unwrap_or_else(|| {
                println!("note: no BRPC receipt; using neutral 0.5 factors");
                vec![0.5; 7]
            });
            let (mask, field) = brpc_stress_field(&values);
            let result = residual_field(&mask, &field, &FieldConfig::default())?;
            let path = out_dir.join("field-run-native.json");
            write_json_pretty(&path, &result)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            println!("receipt: {}", path.display());
            if !result.admissible {
                std::process::exit(1);
            }
        }
        "plan" => {
            let spec_path = args
                .next()
                .ok_or("usage: perci hydra plan <spec.json>")?;
            let raw = std::fs::read_to_string(&spec_path)?;
            let mut spec: CodeInjectSpec = serde_json::from_str(&raw)?;
            if spec.root == "." {
                spec.root = root.display().to_string();
            }
            let result = plan_code_injection(&spec);
            let path = out_dir.join("plan-latest.json");
            write_json_pretty(&path, &result)?;
            println!("admissible: {}", result.admissible);
            println!("risk: {:.3}", result.risk_score);
            for w in &result.warnings {
                println!("warning: {w}");
            }
            if !result.diff.is_empty() {
                println!("{}", result.diff);
            }
            println!("receipt: {}", path.display());
            if !result.admissible {
                std::process::exit(1);
            }
        }
        "apply" => {
            let mut write = false;
            let mut spec_path: Option<String> = None;
            for a in args {
                if a == "--write" {
                    write = true;
                } else if !a.starts_with('-') {
                    spec_path = Some(a);
                }
            }
            let spec_path = spec_path.ok_or("usage: perci hydra apply <spec.json> [--write]")?;
            let raw = std::fs::read_to_string(&spec_path)?;
            let mut spec: CodeInjectSpec = serde_json::from_str(&raw)?;
            if spec.root == "." {
                spec.root = root.display().to_string();
            }
            let dry = !write;
            if write {
                eprintln!("WRITE mode: applying injection (review first). Never promotes .pwgt.");
            }
            let result = apply_code_injection(&spec, dry);
            let path = out_dir.join("apply-latest.json");
            write_json_pretty(&path, &result)?;
            println!(
                "admissible={} applied={} dry_run={}",
                result.admissible, result.applied, result.dry_run
            );
            for w in &result.warnings {
                println!("warning: {w}");
            }
            if !result.diff.is_empty() {
                println!("{}", result.diff);
            }
            println!("receipt: {}", path.display());
            if !result.admissible {
                std::process::exit(1);
            }
        }
        other => {
            return Err(format!(
                "unknown hydra subcommand: {other} (try: perci hydra help)"
            )
            .into());
        }
    }
    Ok(())
}

fn run_modular_command<I>(
    command: &str,
    args: &mut I,
) -> Result<(), Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    // `perci modular <sub>` or shortcuts: packs|thought|fold-exp
    let sub = if matches!(command, "modular") {
        args.next().unwrap_or_else(|| "help".into())
    } else {
        command.to_owned()
    };
    match sub.as_str() {
        "help" | "-h" | "--help" => {
            println!(
                "Modular binary cognition (Phase 1–6)\n\
  perci modular status              pack discovery + law\n\
  perci modular route <prompt>      sparse capability route + telemetry\n\
  perci modular plan <prompt>       ThoughtPlan (SEM1+RSN1)\n\
  perci modular sem <prompt>        PERCISEM1 frame extract + retrieve\n\
  perci modular reason <prompt>     PERCIRSN1 bounded transitions\n\
  perci modular discourse <prompt>  PERCIDSC1 discourse plan\n\
  perci modular realize <prompt>    full SEM→RSN→DSC→LM pipeline\n\
  perci modular build-sem|rsn|dsc|lm  build candidate packs\n\
  perci modular eval-sem | eval-dsc   evals\n\
  perci modular fold [depth]        PERCIFLD1 fold experiment\n\
\n\
Law: PERCIW03 retained; packs candidate; no auto-promote; LM wording-only."
            );
        }
        "status" | "packs" => {
            let root = std::path::Path::new("models/candidates/packs");
            let found = if root.is_dir() {
                perci::pack_manifest::discover_manifests(root)
            } else {
                Vec::new()
            };
            println!("modular cognition status");
            println!("  law: PERCIW03 reflex retained · packs sparse · no auto-promote");
            println!("  pack root: {}", root.display());
            println!("  manifests discovered: {}", found.len());
            for m in found.iter().take(12) {
                println!(
                    "  - {} [{}] status={:?} tags={}",
                    m.pack_id,
                    m.magic,
                    m.promotion_status,
                    m.capability_tags.join(",")
                );
            }
            if found.is_empty() {
                println!("  (scaffold with: python scripts/scaffold_modular_packs.py)");
            }
        }
        "route" => {
            let prompt = args.collect::<Vec<_>>().join(" ");
            if prompt.trim().is_empty() {
                return Err("usage: perci modular route <prompt>".into());
            }
            let d = perci::capability_router::route_prompt(&prompt);
            println!("{}", serde_json::to_string_pretty(&d)?);
        }
        "plan" | "thought" => {
            let prompt = args.collect::<Vec<_>>().join(" ");
            if prompt.trim().is_empty() {
                return Err("usage: perci modular plan <prompt>".into());
            }
            // Prefer RSN1 bounded reason → ThoughtPlan; else deliberation; else route-only.
            let intent = perci::thought_plan::Intent::infer_from_prompt(&prompt);
            let plan = if !matches!(
                intent,
                perci::thought_plan::Intent::Social | perci::thought_plan::Intent::Exact
            ) {
                let (run, state, frame) = perci::reason_transition::run_bounded(&prompt, 8);
                let mut plan = run.to_thought_plan(&frame, &state);
                let route = perci::capability_router::route_prompt(&prompt);
                // Prefer executor packs, merge route ids.
                for p in route.active_packs {
                    if !plan.active_packs.iter().any(|x| x == &p) {
                        plan.active_packs.push(p);
                    }
                }
                plan
            } else if let Some(d) = deliberation::try_deliberate(&prompt, &[], &[]) {
                d.to_thought_plan(&prompt)
            } else {
                let mut p = perci::thought_plan::ThoughtPlan::empty(
                    "route-only",
                    intent,
                );
                let route = perci::capability_router::route_prompt(&prompt);
                p.active_packs = route.active_packs;
                p.halt_reason = "no substantive operator matched; route only".into();
                p
            };
            println!("{}", plan.receipt());
            if !plan.surface_answer.is_empty() {
                println!("--- surface ---");
                println!("{}", plan.surface_answer);
            }
            println!("---");
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        "sem" | "semantic" => {
            let prompt = args.collect::<Vec<_>>().join(" ");
            if prompt.trim().is_empty() {
                return Err("usage: perci modular sem <prompt>".into());
            }
            let frame = perci::semantic_field::extract_frame(&prompt);
            println!("{}", frame.summary_line());
            println!("{}", serde_json::to_string_pretty(&frame)?);
            if let Some(pack) = perci::semantic_field::try_load_default() {
                let hits = pack.retrieve(&frame, 3);
                println!("--- retrieve (top3) ---");
                for h in hits {
                    println!(
                        "  sim={}pm idx={} label={}",
                        h.similarity_pm, h.index, h.label
                    );
                }
                println!("mapped_bytes={}", pack.mapped_bytes());
            } else {
                println!("(no PERCISEM1 pack loaded — run: perci modular build-sem)");
            }
        }
        "reason" | "rsn" => {
            let prompt = args.collect::<Vec<_>>().join(" ");
            if prompt.trim().is_empty() {
                return Err("usage: perci modular reason <prompt>".into());
            }
            let (run, state, frame) = perci::reason_transition::run_bounded(&prompt, 8);
            println!("frame: {}", frame.summary_line());
            println!("halt: {} confidence_pm={}", run.halt_reason, run.final_confidence_pm);
            for s in &run.steps {
                println!(
                    "  c{} {} gain={}pm conf={}pm — {}",
                    s.cycle, s.op, s.expected_info_gain_pm, s.confidence_pm, s.note
                );
            }
            let plan = run.to_thought_plan(&frame, &state);
            println!("--- surface ---");
            println!("{}", plan.surface_answer);
            println!("--- receipt ---");
            println!("{}", plan.receipt());
        }
        "build-sem" => {
            let fixture = std::path::Path::new("training/modular/semantic-frames-v1.jsonl");
            let frames = if fixture.is_file() {
                perci::semantic_field::load_fixture_jsonl(fixture)?
            } else {
                // Built-in seed set
                [
                    "Why does trust collapse when communication is delayed?",
                    "How should interfaces earn trust under lag and retry?",
                    "Explain trust failure when messages are delayed.",
                    "What is the boundary between knowledge and attention?",
                    "Connect entropy, memory, and learning — where does the analogy die?",
                    "What does geometry teach about boundary bands vs max coherence?",
                    "How are memory and identity related?",
                    "Entity Klystron-X has lag and trust. Transfer the relation.",
                ]
                .iter()
                .map(|p| perci::semantic_field::extract_frame(p))
                .collect()
            };
            let out = perci::semantic_field::SemanticFieldPack::default_path();
            let n = perci::semantic_field::build_pack(&frames, &out)?;
            println!("built PERCISEM1 candidate: {} frames → {}", n, out.display());
            println!("promotion_status=candidate · never auto-promote");
        }
        "build-rsn" => {
            let out = perci::reason_transition::ReasonTransitionPack::default_path();
            let n = perci::reason_transition::build_default_pack(&out)?;
            println!("built PERCIRSN1 candidate: {} transitions → {}", n, out.display());
            println!("promotion_status=candidate · never auto-promote");
        }
        "build-dsc" => {
            let out = perci::discourse_plan::DiscoursePack::default_path();
            let n = perci::discourse_plan::build_default_pack(&out)?;
            println!("built PERCIDSC1 candidate: {} plans → {}", n, out.display());
            println!("promotion_status=candidate · never auto-promote");
        }
        "build-lm" => {
            let out = perci::language_realize::LanguagePack::default_path();
            let n = perci::language_realize::build_default_pack(&out)?;
            println!("built PERCILM1 candidate: {} atoms → {}", n, out.display());
            println!("promotion_status=candidate · never auto-promote");
        }
        "discourse" | "dsc" => {
            let prompt = args.collect::<Vec<_>>().join(" ");
            if prompt.trim().is_empty() {
                return Err("usage: perci modular discourse <prompt>".into());
            }
            let (run, state, frame) = perci::reason_transition::run_bounded(&prompt, 8);
            let mut plan = run.to_thought_plan(&frame, &state);
            let d = perci::discourse_plan::plan_discourse(&plan, &prompt, 0);
            perci::discourse_plan::apply_plan(&mut plan, &d);
            println!("{}", d.summary());
            println!("connectives: {:?}", d.connectives);
            println!("style: {:?}", d.style_notes);
            let slots = perci::discourse_plan::materialize_slots(&plan, &d);
            for (act, text) in slots {
                println!("  [{}] {}", act.as_str(), text);
            }
        }
        "realize" | "speak" | "lm" => {
            let all: Vec<String> = args.collect();
            if all.is_empty() {
                return Err(
                    "usage: perci modular realize [1|2|4] <prompt>\n  example: perci modular realize 2 \"why does trust collapse under lag?\""
                        .into(),
                );
            }
            let (bits, prompt) = if matches!(
                all[0].as_str(),
                "1" | "2" | "4" | "1bit" | "2bit" | "4bit"
            ) {
                (
                    perci::language_realize::BitWidth::from_str_loose(&all[0]),
                    all[1..].join(" "),
                )
            } else {
                (
                    perci::language_realize::BitWidth::Two,
                    all.join(" "),
                )
            };
            if prompt.trim().is_empty() {
                return Err("usage: perci modular realize [1|2|4] <prompt>".into());
            }
            let r = perci::language_realize::realize_from_prompt(&prompt, bits, 0);
            println!("{}", r.text);
            println!("---");
            println!(
                "discourse={} engine={} constraints_ok={} packs={}",
                r.discourse,
                r.engine,
                r.constraints_ok,
                r.active_packs.join(",")
            );
            if !r.missing_required.is_empty() {
                println!("missing: {:?}", r.missing_required);
            }
            if !r.forbidden_hits.is_empty() {
                println!("forbidden: {:?}", r.forbidden_hits);
            }
        }
        "eval-dsc" => {
            let report = perci::discourse_plan::evaluate_variation(&[
                (
                    perci::thought_plan::Intent::Trust,
                    "how should interfaces earn trust under lag",
                ),
                (
                    perci::thought_plan::Intent::Trust,
                    "why does trust collapse when delayed",
                ),
                (
                    perci::thought_plan::Intent::Trust,
                    "earn trust under timeout and retry",
                ),
                (
                    perci::thought_plan::Intent::CausalExplanation,
                    "why does life maintain local order",
                ),
                (
                    perci::thought_plan::Intent::CausalExplanation,
                    "explain how boundaries enable repair",
                ),
            ]);
            let out = std::path::Path::new("models/candidates/discourse-eval-latest.json");
            if let Some(parent) = out.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(out, serde_json::to_string_pretty(&report)? + "\n")?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            println!("receipt: {}", out.display());
        }
        "eval-sem" => {
            let cases_path = std::path::Path::new("training/modular/semantic-eval-v1.jsonl");
            let cases: Vec<perci::semantic_field::SemEvalCase> = if cases_path.is_file() {
                let text = std::fs::read_to_string(cases_path)?;
                text.lines()
                    .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                    .filter_map(|l| serde_json::from_str(l).ok())
                    .collect()
            } else {
                vec![
                    perci::semantic_field::SemEvalCase {
                        id: "T1".into(),
                        prompts: vec![
                            "Why does trust collapse under lag?".into(),
                            "Explain trust failure when communication is delayed.".into(),
                        ],
                        expected_subject: "trust".into(),
                        expected_condition: "delay".into(),
                        expected_output: "mechanism".into(),
                    },
                    perci::semantic_field::SemEvalCase {
                        id: "B1".into(),
                        prompts: vec![
                            "What is the boundary between knowledge and attention?".into(),
                            "Explain the boundary of knowledge vs attention.".into(),
                        ],
                        expected_subject: "boundary".into(),
                        expected_condition: String::new(),
                        expected_output: "mechanism".into(),
                    },
                ]
            };
            let pack = perci::semantic_field::try_load_default();
            let report = perci::semantic_field::evaluate_semantic(&cases, pack.as_ref());
            let out = std::path::Path::new("models/candidates/semantic-eval-latest.json");
            if let Some(parent) = out.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(out, serde_json::to_string_pretty(&report)? + "\n")?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            println!("receipt: {}", out.display());
        }
        "fold" | "fold-exp" | "field-fold" => {
            let depth: u32 = args
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3)
                .clamp(1, 8);
            let report = perci::field_fold::run_experiment(0xC0FFEE, depth);
            let out = std::path::Path::new("models/candidates/field-fold-experiment-latest.json");
            if let Some(parent) = out.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(out, serde_json::to_string_pretty(&report)? + "\n")?;
            println!("field-fold experiment depth={depth}");
            for f in &report.findings {
                println!("  finding: {f}");
            }
            println!(
                "  trials={} receipt={}",
                report.trials.len(),
                out.display()
            );
            // Compact table
            for t in report.trials.iter().filter(|t| t.depth == 1 || t.depth == depth) {
                println!(
                    "  {} d{} recon={}pm match={}pm mis={}pm gen={}pm us={}",
                    t.operator,
                    t.depth,
                    t.reconstruction_similarity_pm,
                    t.matched_decode_pm,
                    t.mismatched_decode_pm,
                    t.generic_decode_pm,
                    t.latency_us
                );
            }
        }
        other => {
            return Err(format!(
                "unknown modular subcommand: {other} (try: perci modular help)"
            )
            .into());
        }
    }
    Ok(())
}

fn run_lowbit_command<I>(args: &mut I) -> Result<(), Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref().unwrap_or("status") {
        "status" | "inspect" => {
            println!("{}", perci::low_bit::status_report());
        }
        "probe" | "test" => {
            let probe = perci::low_bit::run_probe()?;
            println!("layered low-bit probe");
            println!("  baseline weight MSE: {:.8}", probe.baseline_mse);
            println!("  corrected weight MSE: {:.8}", probe.corrected_mse);
            println!("  INT4 activation MSE: {:.8}", probe.activation_mse);
            println!("  sparse outliers: {}", probe.outliers);
            println!(
                "  Hadamard roundtrip max error: {:.8}",
                probe.hadamard_roundtrip_max_error
            );
            println!("  PERCLBW1 bytes: {}", probe.serialized_bytes);
            println!(
                "  binary roundtrip max error: {:.8}",
                probe.serialized_roundtrip_max_error
            );
            println!(
                "  result: {}",
                if probe.corrected_mse <= probe.baseline_mse
                    && probe.hadamard_roundtrip_max_error < 1.0e-5
                    && probe.serialized_roundtrip_max_error < 1.0e-6
                {
                    "PASS"
                } else {
                    "FAIL"
                }
            );
        }
        "train" | "pack" => {
            let input = args
                .next()
                .ok_or("usage: perci lowbit train <dataset.json> <candidate.blw> [--block-size N] [--residual-planes N] [--rank N]")?;
            let output = args
                .next()
                .ok_or("usage: perci lowbit train <dataset.json> <candidate.blw> [--block-size N] [--residual-planes N] [--rank N]")?;
            let rest: Vec<String> = args.collect();
            let mut config = perci::low_bit::LayeredWeightConfig::default();
            let mut index = 0usize;
            while index < rest.len() {
                let flag = rest[index].as_str();
                let value = rest
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {flag}; expected an integer"))?;
                match flag {
                    "--block-size" => config.block_size = value.parse()?,
                    "--residual-planes" => config.residual_planes = value.parse()?,
                    "--rank" | "--correction-rank" => config.correction_rank = value.parse()?,
                    other => return Err(format!("unknown lowbit train option: {other}").into()),
                }
                index += 2;
            }
            let report = perci::low_bit::train_from_json(&input, &output, config)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "assess" | "evaluate" => {
            let input = args
                .next()
                .ok_or("usage: perci lowbit assess <dataset.json> <candidate.blw>")?;
            let candidate = args
                .next()
                .ok_or("usage: perci lowbit assess <dataset.json> <candidate.blw>")?;
            let report = perci::low_bit::assess_candidate_from_json(&input, &candidate)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report.assessment != "PASS" {
                std::process::exit(1);
            }
        }
        other => {
            return Err(format!(
                "unknown lowbit subcommand: {other} (try: status|probe|train|assess)"
            )
            .into());
        }
    }
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

fn run_language_command<I>(args: &mut I) -> Result<(), Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    let sub = args.next().unwrap_or_else(|| "status".into());
    match sub.as_str() {
        "status" | "inspect" => println!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
            perci::binary_language::status_report(),
            perci::binary_phrase::status_report(),
            perci::binary_relation::status_report(),
            perci::binary_world::status_report(),
            perci::compositional_world::CompositionalWorld::status_report(),
            perci::native_decoder::status_report()
        ),
        "train" | "rebuild" => {
            let values = args.collect::<Vec<_>>();
            let source = values.first().map(String::as_str).unwrap_or("--repo");
            let output = values
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(perci::binary_language::default_weight_path);
            let order = values
                .get(2)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(6);
            let stats = perci::binary_language::train_source(source, &output, order)?;
            let phrase_output = if output == perci::binary_language::default_weight_path() {
                perci::binary_phrase::default_weight_path()
            } else {
                output.with_extension("bphr")
            };
            let phrase_stats = perci::binary_phrase::train_source(
                source,
                &phrase_output,
                order.min(4),
            )?;
            let relation_output = if output == perci::binary_language::default_weight_path() {
                perci::binary_relation::default_weight_path()
            } else {
                output.with_extension("brel")
            };
            let relation_stats =
                perci::binary_relation::train_source(source, &relation_output)?;
            let world_output = if output == perci::binary_language::default_weight_path() {
                perci::binary_world::default_weight_path()
            } else {
                output.with_extension("bwm")
            };
            let world_stats = perci::binary_world::train_source(source, &world_output)?;
            println!(
                "native language rebuilt\n  byte output: {}\n  byte order: {}\n  byte records: {}\n  byte transitions: {}\n  byte source bytes: {}\n  byte file bytes: {}\n  phrase output: {}\n  phrase order: {}\n  phrase vocabulary: {}\n  phrase records: {}\n  phrase transitions: {}\n  phrase file bytes: {}\n  relation output: {}\n  relation records: {}\n  relation source bytes: {}\n  relation file bytes: {}\n  world output: {}\n  world records: {}\n  world source bytes: {}\n  world file bytes: {}",
                output.display(),
                stats.order,
                stats.records,
                stats.unique_transitions,
                stats.source_bytes,
                stats.file_bytes,
                phrase_output.display(),
                phrase_stats.order,
                phrase_stats.vocabulary,
                phrase_stats.records,
                phrase_stats.entries,
                phrase_stats.file_bytes,
                relation_output.display(),
                relation_stats.records,
                relation_stats.source_bytes,
                relation_stats.file_bytes,
                world_output.display(),
                world_stats.records,
                world_stats.source_bytes,
                world_stats.file_bytes,
            );
        }
        "sample" | "ask" => {
            let prompt = args.collect::<Vec<_>>().join(" ");
            if prompt.trim().is_empty() {
                return Err("usage: perci language sample <prompt>".into());
            }
            let text = if let Some(model) = perci::binary_phrase::BinaryPhraseModel::discover()? {
                model.generate_reply(&prompt, "general", 520, 1)
            } else {
                let model = perci::binary_language::BinaryLanguageModel::discover()?.ok_or(
                    "native language weights are not trained; run perci language train --repo",
                )?;
                model.generate_reply(&prompt, "general", 520, 1)
            };
            println!("{text}");
        }
        "help" | "--help" | "-h" => println!(
            "Native language commands:\n  perci language status\n  perci language train --repo [output] [order]\n  perci language train <file-or-directory> [output] [order]\n  perci language sample <prompt>\n\nTraining rebuilds PERCLNG1, PERCPHR1, PERCREL1, and PERCIWM1 typed native fields; no external model is used."
        ),
        other => {
            return Err(format!(
                "unknown language subcommand: {other} (try: status|train|sample|help)"
            )
            .into())
        }
    }
    Ok(())
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
