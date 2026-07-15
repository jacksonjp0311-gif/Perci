use crate::backend::LanguageBackend;
use crate::cortex::CortexBridge;
use crate::memory::MemoryStore;
use crate::personality::Personality;
use crate::reasoning::{try_solve_arithmetic, try_solve_geometry};
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
    cortex: Option<CortexBridge>,
}

impl ChatEngine {
    pub fn new(
        personality: Personality,
        memory: MemoryStore,
        backend: Box<dyn LanguageBackend>,
        cortex: Option<CortexBridge>,
    ) -> Self {
        Self {
            personality,
            memory,
            router: ReflexRouter,
            backend,
            cortex,
        }
    }

    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    pub fn personality(&self) -> &Personality {
        &self.personality
    }

    pub fn cortex_status(&self) -> String {
        self.cortex
            .as_ref()
            .map(CortexBridge::status_label)
            .unwrap_or_else(|| "not found".to_owned())
    }

    pub fn respond(&mut self, input: &str) -> io::Result<ChatResponse> {
        match self.router.route(input) {
            Route::Help => {
                return Ok(ChatResponse {
                    route: Route::Help,
                    text: help_text().into(),
                });
            }
            Route::MemoryWrite => return self.remember(input),
            Route::MemorySearch => return self.recall(input),
            Route::Chat | Route::Math | Route::Geometry => {}
        }

        match try_solve_arithmetic(input) {
            Ok(Some(value)) => {
                return Ok(ChatResponse {
                    route: Route::Math,
                    text: format!("Exact result: {value}"),
                });
            }
            Ok(None) => {}
            Err(error) => {
                return Ok(ChatResponse {
                    route: Route::Math,
                    text: format!("Exact arithmetic error: {error}"),
                });
            }
        }

        match try_solve_geometry(input) {
            Ok(Some(value)) => {
                return Ok(ChatResponse {
                    route: Route::Geometry,
                    text: value,
                });
            }
            Ok(None) => {}
            Err(error) => {
                return Ok(ChatResponse {
                    route: Route::Geometry,
                    text: format!("Exact geometry error: {error}"),
                });
            }
        }

        let context = self.collect_context(input, 4, 800)?;
        let text = self
            .backend
            .generate(&self.personality.prompt, &context, input)?;

        Ok(ChatResponse {
            route: Route::Chat,
            text,
        })
    }

    fn remember(&mut self, input: &str) -> io::Result<ChatResponse> {
        let content = strip_memory_prefix(input);
        self.memory.append_kind("note", content)?;

        let cortex_note = match self.cortex.as_ref() {
            Some(cortex) if cortex.ready() => match cortex.remember("note", content) {
                Ok(()) => " Cortex episodic memory also recorded the explicit event.",
                Err(_) => " Cortex sync was unavailable; local JSONL memory remains intact.",
            },
            Some(_) => " Cortex is present but requires bootstrap; local JSONL memory was written.",
            None => " Cortex is not attached; local JSONL memory was written.",
        };

        Ok(ChatResponse {
            route: Route::MemoryWrite,
            text: format!("Stored explicit local memory: {content}.{cortex_note}"),
        })
    }

    fn recall(&mut self, input: &str) -> io::Result<ChatResponse> {
        let query = strip_recall_prefix(input);
        let found = self.collect_context(query, 5, 700)?;

        let text = if found.is_empty() {
            "No matching local or Cortex memory was found.".to_owned()
        } else {
            format!("Relevant governed context:\n- {}", found.join("\n- "))
        };

        Ok(ChatResponse {
            route: Route::MemorySearch,
            text,
        })
    }

    fn collect_context(
        &self,
        query: &str,
        local_limit: usize,
        cortex_budget: usize,
    ) -> io::Result<Vec<String>> {
        let mut context = self.memory.search(query, local_limit)?;

        if let Some(cortex) = self.cortex.as_ref() {
            if cortex.ready() {
                if let Ok(mut evidence) = cortex.retrieve(query, cortex_budget) {
                    context.append(&mut evidence);
                }
            }
        }

        context.dedup();
        Ok(context)
    }
}

fn strip_memory_prefix(input: &str) -> &str {
    strip_prefixes(
        input,
        &[
            "remember that",
            "remember",
            "store",
            "save",
            "note that",
            "note",
        ],
    )
}

fn strip_recall_prefix(input: &str) -> &str {
    strip_prefixes(
        input,
        &[
            "recall",
            "search memory for",
            "search memory",
            "find memory for",
            "find memory",
            "what did i remember about",
            "what do you remember about",
        ],
    )
}

fn strip_prefixes<'a>(input: &'a str, prefixes: &[&str]) -> &'a str {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();

    for prefix in prefixes {
        if lower.starts_with(prefix) {
            return trimmed[prefix.len()..].trim_start_matches([' ', ':']);
        }
    }

    trimmed
}

pub fn help_text() -> &'static str {
    "Commands:\n  /help               show commands\n  /status             show runtime status\n  /cortex             show Cortex attachment status\n  /prompt             show personality prompt\n  /quit               exit\nNatural tools:\n  calculate 12 divided by 5\n  calculate 20 percent of 80\n  triangle area base 8 height 5\n  circle circumference radius 4\n  remember that Perci uses governed memory\n  recall governed memory"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recall_prefix_is_removed() {
        assert_eq!(
            strip_recall_prefix("recall the triangle formula"),
            "the triangle formula"
        );
    }

    #[test]
    fn memory_prefix_is_removed() {
        assert_eq!(
            strip_memory_prefix("remember that 2 plus 2 equals 4"),
            "2 plus 2 equals 4"
        );
    }
}
