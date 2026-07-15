use perci::backend::{CompositeBackend, LanguageBackend};
use perci::chat::help_text;
use perci::cortex::CortexBridge;
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
        .unwrap_or_else(|| PathBuf::from("memory/perci.jsonl"));

    let backend: Box<dyn LanguageBackend> = Box::new(CompositeBackend::discover()?);
    let cortex = CortexBridge::discover();
    let mut engine = ChatEngine::new(personality, MemoryStore::new(memory_path), backend, cortex);

    match command.as_str() {
        "chat" => interactive(&mut engine)?,
        "ask" => {
            let input = args.collect::<Vec<_>>().join(" ");
            if input.trim().is_empty() {
                return Err("usage: perci ask <message>".into());
            }
            println!("{}", engine.respond(&input)?.text);
        }
        "status" => print_status(&engine),
        "help" | "--help" | "-h" => println!("{}", help_text()),
        other => return Err(format!("unknown command: {other}").into()),
    }

    Ok(())
}

fn interactive(engine: &mut ChatEngine) -> io::Result<()> {
    println!("â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”");
    println!("â”‚  Perci Â· compact governed local intelligence â”‚");
    println!("â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜");
    println!("backend: {}", engine.backend_name());
    println!("cortex: {}", engine.cortex_status());
    println!("type /help for commands");

    let stdin = io::stdin();
    loop {
        print!("\nYou > ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/quit" | "/exit" => break,
            "/help" => println!("{}", help_text()),
            "/status" => print_status(engine),
            "/cortex" => println!("cortex: {}", engine.cortex_status()),
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
    println!("cortex: {}", engine.cortex_status());
    println!("reflex router: explicit-command parser");
    println!("general cognition: 4,096-bit Bitwork expert + prototype inference");
    println!("exact reasoning: checked i128/rational arithmetic + symbolic geometry");
    println!("memory: append-only JSONL + optional Cortex selective recall");
}

fn load_personality() -> Personality {
    let path = env::var("PERCI_PERSONALITY").unwrap_or_else(|_| "config/personality.prompt".into());
    Personality::load(path).unwrap_or_else(|_| Personality::default_perci())
}
