/// Fast routing decision produced by a tiny binary reflex layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Chat,
    Math,
    Geometry,
    MemoryWrite,
    MemorySearch,
    Help,
}

/// A compact hand-seeded Bitwork router.
///
/// Text is converted into a 64-bit sparse feature word. Routing scores only
/// positive feature overlap rather than rewarding matching zero bits.
#[derive(Clone, Debug)]
pub struct ReflexRouter {
    prototypes: [(Route, u64); 6],
}

impl Default for ReflexRouter {
    fn default() -> Self {
        Self {
            prototypes: [
                (
                    Route::Math,
                    features(
                        "calculate solve equation arithmetic plus minus multiply divide divided number fraction percentage ratio",
                    ),
                ),
                (
                    Route::Geometry,
                    features(
                        "geometry triangle circle angle area perimeter pythagorean square rectangle radius diameter shape",
                    ),
                ),
                (
                    Route::MemoryWrite,
                    features("remember store save memory note record retain"),
                ),
                (
                    Route::MemorySearch,
                    features("recall search memory retrieve remembered find"),
                ),
                (
                    Route::Help,
                    features("help commands usage instructions"),
                ),
                (
                    Route::Chat,
                    features(
                        "talk explain think discuss why how what who purpose can do hello hi describe understand reason",
                    ),
                ),
            ],
        }
    }
}

impl ReflexRouter {
    pub fn route(&self, text: &str) -> Route {
        let normalized = text.trim().to_ascii_lowercase();

        if matches!(
            normalized.as_str(),
            "help" | "/help" | "commands" | "show commands" | "usage"
        ) {
            return Route::Help;
        }

        if contains_any(
            &normalized,
            &[
                "triangle",
                "circle",
                "geometry",
                "angle",
                "area",
                "perimeter",
                "pythagorean",
                "radius",
                "diameter",
                "rectangle",
                "square",
            ],
        ) {
            return Route::Geometry;
        }

        if contains_any(
            &normalized,
            &[
                "calculate",
                "arithmetic",
                "equation",
                "divided by",
                "multiply",
                "plus",
                "minus",
                "percentage",
                "fraction",
                "ratio",
            ],
        ) {
            return Route::Math;
        }

        if starts_with_any(
            &normalized,
            &[
                "remember ",
                "remember that ",
                "store ",
                "save ",
                "note ",
                "note that ",
            ],
        ) {
            return Route::MemoryWrite;
        }

        if starts_with_any(
            &normalized,
            &[
                "recall ",
                "search memory",
                "find memory",
                "what did i remember",
                "what do you remember",
            ],
        ) {
            return Route::MemorySearch;
        }

        let input = features(&normalized);

        let mut best_route = Route::Chat;
        let mut best_score = 0u32;

        for (route, prototype) in &self.prototypes {
            if *route == Route::Help {
                continue;
            }

            let overlap = (input & *prototype).count_ones();
            let union = (input | *prototype).count_ones();

            let score = if union == 0 {
                0
            } else {
                overlap * 1000 / union
            };

            if score > best_score {
                best_score = score;
                best_route = *route;
            }
        }

        if best_score < 40 {
            return Route::Chat;
        }

        best_route
    }
}

fn starts_with_any(text: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| text.starts_with(prefix))
}

fn contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

pub fn features(text: &str) -> u64 {
    let mut bits = 0u64;

    for token in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        if token.is_empty() {
            continue;
        }

        let mut hash = 0xcbf29ce484222325u64;

        for byte in token.bytes().map(|byte| byte.to_ascii_lowercase()) {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        bits |= 1u64 << (hash & 63);
        bits |= 1u64 << ((hash >> 11) & 63);
    }

    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_math() {
        assert_eq!(
            ReflexRouter::default().route("calculate 8 plus 9"),
            Route::Math
        );
    }

    #[test]
    fn detects_geometry() {
        assert_eq!(
            ReflexRouter::default().route("what is the area of a triangle"),
            Route::Geometry
        );
    }

    #[test]
    fn detects_explicit_help() {
        assert_eq!(ReflexRouter::default().route("help"), Route::Help);
        assert_eq!(ReflexRouter::default().route("/help"), Route::Help);
    }

    #[test]
    fn ordinary_questions_are_not_help() {
        let router = ReflexRouter::default();

        assert_eq!(router.route("hello perci"), Route::Chat);
        assert_eq!(router.route("what can you do"), Route::Chat);
        assert_eq!(router.route("what is your purpose"), Route::Chat);
    }

    #[test]
    fn detects_memory_write() {
        assert_eq!(
            ReflexRouter::default().route("remember that Perci is local"),
            Route::MemoryWrite
        );
    }

    #[test]
    fn detects_memory_search() {
        assert_eq!(
            ReflexRouter::default().route("recall governed memory"),
            Route::MemorySearch
        );
    }
}

