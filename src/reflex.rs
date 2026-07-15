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
/// Text is converted into a 64-bit feature word. Routing uses AND, XOR and
/// POPCOUNT only. Later, these masks can be evolved from real interaction logs.
#[derive(Clone, Debug)]
pub struct ReflexRouter {
    prototypes: [(Route, u64); 6],
}

impl Default for ReflexRouter {
    fn default() -> Self {
        Self {
            prototypes: [
                (Route::Math, features("calculate solve equation arithmetic plus minus multiply divide")),
                (Route::Geometry, features("geometry triangle circle angle area perimeter pythagorean")),
                (Route::MemoryWrite, features("remember store save memory note")),
                (Route::MemorySearch, features("recall search memory what did we say")),
                (Route::Help, features("help commands usage")),
                (Route::Chat, features("talk explain think discuss why how")),
            ],
        }
    }
}

impl ReflexRouter {
    pub fn route(&self, text: &str) -> Route {
        let input = features(text);
        self.prototypes
            .iter()
            .max_by_key(|(_, prototype)| (!(input ^ *prototype)).count_ones())
            .map(|(route, _)| *route)
            .unwrap_or(Route::Chat)
    }
}

/// Feature hashing packs word and character signals into one machine word.
pub fn features(text: &str) -> u64 {
    let mut bits = 0u64;
    for token in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        if token.is_empty() { continue; }
        let mut h = 0xcbf29ce484222325u64;
        for byte in token.bytes().map(|b| b.to_ascii_lowercase()) {
            h ^= byte as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        bits |= 1u64 << (h & 63);
        bits |= 1u64 << ((h >> 11) & 63);
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_math() {
        assert_eq!(ReflexRouter::default().route("calculate 8 plus 9"), Route::Math);
    }
}
