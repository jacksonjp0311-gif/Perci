//! `perci doctor [--json] [--deep]`: health report for a Perci install.
//!
//! Reports where Perci resolved its home and data folders, whether every
//! packaged weight artifact is present (and not a Git LFS pointer), the
//! optional integrations, log sizes, and the version. Exit code 0 means the
//! required pieces are ok; 2 means a config/weights problem.

use crate::paths;
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{self, Read};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactState {
    Ok,
    Missing,
    LfsPointer,
    SizeMismatch { expected: u64, actual: u64 },
    HashMismatch,
}

impl ArtifactState {
    pub fn label(&self) -> String {
        match self {
            Self::Ok => "ok".into(),
            Self::Missing => "missing".into(),
            Self::LfsPointer => "lfs-pointer (run git lfs pull)".into(),
            Self::SizeMismatch { expected, actual } => {
                format!("size mismatch (expected {expected}, found {actual})")
            }
            Self::HashMismatch => "sha256 mismatch".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArtifactCheck {
    pub path: String,
    pub kind: String,
    pub state: ArtifactState,
    /// Active/required artifacts make doctor fail; legacy ones only warn.
    pub required: bool,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub version: String,
    pub home: PathBuf,
    pub home_source: paths::HomeSource,
    pub data: PathBuf,
    pub artifacts: Vec<ArtifactCheck>,
    pub manifest_found: bool,
    pub personality_found: bool,
    pub cortex: String,
    pub external_lm: String,
    pub external_lm_refused: bool,
    pub tracing: bool,
    pub logs: Vec<(String, u64)>,
    pub python: Option<String>,
    pub ollama: bool,
    pub problems: Vec<String>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.problems.is_empty()
    }
}

fn check_artifact(home: &Path, entry: &Value, deep: bool) -> ArtifactCheck {
    let rel = entry.get("path").and_then(Value::as_str).unwrap_or("");
    let kind = entry
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("artifact");
    let expected = entry.get("bytes").and_then(Value::as_u64);
    let path = home.join(rel);
    let state = match fs::metadata(&path) {
        Err(_) => ArtifactState::Missing,
        Ok(_) if paths::is_lfs_pointer(&path) => ArtifactState::LfsPointer,
        Ok(meta) => match expected {
            Some(e) if e != meta.len() => ArtifactState::SizeMismatch {
                expected: e,
                actual: meta.len(),
            },
            _ => {
                let want = entry.get("sha256").and_then(Value::as_str);
                match (deep, want) {
                    (true, Some(want)) => match sha256_file(&path) {
                        Ok(got) if got.eq_ignore_ascii_case(want) => ArtifactState::Ok,
                        _ => ArtifactState::HashMismatch,
                    },
                    _ => ArtifactState::Ok,
                }
            }
        },
    };
    // The legacy v0.1 and fallback v0.2 packs are optional for chat.
    let required = !kind.starts_with("legacy") && !kind.starts_with("fallback");
    ArtifactCheck {
        path: rel.to_owned(),
        kind: kind.to_owned(),
        state,
        required,
    }
}

fn find_on_path(names: &[&str]) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for name in names {
            for candidate in [dir.join(name), dir.join(format!("{name}.exe"))] {
                if candidate.is_file() {
                    return Some(candidate.display().to_string());
                }
            }
        }
    }
    None
}

fn port_open(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
}

fn dir_logs(dir: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for entry in read.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    out.push((entry.file_name().to_string_lossy().into_owned(), meta.len()));
                }
            }
        }
    }
    out.sort();
    out
}

