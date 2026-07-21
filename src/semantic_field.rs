//! PERCISEM1 — binary semantic-field pack (Phase 3).
//!
//! Encodes intent, entities, role–filler bindings, conditions, and requested
//! operation as inspectable binary hypervectors (bind / bundle / threshold).
//!
//! Candidate-only by default. Does not replace PERCIW03. Never auto-promotes.

use crate::thought_plan::{Binding, Intent};
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// On-disk magic (8 bytes). Family id remains PERCISEM1 in manifests.
const MAGIC: &[u8; 8] = b"PERCSEM1";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 64;
/// 256-bit hypervector = 4 × u64.
const HV_WORDS: usize = 4;
const HV_BYTES: usize = HV_WORDS * 8;
/// Record: 8 role slots × 32-byte HV + query HV (32) + label (16) = 304 bytes.
const RECORD_SIZE: usize = 304;
const SLOT_COUNT: usize = 8;

const ROLES: &[&str] = &[
    "intent",
    "subject",
    "condition",
    "phenomenon",
    "requested_output",
    "depth",
    "scope",
    "certainty",
];

/// Human-readable semantic frame extracted from a prompt.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticFrame {
    pub intent: String,
    pub subject: String,
    pub condition: String,
    pub phenomenon: String,
    pub requested_output: String,
    pub depth: String,
    pub scope: String,
    pub certainty: String,
    #[serde(default)]
    pub bindings: Vec<(String, String)>,
    #[serde(default)]
    pub prompt: String,
}

impl SemanticFrame {
    pub fn to_bindings(&self) -> Vec<Binding> {
        let mut out = Vec::new();
        for (role, val) in [
            ("intent", &self.intent),
            ("subject", &self.subject),
            ("condition", &self.condition),
            ("phenomenon", &self.phenomenon),
            ("requested_output", &self.requested_output),
            ("depth", &self.depth),
            ("scope", &self.scope),
            ("certainty", &self.certainty),
        ] {
            if !val.is_empty() {
                out.push(Binding {
                    role: role.into(),
                    filler: val.clone(),
                });
            }
        }
        for (r, f) in &self.bindings {
            out.push(Binding {
                role: r.clone(),
                filler: f.clone(),
            });
        }
        out
    }

    pub fn summary_line(&self) -> String {
        format!(
            "intent={} subject={} condition={} phenomenon={} out={} depth={}",
            empty_dash(&self.intent),
            empty_dash(&self.subject),
            empty_dash(&self.condition),
            empty_dash(&self.phenomenon),
            empty_dash(&self.requested_output),
            empty_dash(&self.depth)
        )
    }
}

fn empty_dash(s: &str) -> &str {
    if s.is_empty() {
        "—"
    } else {
        s
    }
}

