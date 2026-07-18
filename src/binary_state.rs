//! Fixed-size recurrent binary state for native dialogue continuity.
//!
//! This is deliberately a state accumulator, not a hidden neural network. It
//! stores four 64-bit lanes, updates them with integer rotations and XORs, and
//! exposes a deterministic fingerprint for the phrase sampler. The state is
//! bounded, order-sensitive, and cheap to reset or replay from recent turns.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryDialogueState {
    lanes: [u64; 4],
    turns: u64,
}

impl Default for BinaryDialogueState {
    fn default() -> Self {
        Self {
            lanes: [
                0x243f6a8885a308d3,
                0x13198a2e03707344,
                0xa4093822299f31d0,
                0x082efa98ec4e6c89,
            ],
            turns: 0,
        }
    }
}

impl BinaryDialogueState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn absorb(&mut self, text: &str) {
        for (position, byte) in text.bytes().enumerate() {
            let position = position as u64;
            for (lane_index, lane) in self.lanes.iter_mut().enumerate() {
                let salt = 0x9e3779b97f4a7c15u64
                    .wrapping_mul((lane_index as u64) + 1)
                    .wrapping_add(position);
                *lane ^= (byte as u64)
                    .wrapping_add(salt)
                    .rotate_left((lane_index * 11 + 7) as u32);
                *lane = lane
                    .wrapping_mul(0x100000001b3)
                    .rotate_left((position as u32 % 31) + 1);
                *lane ^= *lane >> 29;
            }
        }
        self.turns = self.turns.wrapping_add(1);
    }

    pub fn absorb_turn(&mut self, user: &str, assistant: &str) {
        self.absorb("<user>");
        self.absorb(user);
        self.absorb("<assistant>");
        self.absorb(assistant);
    }

    pub fn fingerprint(&self) -> u64 {
        let mixed = self.lanes[0]
            ^ self.lanes[1].rotate_left(13)
            ^ self.lanes[2].rotate_left(29)
            ^ self.lanes[3].rotate_left(47)
            ^ self.turns.wrapping_mul(0x517cc1b727220a95);
        avalanche(mixed)
    }

    pub fn turns(&self) -> u64 {
        self.turns
    }
}

/// Compact typed context layered on top of the recurrent bit lanes. These
/// fields are routing evidence, not facts: they summarize the active domain,
/// uncertainty, evidence posture, and conversation breadth for the next turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedCognitiveState {
    stream: BinaryDialogueState,
    domain_mask: u16,
    active_domain: u8,
    confidence_bin: u8,
    evidence_bin: u8,
    unresolved_bin: u8,
}

impl Default for TypedCognitiveState {
    fn default() -> Self {
        Self {
            stream: BinaryDialogueState::default(),
            domain_mask: 0,
            active_domain: 0,
            confidence_bin: 0,
            evidence_bin: 0,
            unresolved_bin: 0,
        }
    }
}

impl TypedCognitiveState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn observe(&mut self, user: &str, domain: &str, score: i32, overlap: u32) {
        self.stream.absorb(user);
        self.active_domain = domain_index(domain);
        self.domain_mask |= 1u16 << self.active_domain.min(15);
        self.confidence_bin = if score >= 180 {
            3
        } else if score >= 100 {
            2
        } else if score >= 40 {
            1
        } else {
            0
        };
        self.evidence_bin = evidence_bin(user, overlap);
        self.unresolved_bin =
            if contains_any(user, &["unknown", "uncertain", "ambiguous", "what if"]) {
                2
            } else if user.contains('?') {
                1
            } else {
                0
            };
    }

    pub fn absorb_history(&mut self, recent: &[(String, String)]) {
        for (user, assistant) in recent {
            self.stream.absorb_turn(user, assistant);
        }
    }

    pub fn routing_features(&self) -> Vec<String> {
        vec![
            format!("state-domain-{}", domain_name(self.active_domain)),
            format!("state-domains-{:04x}", self.domain_mask),
            format!("state-confidence-{}", self.confidence_bin),
            format!("state-evidence-{}", self.evidence_bin),
            format!("state-unresolved-{}", self.unresolved_bin),
        ]
    }

    pub fn hint(&self) -> String {
        format!(
            "domain={} domains={:04x} confidence={} evidence={} unresolved={} fingerprint={:016x}",
            domain_name(self.active_domain),
            self.domain_mask,
            self.confidence_bin,
            self.evidence_bin,
            self.unresolved_bin,
            self.stream.fingerprint()
        )
    }

    pub fn fingerprint(&self) -> u64 {
        self.stream.fingerprint()
            ^ ((self.domain_mask as u64) << 32)
            ^ ((self.confidence_bin as u64) << 24)
            ^ ((self.evidence_bin as u64) << 16)
            ^ ((self.unresolved_bin as u64) << 8)
            ^ self.active_domain as u64
    }
}

fn evidence_bin(user: &str, overlap: u32) -> u8 {
    let mut score: u8 = if overlap > 8 {
        2
    } else if overlap > 2 {
        1
    } else {
        0
    };
    if contains_any(
        user,
        &["evidence", "source", "test", "measure", "prove", "why"],
    ) {
        score = score.saturating_add(1);
    }
    score.min(3)
}

fn contains_any(text: &str, terms: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    terms.iter().any(|term| lower.contains(term))
}

fn domain_index(domain: &str) -> u8 {
    match domain.to_ascii_lowercase().as_str() {
        "geometry" => 1,
        "logic" => 2,
        "science" => 3,
        "code" => 4,
        "identity" => 5,
        "governance" => 6,
        "planning" => 7,
        "memory" => 8,
        "language" => 9,
        _ => 0,
    }
}

fn domain_name(index: u8) -> &'static str {
    match index {
        1 => "geometry",
        2 => "logic",
        3 => "science",
        4 => "code",
        5 => "identity",
        6 => "governance",
        7 => "planning",
        8 => "memory",
        9 => "language",
        _ => "general",
    }
}

fn avalanche(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_is_deterministic_and_order_sensitive() {
        let mut first = BinaryDialogueState::default();
        first.absorb("geometry");
        first.absorb("life");

        let mut replay = BinaryDialogueState::default();
        replay.absorb("geometry");
        replay.absorb("life");
        assert_eq!(first, replay);

        let mut reversed = BinaryDialogueState::default();
        reversed.absorb("life");
        reversed.absorb("geometry");
        assert_ne!(first.fingerprint(), reversed.fingerprint());
        assert_eq!(first.turns(), 2);
    }

    #[test]
    fn turn_markers_bind_user_and_assistant_channels() {
        let mut one = BinaryDialogueState::default();
        one.absorb_turn("hello", "welcome");
        let mut two = BinaryDialogueState::default();
        two.absorb_turn("welcome", "hello");
        assert_ne!(one.fingerprint(), two.fingerprint());
    }

    #[test]
    fn typed_state_tracks_domain_evidence_and_uncertainty() {
        let mut state = TypedCognitiveState::default();
        state.observe(
            "what evidence would test this geometry claim if it is uncertain?",
            "geometry",
            120,
            10,
        );
        let hint = state.hint();
        assert!(hint.contains("domain=geometry"));
        assert!(hint.contains("confidence=2"));
        assert!(hint.contains("evidence=3"));
        assert!(hint.contains("unresolved=2"));
        assert!(state
            .routing_features()
            .iter()
            .any(|item| item == "state-domain-geometry"));
    }
}