/// Build the report. `deep` adds SHA-256 verification of every artifact.
pub fn run(deep: bool) -> Report {
    let home = paths::home();
    let home_source = paths::home_source();
    let data = paths::data();
    let mut problems = Vec::new();

    if home_source == paths::HomeSource::Unresolved {
        problems.push(
            "Perci home not found. Set PERCI_HOME to your Perci checkout (the folder with models/)."
                .to_owned(),
        );
    }

    let manifest_path = home.join(paths::HOME_MARKER);
    let manifest: Option<Value> = fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok());
    let artifacts: Vec<ArtifactCheck> = manifest
        .as_ref()
        .and_then(|m| m.get("artifacts"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|e| check_artifact(&home, e, deep))
                .collect()
        })
        .unwrap_or_default();
    if manifest.is_none() && home_source != paths::HomeSource::Unresolved {
        problems.push(format!("missing {}", manifest_path.display()));
    }
    for a in &artifacts {
        if a.required && a.state != ArtifactState::Ok {
            problems.push(format!("{}: {}", a.path, a.state.label()));
        }
    }

    let personality_found = home.join("config/personality.prompt").is_file();

    let cortex = match crate::cortex::CortexBridge::discover() {
        Some(bridge) if bridge.ready() => "attached (bootstrapped)".to_owned(),
        Some(_) => "engine found, not bootstrapped (optional)".to_owned(),
        None => "not installed (optional)".to_owned(),
    };

    let external_lm_refused = false;
    let external_lm = if std::env::var("PERCI_ENABLE_EXTERNAL_LM").as_deref() == Ok("1") {
        format!(
            "enabled ({})",
            std::env::var("PERCI_MODEL_URL").unwrap_or_else(|_| "no PERCI_MODEL_URL".into())
        )
    } else {
        "off".to_owned()
    };
    let tracing = false;
    let logs = dir_logs(&data.join("logs"));
    let python = find_on_path(&["python3", "python", "py"]);
    let ollama = port_open(11434);

    Report {
        version: crate::branding::build_id().to_owned(),
        home,
        home_source,
        data,
        artifacts,
        manifest_found: manifest.is_some(),
        personality_found,
        cortex,
        external_lm,
        external_lm_refused,
        tracing,
        logs,
        python,
        ollama,
        problems,
    }
}

pub fn to_json(r: &Report) -> Value {
    json!({
        "ok": r.ok(),
        "version": r.version,
        "home": r.home.display().to_string(),
        "home_source": r.home_source.label(),
        "data": r.data.display().to_string(),
        "manifest_found": r.manifest_found,
        "artifacts": r.artifacts.iter().map(|a| json!({
            "path": a.path,
            "kind": a.kind,
            "required": a.required,
            "status": a.state.label(),
            "ok": a.state == ArtifactState::Ok,
        })).collect::<Vec<_>>(),
        "personality_found": r.personality_found,
        "cortex": r.cortex,
        "external_lm": r.external_lm,
        "external_lm_refused": r.external_lm_refused,
        "tracing": r.tracing,
        "logs": r.logs.iter().map(|(n, b)| json!({"name": n, "bytes": b})).collect::<Vec<_>>(),
        "python": r.python,
        "ollama": r.ollama,
        "problems": r.problems,
    })
}

pub fn to_text(r: &Report) -> String {
    let mut out = String::new();
    let mark = |ok: bool| if ok { "ok  " } else { "FAIL" };
    out.push_str(&format!("Perci doctor · {}\n\n", r.version));
    out.push_str(&format!(
        "  home   {}  ({})\n  data   {}\n\n",
        r.home.display(),
        r.home_source.label(),
        r.data.display()
    ));
    out.push_str("  weights\n");
    if r.artifacts.is_empty() {
        out.push_str("    (no models/PACKAGE_MANIFEST.json found)\n");
    }
    for a in &r.artifacts {
        let ok = a.state == ArtifactState::Ok;
        let tag = if ok || a.required { mark(ok) } else { "warn" };
        out.push_str(&format!("    {tag} {}  {}\n", a.path, a.state.label()));
    }
    out.push_str(&format!(
        "\n  personality  {}\n  cortex       {}\n  external LM  {}\n  tracing      {}\n",
        if r.personality_found {
            "ok"
        } else {
            "missing (built-in default used)"
        },
        r.cortex,
        r.external_lm,
        if r.tracing {
            "on (PERCI_TRACE=1)"
        } else {
            "off"
        },
    ));
    out.push_str(&format!(
        "  python       {}\n  ollama       {}\n",
        r.python
            .as_deref()
            .unwrap_or("not found (needed later for AgentSearch)"),
        if r.ollama {
            "listening on 127.0.0.1:11434"
        } else {
            "not running (optional)"
        }
    ));
    if !r.logs.is_empty() {
        out.push_str("  logs\n");
        for (name, bytes) in &r.logs {
            out.push_str(&format!("    {name}  {:.1} KiB\n", *bytes as f64 / 1024.0));
        }
    }
    if r.ok() {
        out.push_str("\nAll required checks passed.\n");
    } else {
        out.push_str("\nProblems:\n");
        for p in &r.problems {
            out.push_str(&format!("  - {p}\n"));
        }
        out.push_str(&format!(
            "\nFix: cd \"{}\" && git lfs pull   (or set PERCI_HOME)\n",
            r.home.display()
        ));
    }
    out
}