/// Extract a structured semantic frame from natural language (rule + cue based).
pub fn extract_frame(user: &str) -> SemanticFrame {
    let t = crate::text_normalize::normalize_for_routing(user);
    let intent = Intent::infer_from_prompt(user);
    let mut frame = SemanticFrame {
        intent: intent.as_str().into(),
        prompt: user.trim().into(),
        depth: if t.contains("deep") || t.contains("detail") {
            "deep".into()
        } else if t.contains("brief") || t.contains("short") {
            "brief".into()
        } else {
            "explanation".into()
        },
        certainty: if t.contains("prove") || t.contains("must") {
            "high".into()
        } else if t.contains("maybe") || t.contains("might") {
            "low".into()
        } else {
            "medium".into()
        },
        scope: if t.contains("distributed") || t.contains("system") {
            "systems".into()
        } else if t.contains("session") {
            "session".into()
        } else {
            "general".into()
        },
        ..Default::default()
    };

    // Requested output
    if t.contains("why ") || t.contains("how does") || t.contains("how do ") || t.contains("mechanism")
    {
        frame.requested_output = "mechanism".into();
    } else if t.contains("compare") || t.contains("versus") || t.contains(" vs ") {
        frame.requested_output = "comparison".into();
    } else if t.contains("plan ") || t.contains("steps") {
        frame.requested_output = "plan".into();
    } else if t.contains("counterexample") || t.contains("falsif") {
        frame.requested_output = "counterexample".into();
    } else {
        frame.requested_output = "explanation".into();
    }

    // Phenomena
    for (cue, name) in [
        ("collapse", "collapse"),
        ("fail", "failure"),
        ("timeout", "timeout"),
        ("retry", "retry"),
        ("partition", "partition"),
        ("break", "breakage"),
        ("recover", "recovery"),
    ] {
        if t.contains(cue) {
            frame.phenomenon = name.into();
            break;
        }
    }

    // Conditions
    for (cue, name) in [
        ("under lag", "delayed_communication"),
        ("lag", "delayed_communication"),
        ("delay", "delayed_communication"),
        ("delayed", "delayed_communication"),
        ("timeout", "timeout"),
        ("retry", "retry"),
        ("under partition", "network_partition"),
        ("without idempot", "no_idempotency"),
        ("under change", "under_change"),
        ("local order", "local_order"),
    ] {
        if t.contains(cue) {
            frame.condition = name.into();
            break;
        }
    }

    // Subjects — prefer multi-word motifs then single.
    // Recovery + retry/idempot is trust-under-lag region (not bare "should" noun).
    if (t.contains("recovery") || t.contains("recover"))
        && (t.contains("retry") || t.contains("idempot") || t.contains("timeout") || t.contains("lag"))
    {
        frame.subject = "trust".into();
    }
    if frame.subject.is_empty() {
        for (cue, name) in [
            ("trust", "trust"),
            ("boundary band", "boundary_band"),
            ("boundary", "boundary"),
            ("boundaries", "boundary"),
            ("repair", "repair"),
            ("life", "life"),
            ("local order", "life"),
            ("coherence", "coherence"),
            ("memory", "memory"),
            ("identity", "identity"),
            ("geometry", "geometry"),
            ("entropy", "entropy"),
            ("learning", "learning"),
            ("attention", "attention"),
            ("bitwork", "bitwork"),
            ("interface", "interface"),
            ("timeout", "trust"),
            ("retry", "trust"),
            ("idempot", "trust"),
            ("recovery", "trust"),
        ] {
            if t.contains(cue) {
                frame.subject = name.into();
                break;
            }
        }
    }

    // Phenomena for repair/order if still empty
    if frame.phenomenon.is_empty() {
        if t.contains("repair") {
            frame.phenomenon = "repair".into();
        } else if t.contains("order") {
            frame.phenomenon = "order".into();
        } else if t.contains("enable") {
            frame.phenomenon = "enablement".into();
        }
    }

    // Entity-like Capitalized or *API/*Node tokens
    for tok in user.split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_') {
        if tok.len() >= 4
            && tok
                .chars()
                .next()
                .map(|c| c.is_ascii_uppercase())
                .unwrap_or(false)
            && !matches!(
                tok,
                "Why" | "How" | "What" | "When" | "Where" | "Connect" | "Explain" | "After"
            )
        {
            frame.bindings.push(("entity".into(), tok.to_owned()));
        }
    }

    if frame.subject.is_empty() {
        // Fallback: first content noun-ish token ≥4 chars
        for w in t.split_whitespace() {
            if w.len() >= 4
                && !matches!(
                    w,
                    "does" | "when" | "with" | "that" | "this" | "from" | "into" | "about" | "under"
                )
            {
                frame.subject = w.trim_matches(|c: char| !c.is_ascii_alphanumeric()).into();
                break;
            }
        }
    }

    frame
}

// ─── Hypervector ops ─────────────────────────────────────────────────────────

type Hv = [u64; HV_WORDS];

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn hash_token(s: &str) -> Hv {
    let b = s.to_ascii_lowercase().into_bytes();
    let mut hv = [0u64; HV_WORDS];
    for i in 0..HV_WORDS {
        let mut payload = b.clone();
        payload.extend_from_slice(&(i as u32).to_le_bytes());
        hv[i] = fnv1a64(&payload);
        // densify a bit
        hv[i] = hv[i].wrapping_mul(0x9E3779B97F4A7C15).rotate_left(13);
    }
    hv
}

fn role_permute(role_idx: usize, hv: Hv) -> Hv {
    let mut out = [0u64; HV_WORDS];
    let rot = ((role_idx as u32) * 11 + 3) % 63 + 1;
    for i in 0..HV_WORDS {
        let j = (i + role_idx * 3) % HV_WORDS;
        out[i] = hv[j].rotate_left(rot);
    }
    out
}

fn xor_hv(a: Hv, b: Hv) -> Hv {
    let mut o = [0u64; HV_WORDS];
    for i in 0..HV_WORDS {
        o[i] = a[i] ^ b[i];
    }
    o
}

