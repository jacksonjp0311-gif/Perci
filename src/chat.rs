use crate::backend::LanguageBackend;
use crate::memory::MemoryStore;
use crate::personality::Personality;
use crate::reasoning::{solve_arithmetic, solve_geometry};
use crate::reflex::{ReflexRouter, Route};
use std::io;

#[derive(Debug)]
pub struct ChatResponse {
    pub route: Route,
    pub text: String,
}

pub struct ChatEngine {
    personality: Personality,
    memory: MemoryStore,
    router: ReflexRouter,
    backend: Box<dyn LanguageBackend>,
}

impl ChatEngine {
    pub fn new(personality: Personality, memory: MemoryStore, backend: Box<dyn LanguageBackend>) -> Self {
        Self { personality, memory, router: ReflexRouter::default(), backend }
    }

    pub fn backend_name(&self) -> &str { self.backend.name() }
    pub fn personality(&self) -> &Personality { &self.personality }

    pub fn respond(&mut self, input: &str) -> io::Result<ChatResponse> {
        let route = self.router.route(input);
        let text = match route {
            Route::Math => solve_arithmetic(input).map(|v| format!("Exact result: {v}")).unwrap_or_else(|e| e.to_string()),
            Route::Geometry => solve_geometry(input).unwrap_or_else(|e| e.to_string()),
            Route::MemoryWrite => {
                let content = strip_memory_prefix(input);
                self.memory.append(content)?;
                format!("Stored in local append-only memory: {content}")
            }
            Route::MemorySearch => {
                let found = self.memory.search(input, 5)?;
                if found.is_empty() { "No matching local memories found.".into() }
                else { format!("Relevant memory:\n- {}", found.join("\n- ")) }
            }
            Route::Help => help_text().into(),
            Route::Chat => {
                let context = self.memory.search(input, 4)?;
                self.backend.generate(&self.personality.prompt, &context, input)?
            }
        };
        Ok(ChatResponse { route, text })
    }
}

fn strip_memory_prefix(input: &str) -> &str {
    let trimmed = input.trim();
    for prefix in ["remember that", "remember", "store", "save", "note that"] {
        if trimmed.to_ascii_lowercase().starts_with(prefix) {
            return trimmed[prefix.len()..].trim_start_matches([' ', ':']);
        }
    }
    trimmed
}

pub fn help_text() -> &'static str {
    "Commands:\n  /help               show commands\n  /status             show runtime status\n  /prompt             show personality prompt\n  /quit               exit\nNatural tools:\n  calculate 12 divided by 5\n  triangle area base 8 height 5\n  remember that Perci uses governed memory\n  recall governed memory"
}