// ─── SHA-256 (small, dependency-free; used only by --deep) ────────────────────

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

struct Sha256 {
    h: [u32; 8],
    buf: Vec<u8>,
    len: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: Vec::with_capacity(64),
            len: 0,
        }
    }

    fn block(&mut self, chunk: &[u8]) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, v) in self.h.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(v);
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.len += data.len() as u64;
        if !self.buf.is_empty() {
            let need = 64 - self.buf.len();
            let take = need.min(data.len());
            self.buf.extend_from_slice(&data[..take]);
            data = &data[take..];
            if self.buf.len() == 64 {
                let chunk = std::mem::take(&mut self.buf);
                self.block(&chunk);
                self.buf = chunk;
                self.buf.clear();
            }
        }
        let mut chunks = data.chunks_exact(64);
        for chunk in &mut chunks {
            self.block(chunk);
        }
        self.buf.extend_from_slice(chunks.remainder());
    }

    fn finish(mut self) -> String {
        let bit_len = self.len.wrapping_mul(8);
        let mut pad = vec![0x80u8];
        while (self.buf.len() + pad.len()) % 64 != 56 {
            pad.push(0);
        }
        pad.extend_from_slice(&bit_len.to_be_bytes());
        let len = self.len;
        self.update(&pad);
        self.len = len;
        self.h.iter().map(|v| format!("{v:08x}")).collect()
    }
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut s = Sha256::new();
    s.update(data);
    s.finish()
}

pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut s = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        s.update(&buf[..n]);
    }
    Ok(s.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let long = vec![b'a'; 1000];
        assert_eq!(
            sha256_hex(&long),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    #[test]
    fn pointer_artifact_is_reported_with_lfs_hint() {
        let dir = std::env::temp_dir().join(format!("perci-doctor-{}", std::process::id()));
        fs::create_dir_all(dir.join("models")).unwrap();
        fs::write(
            dir.join("models/w.pwgt"),
            "version https://git-lfs.github.com/spec/v1\noid sha256:00\nsize 9\n",
        )
        .unwrap();
        let entry =
            json!({"path": "models/w.pwgt", "kind": "active_associative_weights", "bytes": 9});
        let check = check_artifact(&dir, &entry, false);
        assert_eq!(check.state, ArtifactState::LfsPointer);
        assert!(check.required);
        assert!(check.state.label().contains("git lfs pull"));
        let missing = check_artifact(
            &dir,
            &json!({"path": "models/none.pwgt", "kind": "legacy_associative_weights"}),
            false,
        );
        assert_eq!(missing.state, ArtifactState::Missing);
        assert!(!missing.required);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn doctor_on_this_checkout_reports_real_weights() {
        let report = run(false);
        assert!(report.manifest_found);
        let active = report
            .artifacts
            .iter()
            .find(|a| a.path.ends_with("perci-cognitive-v0.3.pwgt"))
            .expect("v0.3 listed in manifest");
        if std::env::var("PERCI_REQUIRE_WEIGHTS").as_deref() == Ok("1") {
            assert_eq!(active.state, ArtifactState::Ok, "{}", active.state.label());
        }
    }
}