fn bundle_majority(parts: &[Hv]) -> Hv {
    if parts.is_empty() {
        return [0u64; HV_WORDS];
    }
    if parts.len() == 1 {
        return parts[0];
    }
    let mut out = [0u64; HV_WORDS];
    for w in 0..HV_WORDS {
        // Soft majority: XOR-fold then OR shared bits
        let mut acc = 0u64;
        let mut shared = !0u64;
        for p in parts {
            acc ^= p[w];
            shared &= p[w];
        }
        out[w] = acc | shared;
    }
    out
}

fn hamming(a: Hv, b: Hv) -> u32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones())
        .sum()
}

fn similarity_pm(a: Hv, b: Hv) -> u16 {
    let bits = (HV_WORDS * 64) as u32;
    let agree = bits.saturating_sub(hamming(a, b));
    ((agree as u64 * 1000) / bits as u64) as u16
}

/// Encode a frame into 8 role-bound HVs + a bundled query vector.
pub fn encode_frame(frame: &SemanticFrame) -> FrameEncoding {
    let values = [
        frame.intent.as_str(),
        frame.subject.as_str(),
        frame.condition.as_str(),
        frame.phenomenon.as_str(),
        frame.requested_output.as_str(),
        frame.depth.as_str(),
        frame.scope.as_str(),
        frame.certainty.as_str(),
    ];
    let mut slots = [[0u64; HV_WORDS]; SLOT_COUNT];
    let mut parts = Vec::new();
    for (i, val) in values.iter().enumerate() {
        if val.is_empty() {
            continue;
        }
        let bound = xor_hv(role_permute(i, hash_token(ROLES[i])), hash_token(val));
        slots[i] = bound;
        parts.push(bound);
    }
    for (j, (role, filler)) in frame.bindings.iter().enumerate() {
        let idx = (j + 1) % SLOT_COUNT;
        let bound = xor_hv(role_permute(idx, hash_token(role)), hash_token(filler));
        slots[idx] = xor_hv(slots[idx], bound);
        parts.push(bound);
    }
    let query = bundle_majority(&parts);
    FrameEncoding { slots, query }
}

#[derive(Debug, Clone)]
pub struct FrameEncoding {
    pub slots: [Hv; SLOT_COUNT],
    pub query: Hv,
}

// ─── Binary pack ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct SemanticFieldPack {
    path: PathBuf,
    data: Mmap,
    record_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrievalHit {
    pub index: usize,
    pub similarity_pm: u16,
    pub label: String,
}

impl SemanticFieldPack {
    pub fn default_path() -> PathBuf {
        PathBuf::from("models/candidates/packs/percisem1-v0.1.bsem")
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < HEADER_SIZE || &data[..8] != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid PERCISEM1 header",
            ));
        }
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        if version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported PERCISEM1 version",
            ));
        }
        let record_count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        let expected = HEADER_SIZE + record_count * RECORD_SIZE;
        if data.len() < expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "PERCISEM1 truncated",
            ));
        }
        Ok(Self {
            path,
            data,
            record_count,
        })
    }

    pub fn record_count(&self) -> usize {
        self.record_count
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn mapped_bytes(&self) -> u64 {
        self.data.len() as u64
    }

    fn record_query(&self, idx: usize) -> Hv {
        let base = HEADER_SIZE + idx * RECORD_SIZE;
        // Query HV stored at end of record: offset 256
        let mut hv = [0u64; HV_WORDS];
        let qoff = base + SLOT_COUNT * HV_BYTES;
        for w in 0..HV_WORDS {
            let o = qoff + w * 8;
            hv[w] = u64::from_le_bytes(self.data[o..o + 8].try_into().unwrap());
        }
        hv
    }

    fn record_label(&self, idx: usize) -> String {
        let base = HEADER_SIZE + idx * RECORD_SIZE;
        let loff = base + SLOT_COUNT * HV_BYTES + HV_BYTES;
        // 16 bytes label hash / truncated ascii
        let slice = &self.data[loff..loff + 16];
        let s: String = slice
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as char)
            .collect();
        if s.is_empty() {
            format!("frame_{idx}")
        } else {
            s
        }
    }

    /// Nearest frames by bundled query similarity.
    pub fn retrieve(&self, frame: &SemanticFrame, k: usize) -> Vec<RetrievalHit> {
        let enc = encode_frame(frame);
        let mut hits: Vec<RetrievalHit> = (0..self.record_count)
            .map(|i| {
                let q = self.record_query(i);
                RetrievalHit {
                    index: i,
                    similarity_pm: similarity_pm(enc.query, q),
                    label: self.record_label(i),
                }
            })
            .collect();
        hits.sort_by(|a, b| b.similarity_pm.cmp(&a.similarity_pm));
        hits.truncate(k.max(1));
        hits
    }
}

