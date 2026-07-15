use perci::backend::{CognitiveBackend, CommandBackend, DeterministicBackend, LanguageBackend};
use perci::chat::help_text;
use perci::memory::MemoryStore;
use perci::{ChatEngine, Personality};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "chat".into());
    let personality = load_personality();
    let memory_path = env::var_os("PERCI_MEMORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("memory/perci.mem"));
    let backend: Box<dyn LanguageBackend> = match CommandBackend::from_env() {
        Some(v) => Box::new(v),
        None => match CognitiveBackend::discover()? {
            Some(v) => Box::new(v),
            None => Box::new(DeterministicBackend),
        },
    };
    let mut engine = ChatEngine::new(personality, MemoryStore::new(memory_path), backend);

    match command.as_str() {
        "chat" => interactive(&mut engine)?,
        "ask" => {
            let input = args.collect::<Vec<_>>().join(" ");
            if input.trim().is_empty() { return Err("usage: perci ask <message>".into()); }
            println!("{}", engine.respond(&input)?.text);
        }
        "status" => print_status(&engine),
        "help" | "--help" | "-h" => println!("{}", help_text()),
        other => return Err(format!("unknown command: {other}").into()),
    }
    Ok(())
}

fn interactive(engine: &mut ChatEngine) -> io::Result<()> {
    println!("┌──────────────────────────────────────────────┐");
    println!("│  Perci · compact governed local intelligence │");
    println!("└──────────────────────────────────────────────┘");
    println!("backend: {} · type /help for commands", engine.backend_name());
    let stdin = io::stdin();
    loop {
        print!("\nYou > ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 { break; }
        let input = line.trim();
        if input.is_empty() { continue; }
        match input {
            "/quit" | "/exit" => break,
            "/help" => println!("{}", help_text()),
            "/status" => print_status(engine),
            "/prompt" => println!("{}", engine.personality().prompt),
            _ => match engine.respond(input) {
                Ok(response) => println!("Perci [{:?}] > {}", response.route, response.text),
                Err(error) => eprintln!("Perci error: {error}"),
            },
        }
    }
    Ok(())
}

fn print_status(engine: &ChatEngine) {
    println!("name: {}", engine.personality().name);
    println!("backend: {}", engine.backend_name());
    println!("cognitive weights: {}", if engine.backend_name().contains("fallback") { "not attached" } else { "attached" });
    println!("reflex router: active (64-bit POPCOUNT)");
    println!("exact reasoning: integer/rational arithmetic + basic geometry");
}

fn load_personality() -> Personality {
    let path = env::var("PERCI_PERSONALITY").unwrap_or_else(|_| "config/personality.prompt".into());
    Personality::load(path).unwrap_or_else(|_| Personality::default_perci())
}
