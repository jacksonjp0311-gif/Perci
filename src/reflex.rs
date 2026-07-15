/// Fast, deterministic first-pass routing.
///
/// The reflex layer handles only explicit control intents. Open-language domain
/// classification belongs to Perci's 4,096-bit cognitive weights, avoiding the
/// collision problems of the former 64-bit fuzzy router.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Chat,
    Math,
    Geometry,
    MemoryWrite,
    MemorySearch,
    Help,
}

#[derive(Clone, Debug, Default)]
pub struct ReflexRouter;

impl ReflexRouter {
    pub fn route(&self, text: &str) -> Route {
        let normalized = normalize(text);

        if matches!(
            normalized.as_str(),
            "help" | "/help" | "commands" | "show commands" | "usage"
        ) {
            return Route::Help;
        }

        // Explicit memory mutation and recall always outrank topic words.
        if starts_with_any(
            &normalized,
            &[
                "remember that ",
                "remember ",
                "store ",
                "save ",
                "note that ",
                "note ",
            ],
        ) {
            return Route::MemoryWrite;
        }

        if starts_with_any(
            &normalized,
            &[
                "recall ",
                "search memory ",
                "find memory ",
                "what did i remember",
                "what do you remember",
            ],
        ) {
            return Route::MemorySearch;
        }

        Route::Chat
    }
}

fn starts_with_any(text: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| text.starts_with(prefix))
}

fn normalize(text: &str) -> String {
    text.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_help_is_help() {
        assert_eq!(ReflexRouter.route("help"), Route::Help);
        assert_eq!(ReflexRouter.route("/help"), Route::Help);
    }

    #[test]
    fn ordinary_questions_are_chat() {
        let router = ReflexRouter;
        assert_eq!(router.route("hello perci"), Route::Chat);
        assert_eq!(router.route("what can you do"), Route::Chat);
        assert_eq!(router.route("what is your purpose"), Route::Chat);
        assert_eq!(router.route("what is a triangle"), Route::Chat);
    }

    #[test]
    fn memory_commands_have_highest_semantic_precedence() {
        let router = ReflexRouter;
        assert_eq!(
            router.route("remember that triangle area is base times height divided by two"),
            Route::MemoryWrite
        );
        assert_eq!(
            router.route("remember that 2 plus 2 equals 4"),
            Route::MemoryWrite
        );
        assert_eq!(
            router.route("recall the triangle formula"),
            Route::MemorySearch
        );
    }

    #[test]
    fn keyword_traps_remain_chat() {
        let router = ReflexRouter;
        assert_eq!(router.route("explain square brackets in Rust"), Route::Chat);
        assert_eq!(
            router.route("we need a perimeter security design"),
            Route::Chat
        );
        assert_eq!(
            router.route("compare the ratio of memory to CPU usage"),
            Route::Chat
        );
        assert_eq!(
            router.route("explain the minus token in this parser"),
            Route::Chat
        );
    }
}