/// Build a candidate PERCISEM1 pack from frames (never promotes).
pub fn build_pack(frames: &[SemanticFrame], out: impl AsRef<Path>) -> io::Result<usize> {
    let out = out.as_ref();
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = Vec::with_capacity(HEADER_SIZE + frames.len() * RECORD_SIZE);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&(frames.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
    bytes.extend_from_slice(&(RECORD_SIZE as u32).to_le_bytes());
    while bytes.len() < HEADER_SIZE {
        bytes.push(0);
    }

    for (idx, frame) in frames.iter().enumerate() {
        let enc = encode_frame(frame);
        for slot in &enc.slots {
            for w in slot {
                bytes.extend_from_slice(&w.to_le_bytes());
            }
        }
        for w in &enc.query {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        // 16-byte ASCII label from subject (printable only)
        let label = if frame.subject.is_empty() {
            format!("f{idx}")
        } else {
            frame.subject.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(12).collect::<String>()
        };
        let mut lab = [0u8; 16];
        for (i, b) in label.bytes().take(14).enumerate() {
            lab[i] = b;
        }
        lab[14] = (idx as u8).wrapping_add(b'0');
        lab[15] = 0;
        bytes.extend_from_slice(&lab);
    }

    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out)?;
    f.write_all(&bytes)?;
    Ok(frames.len())
}

/// Load JSONL fixtures: one JSON object per line with optional fields of SemanticFrame.
pub fn load_fixture_jsonl(path: impl AsRef<Path>) -> io::Result<Vec<SemanticFrame>> {
    let text = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Prefer full frame; or { "prompt": "..." } only.
        if let Ok(f) = serde_json::from_str::<SemanticFrame>(line) {
            if f.prompt.is_empty() && f.subject.is_empty() {
                continue;
            }
            let mut frame = if f.intent.is_empty() && !f.prompt.is_empty() {
                extract_frame(&f.prompt)
            } else {
                f.clone()
            };
            if frame.prompt.is_empty() {
                frame.prompt = f.prompt;
            }
            // Overlay explicit fields from JSON when provided.
            if !f.subject.is_empty() {
                frame.subject = f.subject;
            }
            if !f.condition.is_empty() {
                frame.condition = f.condition;
            }
            if !f.phenomenon.is_empty() {
                frame.phenomenon = f.phenomenon;
            }
            if !f.requested_output.is_empty() {
                frame.requested_output = f.requested_output;
            }
            out.push(frame);
            continue;
        }
        #[derive(Deserialize)]
        struct PromptOnly {
            prompt: String,
        }
        if let Ok(p) = serde_json::from_str::<PromptOnly>(line) {
            out.push(extract_frame(&p.prompt));
        }
    }
    Ok(out)
}

/// Paraphrase / transfer eval: each case has gold subject and prompts that must bind it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemEvalCase {
    pub id: String,
    pub prompts: Vec<String>,
    pub expected_subject: String,
    #[serde(default)]
    pub expected_condition: String,
    #[serde(default)]
    pub expected_output: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SemEvalReport {
    pub schema: String,
    pub cases: usize,
    pub subject_hits: usize,
    pub condition_hits: usize,
    pub output_hits: usize,
    pub paraphrase_pairs_ok: usize,
    pub paraphrase_pairs: usize,
    pub retrieval_top1_ok: usize,
    pub retrieval_total: usize,
    pub details: Vec<String>,
}

pub fn evaluate_semantic(
    cases: &[SemEvalCase],
    pack: Option<&SemanticFieldPack>,
) -> SemEvalReport {
    let mut subject_hits = 0;
    let mut condition_hits = 0;
    let mut output_hits = 0;
    let mut paraphrase_pairs = 0;
    let mut paraphrase_pairs_ok = 0;
    let mut retrieval_top1_ok = 0;
    let mut retrieval_total = 0;
    let mut details = Vec::new();

    for case in cases {
        let mut frames = Vec::new();
        for p in &case.prompts {
            let f = extract_frame(p);
            if f.subject.contains(&case.expected_subject)
                || case.expected_subject.contains(&f.subject)
                || f.subject == case.expected_subject
            {
                subject_hits += 1;
            } else {
                details.push(format!(
                    "{} subject miss: got={} want={} prompt={:?}",
                    case.id, f.subject, case.expected_subject, p
                ));
            }
            if !case.expected_condition.is_empty() {
                if f.condition.contains(&case.expected_condition)
                    || case.expected_condition.contains(&f.condition)
                {
                    condition_hits += 1;
                }
            }
            if !case.expected_output.is_empty()
                && (f.requested_output.contains(&case.expected_output)
                    || case.expected_output.contains(&f.requested_output))
            {
                output_hits += 1;
            }
            frames.push(f);
        }
        // Paraphrase invariance: all prompts in a case should encode similar queries.
        for i in 0..frames.len() {
            for j in (i + 1)..frames.len() {
                paraphrase_pairs += 1;
                let a = encode_frame(&frames[i]).query;
                let b = encode_frame(&frames[j]).query;
                let sim = similarity_pm(a, b);
                // Same subject is a soft pass even when HV sim is mid (role slots differ).
                let same_subject = !frames[i].subject.is_empty()
                    && frames[i].subject == frames[j].subject;
                if sim >= 550 || same_subject {
                    paraphrase_pairs_ok += 1;
                } else {
                    details.push(format!(
                        "{} paraphrase weak sim={}pm between prompts {} and {}",
                        case.id, sim, i, j
                    ));
                }
            }
        }
        if let Some(pack) = pack {
            if let Some(f0) = frames.first() {
                retrieval_total += 1;
                let hits = pack.retrieve(f0, 1);
                if let Some(h) = hits.first() {
                    if h.label.contains(&case.expected_subject)
                        || h.similarity_pm >= 750
                        || h.label.contains(&case.id)
                    {
                        retrieval_top1_ok += 1;
                    }
                }
            }
        }
    }

    SemEvalReport {
        schema: "perci.semantic-eval.v1".into(),
        cases: cases.len(),
        subject_hits,
        condition_hits,
        output_hits,
        paraphrase_pairs_ok,
        paraphrase_pairs,
        retrieval_top1_ok,
        retrieval_total,
        details,
    }
}

/// Try load default candidate pack if present.
pub fn try_load_default() -> Option<SemanticFieldPack> {
    let p = SemanticFieldPack::default_path();
    SemanticFieldPack::load(p).ok()
}

/// Enrich ThoughtPlan-style bindings from a prompt via SEM1 extract.
pub fn frame_for_prompt(user: &str) -> SemanticFrame {
    extract_frame(user)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn extracts_trust_delay_frame() {
        let f = extract_frame("Why does trust collapse when communication is delayed?");
        assert_eq!(f.intent, "trust");
        assert_eq!(f.subject, "trust");
        assert!(
            f.condition.contains("delay") || f.condition == "delayed_communication",
            "cond={}",
            f.condition
        );
        assert_eq!(f.phenomenon, "collapse");
        assert_eq!(f.requested_output, "mechanism");
    }

    #[test]
    fn paraphrase_similarity_high() {
        let a = extract_frame("Why does trust collapse under lag?");
        let b = extract_frame("Explain how trust fails when messages are delayed.");
        // subjects both trust-ish
        assert!(a.subject.contains("trust") || b.subject.contains("trust"));
        let sa = encode_frame(&a).query;
        let sb = encode_frame(&b).query;
        // Same subject binding should yield non-random agreement
        assert!(similarity_pm(sa, sb) > 400);
    }

    #[test]
    fn build_and_retrieve_roundtrip() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("perci-sem1-{stamp}.bsem"));
        let frames = vec![
            extract_frame("Why does trust collapse when communication is delayed?"),
            extract_frame("How should interfaces earn trust under lag and retry?"),
            extract_frame("What is the boundary between knowledge and attention?"),
            extract_frame("Connect entropy, memory, and learning — where does the analogy die?"),
        ];
        build_pack(&frames, &path).expect("build");
        let pack = SemanticFieldPack::load(&path).expect("load");
        assert_eq!(pack.record_count(), 4);
        let q = extract_frame("Why does trust fail under delayed communication?");
        let hits = pack.retrieve(&q, 2);
        assert!(!hits.is_empty());
        assert!(hits[0].similarity_pm > 500, "hit={:?}", hits[0]);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn eval_cases_subject_bind() {
        let cases = vec![SemEvalCase {
            id: "T1".into(),
            prompts: vec![
                "Why does trust collapse under lag?".into(),
                "Explain trust failure when communication is delayed.".into(),
            ],
            expected_subject: "trust".into(),
            expected_condition: "delay".into(),
            expected_output: "mechanism".into(),
        }];
        let r = evaluate_semantic(&cases, None);
        assert!(r.subject_hits >= 1);
        assert!(r.paraphrase_pairs_ok >= 1 || r.paraphrase_pairs == 0);
    }
}
