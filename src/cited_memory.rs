//! Cited memory: recall with evidence hashes, backed by AgentSearch.
//!
//! **Source of truth** is still the append-only journal written by
//! [`MemoryStore`] (`PERCI_DATA/memory/perci.jsonl`). This module derives one
//! immutable Markdown note per journal record under `memory/notes/` and lets
//! AgentSearch index that folder. AgentSearch hashes *whole files*, so a note
//! per record keeps every citation's SHA-256 stable while the journal grows.
//!
//! A [`Citation`] names the note (path, line, `sha256` of the note file) *and*
//! the journal (path, 1-based line, byte offset, `sha256` of the raw record
//! line), so a harness can check both "is this the file I was shown?" and
//! "does it still say what the journal says?" ([`CitedMemory::verify`]).
//!
//! Notes are derived data: deleting `memory/notes/` and `memory/agentsearch/`
//! loses nothing, they are rebuilt from the journal on the next call. A note
//! that was edited by hand is never overwritten; `verify` reports it.
//!
//! When AgentSearch is unavailable (no Python 3.11+, package not installed,
//! `PERCI_AGENTSEARCH=off`, or the child dies) recall falls back to a
//! substring scan of the journal and still returns hashed citations, with
//! `backend: "substring"` and a warning.
//!
//! Recalled text is user data, never instructions.

use crate::agentsearch::AgentSearchClient;
use crate::doctor::sha256_hex;
use crate::memory::{parse_record, search_terms, MemoryRecord, MemoryStore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const SCHEMA: &str = "perci.recall.v1";
const NOTE_HEADER_LINES: u64 = 9;
const MAX_TERMS: usize = 4;

/// Evidence for one recalled memory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    /// Absolute path of the note file.
    pub note_path: String,
    /// 1-based line inside the note where the match is.
    pub note_line: u64,
    /// SHA-256 of the whole note file when it was cited.
    pub sha256: String,
    /// Journal (source of truth) path.
    pub journal_path: String,
    /// 1-based line of the record in the journal.
    pub journal_line: u64,
    /// Byte offset of the record's first byte in the journal.
    pub journal_offset: u64,
    /// SHA-256 of the raw journal line (without the newline).
    pub record_sha256: String,
}

/// One recalled memory with its citation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecallHit {
    pub text: String,
    pub kind: String,
    /// Unix seconds; 0 for legacy records without a timestamp.
    pub created_unix: u64,
    /// Number of distinct query terms found in this note.
    pub terms_matched: usize,
    pub citation: Citation,
    /// AgentSearch saw the file change after it was indexed.
    pub changed_since_index: bool,
}

/// Result of [`CitedMemory::recall`] (serialises as `perci.recall.v1`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecallResult {
    pub schema: String,
    pub query: String,
    pub terms: Vec<String>,
    /// `"agentsearch"` or `"substring"`.
    pub backend: String,
    pub hits: Vec<RecallHit>,
    /// False when a search reported partial coverage: absence is then not proof.
    pub complete: bool,
    /// Older matches existed beyond the per-term window and were not ranked.
    #[serde(default)]
    pub truncated: bool,
    pub warnings: Vec<String>,
    pub elapsed_us: u64,
}

/// Receipt for [`CitedMemory::remember`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RememberReceipt {
    pub status: String,
    pub citation: Citation,
}

/// Outcome of [`CitedMemory::verify`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifyResult {
    /// The note file still hashes to the cited `sha256`.
    pub note_unchanged: bool,
    /// The journal still holds the cited record at the cited offset.
    pub journal_matches: bool,
    /// The note text equals what the journal record says.
    pub note_matches_journal: bool,
    pub ok: bool,
    pub detail: String,
}

#[derive(Default, Serialize, Deserialize)]
struct SyncState {
    journal_len: u64,
    next_line: u64,
    /// SHA-256 of the first record line; a different value means the journal
    /// was replaced, so everything is re-derived.
    head_sha256: String,
}

/// A journal record with its position.
#[derive(Clone, Debug)]
struct Located {
    line: u64,
    offset: u64,
    raw_sha256: String,
    record: MemoryRecord,
}

enum Backend {
    NotStarted,
    Ready(Box<AgentSearchClient>),
    Unavailable(String),
}

/// Cited recall over the append-only memory journal.
pub struct CitedMemory {
    store: MemoryStore,
    notes: PathBuf,
    index_dir: PathBuf,
    backend: Backend,
    /// Notes were added since AgentSearch last refreshed its index.
    dirty: bool,
    /// Parsed journal records, extended incrementally (append-only journal).
    cache: Vec<Located>,
    /// Journal bytes covered by `cache` (always ends on a line boundary).
    cache_len: u64,
    cache_lines: u64,
    cache_head: String,
    /// Persisted watermark: notes exist for every record before this offset.
    state: Option<SyncState>,
}

impl CitedMemory {
    /// Use the journal at `journal` (notes and index live next to it).
    pub fn new(journal: impl Into<PathBuf>) -> Self {
        let journal = journal.into();
        let dir = journal
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            store: MemoryStore::new(journal),
            notes: dir.join("notes"),
            index_dir: dir.join("agentsearch"),
            backend: Backend::NotStarted,
            dirty: true,
            cache: Vec::new(),
            cache_len: 0,
            cache_lines: 0,
            cache_head: String::new(),
            state: None,
        }
    }

    /// `PERCI_MEMORY`, else `PERCI_DATA/memory/perci.jsonl`.
    pub fn open_default() -> Self {
        Self::new(default_journal())
    }

    pub fn journal_path(&self) -> &Path {
        self.store.path()
    }

    pub fn notes_dir(&self) -> &Path {
        &self.notes
    }

    /// Never start AgentSearch for this instance (substring recall only).
    pub fn without_agentsearch(mut self) -> Self {
        self.backend = Backend::Unavailable("disabled for this instance".into());
        self
    }

    /// `"agentsearch"`, `"substring"` or `"not started"`.
    pub fn backend_name(&self) -> &'static str {
        match self.backend {
            Backend::Ready(_) => "agentsearch",
            Backend::Unavailable(_) => "substring",
            Backend::NotStarted => "not started",
        }
    }

    /// Append to the journal (the source of truth), then derive its note.
    pub fn remember(&mut self, kind: &str, text: &str) -> io::Result<RememberReceipt> {
        let offset = fs::metadata(self.store.path())
            .map(|m| m.len())
            .unwrap_or(0);
        self.store.append_kind(kind, text)?;
        self.sync()?;
        let located = self
            .cache
            .iter()
            .rev()
            .take_while(|r| r.offset >= offset)
            .last()
            .cloned()
            .ok_or_else(|| io::Error::other("record not found after append"))?;
        let note = self.note_path(&located);
        let citation = self.citation_for(&located, &note, NOTE_HEADER_LINES + 1, None)?;
        Ok(RememberReceipt {
            status: "stored".into(),
            citation,
        })
    }

    /// Derive any missing note files from the journal. Cheap when nothing is new:
    /// only bytes appended since the last call are read.
    pub fn sync(&mut self) -> io::Result<usize> {
        self.refresh_cache()?;
        let state_path = self.index_dir.join("sync.json");
        let mut state = match self.state.take() {
            Some(state) => state,
            None => fs::read_to_string(&state_path)
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default(),
        };
        if state.journal_len > self.cache_len || state.head_sha256 != self.cache_head {
            // Journal replaced or truncated: re-derive (existing notes are kept).
            state = SyncState {
                head_sha256: self.cache_head.clone(),
                ..SyncState::default()
            };
        }
        if state.journal_len == self.cache_len {
            self.state = Some(state);
            return Ok(0);
        }
        let mut written = 0;
        let start = self.cache.partition_point(|r| r.offset < state.journal_len);
        for located in &self.cache[start..] {
            if self.write_note(located)? {
                written += 1;
            }
        }
        state.journal_len = self.cache_len;
        state.next_line = self.cache_lines;
        fs::create_dir_all(&self.index_dir)?;
        fs::write(
            &state_path,
            serde_json::to_string(&state).map_err(io::Error::other)?,
        )?;
        self.state = Some(state);
        if written > 0 {
            self.dirty = true;
        }
        Ok(written)
    }

    /// Extend the in-memory record cache with lines appended since last time.
    fn refresh_cache(&mut self) -> io::Result<()> {
        let journal = self.store.path().to_path_buf();
        let len = match fs::metadata(&journal) {
            Ok(m) => m.len(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                self.cache.clear();
                self.cache_len = 0;
                self.cache_lines = 0;
                self.cache_head.clear();
                return Ok(());
            }
            Err(e) => return Err(e),
        };
        if len == self.cache_len && len > 0 {
            return Ok(());
        }
        let head = first_line_sha(&journal)?;
        if len < self.cache_len || head != self.cache_head {
            self.cache.clear();
            self.cache_len = 0;
            self.cache_lines = 0;
            self.cache_head = head;
        }
        let mut file = fs::File::open(&journal)?;
        file.seek(SeekFrom::Start(self.cache_len))?;
        let mut tail = Vec::new();
        file.take(len - self.cache_len).read_to_end(&mut tail)?;
        // Only whole lines; a half-written trailing line waits for the next call.
        let complete = tail.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        let mut offset = self.cache_len;
        for raw in tail[..complete].split_inclusive(|b| *b == b'\n') {
            self.cache_lines += 1;
            let body = trim_newline(raw);
            if let Some(record) = std::str::from_utf8(body).ok().and_then(parse_record) {
                self.cache.push(Located {
                    line: self.cache_lines,
                    offset,
                    raw_sha256: sha256_hex(body),
                    record,
                });
            }
            offset += raw.len() as u64;
        }
        self.cache_len = offset;
        Ok(())
    }

    /// Find notes matching `query`, best first, each with a citation.
    ///
    /// AgentSearch is literal, so this runs one grep per term (at most four)
    /// and ranks notes by distinct terms hit, then recency. Each grep looks at
    /// the newest `3 × limit` (8..=60) matching notes; `truncated` says older
    /// matches existed beyond that window.
    pub fn recall(&mut self, query: &str, limit: usize) -> io::Result<RecallResult> {
        let started = Instant::now();
        self.sync()?;
        let terms = recall_terms(query);
        let limit = limit.clamp(1, 50);
        let window = (limit * 3).clamp(8, 60) as u32;
        let mut warnings = Vec::new();
        self.ensure_backend();
        let mut found = None;
        if let Backend::Ready(client) = &mut self.backend {
            match agentsearch_recall(client, &self.notes, &terms, window, self.dirty) {
                Ok(result) => {
                    self.dirty = false;
                    found = Some(result);
                }
                Err(e) => {
                    let why = format!("AgentSearch failed ({e}); fell back to substring recall");
                    warnings.push(why.clone());
                    self.backend = Backend::Unavailable(why);
                }
            }
        } else if let Backend::Unavailable(why) = &self.backend {
            warnings.push(format!("substring recall: {why}"));
        }
        // (terms hit, record, note line, sha256 from AgentSearch, changed since index)
        let mut scored: Vec<(usize, Located, u64, Option<String>, bool)> = Vec::new();
        let (backend, complete, truncated) = match found {
            Some(result) => {
                warnings.extend(result.warnings);
                for (line, h) in result.hits {
                    if let Some(located) = self.record_at_line(line) {
                        scored.push((
                            h.terms,
                            located.clone(),
                            h.note_line,
                            Some(h.sha256),
                            h.changed,
                        ));
                    }
                }
                ("agentsearch", result.complete, result.truncated)
            }
            None => {
                for r in &self.cache {
                    let lower = r.record.text.to_lowercase();
                    let n = if terms.is_empty() {
                        1
                    } else {
                        terms.iter().filter(|t| lower.contains(t.as_str())).count()
                    };
                    if n > 0 {
                        scored.push((n, r.clone(), NOTE_HEADER_LINES + 1, None, false));
                    }
                }
                ("substring", true, false)
            }
        };
        // Most terms first, then newest.
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.line.cmp(&a.1.line)));
        let mut hits = Vec::new();
        for (n, located, note_line, sha, changed) in scored.into_iter().take(limit) {
            let note = self.note_path(&located);
            let citation = self.citation_for(&located, &note, note_line, sha)?;
            hits.push(RecallHit {
                text: located.record.text.clone(),
                kind: located.record.kind.clone(),
                created_unix: located.record.timestamp_unix,
                terms_matched: n,
                citation,
                changed_since_index: changed,
            });
        }
        Ok(RecallResult {
            schema: SCHEMA.into(),
            query: query.to_owned(),
            terms,
            backend: backend.into(),
            hits,
            complete,
            truncated,
            warnings,
            elapsed_us: started.elapsed().as_micros() as u64,
        })
    }

    fn record_at_line(&self, line: u64) -> Option<&Located> {
        self.cache
            .binary_search_by_key(&line, |r| r.line)
            .ok()
            .map(|i| &self.cache[i])
    }

    /// Re-check a citation against the note file and the journal.
    pub fn verify(&mut self, citation: &Citation) -> io::Result<VerifyResult> {
        let note = PathBuf::from(&citation.note_path);
        let note_bytes = fs::read(&note).ok();
        let current = note_bytes.as_deref().map(sha256_hex);
        // Prefer AgentSearch's own verify_evidence when it is running.
        let note_unchanged = match (&mut self.backend, &current) {
            (Backend::Ready(client), Some(_)) => client
                .verify(&note, &citation.sha256)
                .unwrap_or_else(|_| current.as_deref() == Some(citation.sha256.as_str())),
            _ => current.as_deref() == Some(citation.sha256.as_str()),
        };
        let journal_line =
            read_line_at(Path::new(&citation.journal_path), citation.journal_offset)?;
        let journal_matches = journal_line
            .as_deref()
            .map(sha256_hex)
            .is_some_and(|h| h == citation.record_sha256);
        let note_matches_journal = match (&note_bytes, journal_line.as_deref()) {
            (Some(bytes), Some(raw)) => std::str::from_utf8(raw)
                .ok()
                .and_then(parse_record)
                .map(|record| {
                    let located = Located {
                        line: citation.journal_line,
                        offset: citation.journal_offset,
                        raw_sha256: citation.record_sha256.clone(),
                        record,
                    };
                    render_note(&located, &citation.journal_path).as_bytes() == bytes.as_slice()
                })
                .unwrap_or(false),
            _ => false,
        };
        let ok = note_unchanged && journal_matches && note_matches_journal;
        let detail = if ok {
            "note and journal match the citation".to_owned()
        } else if current.is_none() {
            "note file is missing".to_owned()
        } else if !journal_matches {
            "journal no longer holds the cited record at that offset".to_owned()
        } else if !note_unchanged {
            "note file changed since it was cited".to_owned()
        } else {
            "note file no longer matches its journal record".to_owned()
        };
        Ok(VerifyResult {
            note_unchanged,
            journal_matches,
            note_matches_journal,
            ok,
            detail,
        })
    }

    /// Kill the AgentSearch child, if running (tests: simulate a crash).
    #[doc(hidden)]
    pub fn kill_backend(&mut self) {
        if let Backend::Ready(client) = &mut self.backend {
            client.kill();
        }
    }

    /// Start AgentSearch now instead of on the first recall. Returns the backend name.
    pub fn warm_up(&mut self) -> &'static str {
        self.ensure_backend();
        self.backend_name()
    }

    /// AgentSearch version when it is running.
    pub fn agentsearch_version(&self) -> Option<&str> {
        match &self.backend {
            Backend::Ready(client) => client.version(),
            _ => None,
        }
    }

    /// Why AgentSearch is not in use, if it is not.
    pub fn unavailable_reason(&self) -> Option<&str> {
        match &self.backend {
            Backend::Unavailable(why) => Some(why),
            _ => None,
        }
    }

    fn ensure_backend(&mut self) {
        if !matches!(self.backend, Backend::NotStarted) {
            return;
        }
        let config = self.index_dir.join("config.json");
        let db = self.index_dir.join("index.sqlite3");
        self.backend = match fs::create_dir_all(&self.notes)
            .and_then(|_| AgentSearchClient::spawn(&config, &db, std::slice::from_ref(&self.notes)))
        {
            Ok(client) => {
                self.dirty = true;
                Backend::Ready(Box::new(client))
            }
            Err(e) => Backend::Unavailable(format!("AgentSearch unavailable: {e}")),
        };
    }

    fn note_path(&self, located: &Located) -> PathBuf {
        self.notes.join(note_file_name(located))
    }

    /// Create the note if it does not exist. Never overwrites an existing file.
    fn write_note(&self, located: &Located) -> io::Result<bool> {
        fs::create_dir_all(&self.notes)?;
        let path = self.note_path(located);
        let body = render_note(located, &self.store.path().to_string_lossy());
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(body.as_bytes())?;
                file.sync_all()?;
                Ok(true)
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn citation_for(
        &self,
        located: &Located,
        note: &Path,
        note_line: u64,
        sha256: Option<String>,
    ) -> io::Result<Citation> {
        let sha256 = match sha256 {
            Some(sha) if sha.len() == 64 => sha,
            _ => fs::read(note).map(|b| sha256_hex(&b)).unwrap_or_default(),
        };
        Ok(Citation {
            note_path: note.to_string_lossy().into_owned(),
            note_line,
            sha256,
            journal_path: self.store.path().to_string_lossy().into_owned(),
            journal_line: located.line,
            journal_offset: located.offset,
            record_sha256: located.raw_sha256.clone(),
        })
    }
}

/// `PERCI_MEMORY`, else `PERCI_DATA/memory/perci.jsonl`.
pub fn default_journal() -> PathBuf {
    std::env::var_os("PERCI_MEMORY")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::paths::data_file("memory/perci.jsonl"))
}

/// Short human tag for a citation: `note 000012 · sha256:3fa2c1d4e5f6`.
pub fn short_tag(c: &Citation) -> String {
    format!(
        "note {:06} · sha256:{}",
        c.journal_line,
        c.sha256.get(..12).unwrap_or(&c.sha256)
    )
}

/// `2026-10-17 14:31 UTC` (or `undated` for legacy records).
pub fn format_saved_at(unix: u64) -> String {
    if unix == 0 {
        return "undated".into();
    }
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    // Civil-from-days (Howard Hinnant), proleptic Gregorian, UTC.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "saved {year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        secs / 3600,
        (secs % 3600) / 60
    )
}

/// `perci memory <remember|recall|verify|status|sync> …`. Returns the exit code
/// (0 ok, 1 verify failed / nothing found with `--strict`, 2 usage).
pub fn run_cli(args: &[String]) -> io::Result<i32> {
    let json = args.iter().any(|a| a == "--json");
    let mut rest: Vec<&str> = Vec::new();
    let mut limit = 5usize;
    let mut kind = "note".to_owned();
    let mut iter = args.iter().filter(|a| a.as_str() != "--json");
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--limit" => limit = iter.next().and_then(|v| v.parse().ok()).unwrap_or(limit),
            "--kind" => kind = iter.next().cloned().unwrap_or(kind),
            other => rest.push(other),
        }
    }
    let Some((&sub, words)) = rest.split_first() else {
        eprintln!("{CLI_USAGE}");
        return Ok(2);
    };
    let text = words.join(" ");
    let mut mem = CitedMemory::open_default();
    let print = |value: &dyn erased::Json, human: String| {
        if json {
            println!("{}", value.to_json());
        } else {
            println!("{human}");
        }
    };
    match sub {
        "remember" | "add" => {
            if text.trim().is_empty() {
                eprintln!("{CLI_USAGE}");
                return Ok(2);
            }
            let r = mem.remember(&kind, &text)?;
            let human = format!(
                "Saved. [{}]\n  {}",
                short_tag(&r.citation),
                r.citation.note_path
            );
            print(&r, human);
            Ok(0)
        }
        "recall" | "search" => {
            let r = mem.recall(&text, limit)?;
            let mut human = String::new();
            if r.hits.is_empty() {
                human.push_str("No matching notes.");
            }
            for hit in &r.hits {
                human.push_str(&format!(
                    "- \"{}\" ({}) [{}]\n    {}:{}  journal line {} @ byte {}\n",
                    hit.text,
                    format_saved_at(hit.created_unix),
                    short_tag(&hit.citation),
                    hit.citation.note_path,
                    hit.citation.note_line,
                    hit.citation.journal_line,
                    hit.citation.journal_offset
                ));
            }
            human.push_str(&format!(
                "backend: {} · {} µs{}",
                r.backend,
                r.elapsed_us,
                if r.complete {
                    ""
                } else {
                    " · partial coverage"
                }
            ));
            for w in &r.warnings {
                human.push_str(&format!("\nnote: {w}"));
            }
            print(&r, human);
            Ok(0)
        }
        "verify" => {
            let raw = if text.trim().is_empty() {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                text
            };
            let value: serde_json::Value =
                serde_json::from_str(raw.trim()).map_err(io::Error::other)?;
            // Accept a bare citation, a recall hit, or a remember receipt.
            let citation_value = value.get("citation").cloned().unwrap_or(value);
            let citation: Citation =
                serde_json::from_value(citation_value).map_err(io::Error::other)?;
            let v = mem.verify(&citation)?;
            let human = format!(
                "{}: {}",
                if v.ok { "verified" } else { "NOT verified" },
                v.detail
            );
            let code = i32::from(!v.ok);
            print(&v, human);
            Ok(code)
        }
        "sync" => {
            let n = mem.sync()?;
            println!("{n} new note file(s) in {}", mem.notes_dir().display());
            Ok(0)
        }
        "status" => {
            let written = mem.sync()?;
            let backend = mem.warm_up();
            let notes = fs::read_dir(mem.notes_dir())
                .map(|d| d.count())
                .unwrap_or(0);
            let status = serde_json::json!({
                "journal": mem.journal_path(),
                "notes_dir": mem.notes_dir(),
                "notes": notes,
                "new_notes": written,
                "backend": backend,
                "agentsearch_version": mem.agentsearch_version(),
                "reason": mem.unavailable_reason(),
            });
            let human = format!(
                "journal: {}\nnotes: {} in {}\nrecall backend: {}{}",
                mem.journal_path().display(),
                notes,
                mem.notes_dir().display(),
                backend,
                mem.unavailable_reason()
                    .map(|r| format!(" ({r})"))
                    .unwrap_or_default()
            );
            print(&status, human);
            Ok(0)
        }
        _ => {
            eprintln!("{CLI_USAGE}");
            Ok(2)
        }
    }
}

const CLI_USAGE: &str = "usage: perci memory remember <text> [--kind K] [--json]\n       perci memory recall <query> [--limit N] [--json]\n       perci memory verify '<citation json>'   (or JSON on stdin)\n       perci memory status [--json] | sync";

mod erased {
    pub trait Json {
        fn to_json(&self) -> String;
    }
    impl<T: serde::Serialize> Json for T {
        fn to_json(&self) -> String {
            serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
        }
    }
}

/// Query terms for literal search: the memory stop list plus filler words,
/// longest first, at most four (AgentSearch is literal, so each term is one grep).
pub fn recall_terms(query: &str) -> Vec<String> {
    const EXTRA_STOP: &[&str] = &[
        "about",
        "tell",
        "told",
        "said",
        "say",
        "where",
        "when",
        "which",
        "who",
        "how",
        "was",
        "were",
        "are",
        "is",
        "my",
        "me",
        "your",
        "it",
        "of",
        "to",
        "in",
        "on",
        "for",
        "and",
        "do",
        "does",
        "know",
        "anything",
        "something",
        "note",
        "notes",
        "saved",
        "please",
        "any",
    ];
    let mut terms: Vec<String> = search_terms(query)
        .into_iter()
        .filter(|t| t.len() >= 3 && !EXTRA_STOP.contains(&t.as_str()))
        .collect();
    terms.dedup();
    let mut seen = std::collections::BTreeSet::new();
    terms.retain(|t| seen.insert(t.clone()));
    terms.sort_by_key(|t| std::cmp::Reverse(t.len()));
    terms.truncate(MAX_TERMS);
    terms
}

struct AsHit {
    terms: usize,
    note_line: u64,
    sha256: String,
    changed: bool,
}

struct AsRecall {
    complete: bool,
    truncated: bool,
    warnings: Vec<String>,
    /// Keyed by journal line (parsed from the note file name).
    hits: Vec<(u64, AsHit)>,
}

/// One literal grep per term, merged by note. Notes are named so that path
/// order (AgentSearch's result order) is newest first.
fn agentsearch_recall(
    client: &mut AgentSearchClient,
    notes: &Path,
    terms: &[String],
    window: u32,
    fresh: bool,
) -> io::Result<AsRecall> {
    let mut merged: BTreeMap<u64, AsHit> = BTreeMap::new();
    let mut complete = true;
    let mut truncated = false;
    let mut warnings = Vec::new();
    // No usable terms: list recent notes through a pattern every note has.
    let patterns: Vec<String> = if terms.is_empty() {
        vec!["perci_note:".to_owned()]
    } else {
        terms.to_vec()
    };
    for (i, term) in patterns.iter().enumerate() {
        let found = client.grep(term, notes, window, fresh && i == 0)?;
        if found.files.len() >= window as usize {
            // AgentSearch marks a result cut at `limit` as incomplete; with
            // newest-first names that only means older matches were skipped.
            truncated = true;
        } else {
            complete &= found.complete;
        }
        warnings.extend(found.warnings);
        for file in found.files {
            let Some(line) = Path::new(&file.path)
                .file_name()
                .and_then(|n| note_line_from_name(&n.to_string_lossy()))
            else {
                continue;
            };
            // Only matches in the note body count (not the header).
            let Some(m) = file
                .matches
                .iter()
                .find(|m| m.line > NOTE_HEADER_LINES || terms.is_empty())
            else {
                continue;
            };
            let entry = merged.entry(line).or_insert(AsHit {
                terms: 0,
                note_line: m.line.max(NOTE_HEADER_LINES + 1),
                sha256: file.content_sha256.clone().unwrap_or_default(),
                changed: false,
            });
            entry.terms += 1;
            entry.changed |= file.changed_since_index;
        }
    }
    Ok(AsRecall {
        complete,
        truncated,
        warnings,
        hits: merged.into_iter().collect(),
    })
}

/// Journal line encoded in a note file name (see [`note_file_name`]).
fn note_line_from_name(name: &str) -> Option<u64> {
    let rest = name.split_once("-L")?.1;
    rest.split_once('-')?.0.parse().ok()
}

fn note_file_name(located: &Located) -> String {
    let kind: String = located
        .record
        .kind
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(24)
        .collect();
    let kind = if kind.is_empty() { "note".into() } else { kind };
    // `9999999999 - line` first, so a path-ordered listing is newest first.
    format!(
        "{:010}-L{:06}-{kind}.md",
        9_999_999_999u64.saturating_sub(located.line),
        located.line
    )
}

/// Note layout: a fixed 9-line header, then the record text from line 10.
fn render_note(located: &Located, journal: &str) -> String {
    let journal_name = Path::new(journal)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!(
        "---\nperci_note: 1\nkind: {}\ncreated_unix: {}\njournal: {}\njournal_line: {}\njournal_offset: {}\nrecord_sha256: {}\n---\n{}\n",
        located.record.kind.replace('\n', " "),
        located.record.timestamp_unix,
        journal_name,
        located.line,
        located.offset,
        located.raw_sha256,
        located.record.text
    )
}

fn trim_newline(raw: &[u8]) -> &[u8] {
    let raw = raw.strip_suffix(b"\n").unwrap_or(raw);
    raw.strip_suffix(b"\r").unwrap_or(raw)
}

fn first_line_sha(path: &Path) -> io::Result<String> {
    Ok(read_line_at(path, 0)?
        .map(|l| sha256_hex(&l))
        .unwrap_or_default())
}

/// The raw line (without newline) that starts at `offset`, if it is complete.
fn read_line_at(path: &Path, offset: u64) -> io::Result<Option<Vec<u8>>> {
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if offset >= file.metadata()?.len() {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut reader = io::BufReader::new(file.take(1 << 20));
    let mut line = Vec::new();
    io::BufRead::read_until(&mut reader, b'\n', &mut line)?;
    if line.last() != Some(&b'\n') {
        return Ok(None);
    }
    Ok(Some(trim_newline(&line).to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_journal(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "perci-cited-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join("perci.jsonl")
    }

    #[test]
    fn remember_derives_an_immutable_note_with_matching_hashes() {
        let journal = temp_journal("remember");
        let mut mem = CitedMemory::new(&journal).without_agentsearch();
        mem.remember("note", "dentist moved to Tuesday 3pm")
            .unwrap();
        let r = mem
            .remember("note", "the backup drive is in the left drawer")
            .unwrap();
        let c = &r.citation;
        assert_eq!(c.journal_line, 2);
        assert!(c.journal_offset > 0);
        let note = fs::read(&c.note_path).unwrap();
        assert_eq!(sha256_hex(&note), c.sha256);
        let raw = read_line_at(&journal, c.journal_offset).unwrap().unwrap();
        assert_eq!(sha256_hex(&raw), c.record_sha256);
        let text = String::from_utf8(note).unwrap();
        assert_eq!(
            text.lines().nth(NOTE_HEADER_LINES as usize).unwrap(),
            "the backup drive is in the left drawer"
        );
        assert!(mem.verify(c).unwrap().ok);
    }

    #[test]
    fn substring_recall_cites_and_ranks_by_terms_then_recency() {
        let journal = temp_journal("recall");
        let mut mem = CitedMemory::new(&journal).without_agentsearch();
        mem.remember("note", "backup drive is the grey one")
            .unwrap();
        mem.remember("note", "dentist moved to Tuesday").unwrap();
        mem.remember("note", "the backup drive is in the left drawer")
            .unwrap();
        let r = mem.recall("where is the backup drive", 5).unwrap();
        assert_eq!(r.schema, SCHEMA);
        assert_eq!(r.backend, "substring");
        assert_eq!(r.hits.len(), 2);
        assert_eq!(r.hits[0].text, "the backup drive is in the left drawer");
        assert_eq!(r.hits[0].citation.journal_line, 3);
        for hit in &r.hits {
            let bytes = fs::read(&hit.citation.note_path).unwrap();
            assert_eq!(sha256_hex(&bytes), hit.citation.sha256);
        }
    }

    #[test]
    fn edited_note_is_detected_and_never_overwritten() {
        let journal = temp_journal("edit");
        let mut mem = CitedMemory::new(&journal).without_agentsearch();
        let c = mem
            .remember("note", "wifi password hint is the dog's name")
            .unwrap()
            .citation;
        fs::write(&c.note_path, "---\ntampered\n").unwrap();
        let v = mem.verify(&c).unwrap();
        assert!(!v.ok && !v.note_unchanged && v.journal_matches);
        // A later sync must not silently "repair" the evidence of tampering.
        mem.remember("note", "another note").unwrap();
        assert_eq!(fs::read_to_string(&c.note_path).unwrap(), "---\ntampered\n");
    }

    #[test]
    fn notes_are_rebuilt_from_the_journal_after_deletion() {
        let journal = temp_journal("rebuild");
        let mut mem = CitedMemory::new(&journal).without_agentsearch();
        let first = mem.remember("note", "alpha bravo").unwrap().citation;
        fs::remove_dir_all(mem.notes_dir()).unwrap();
        fs::remove_dir_all(journal.parent().unwrap().join("agentsearch")).unwrap();
        let mut again = CitedMemory::new(&journal).without_agentsearch();
        let r = again.recall("alpha", 3).unwrap();
        assert_eq!(r.hits[0].citation, first);
    }

    #[test]
    fn legacy_and_half_written_lines_are_handled() {
        let journal = temp_journal("legacy");
        fs::write(&journal, "old escaped note\n{\"schema_version\":1,\"timestamp_unix\":5,\"kind\":\"note\",\"text\":\"partial").unwrap();
        let mut mem = CitedMemory::new(&journal).without_agentsearch();
        assert_eq!(mem.sync().unwrap(), 1);
        let r = mem.recall("escaped", 3).unwrap();
        assert_eq!(r.hits.len(), 1);
        assert_eq!(r.hits[0].kind, "legacy");
    }

    /// Runs against a real AgentSearch when `PERCI_AGENTSEARCH` (or python3 -m
    /// agentsearch) works; skips otherwise unless PERCI_REQUIRE_AGENTSEARCH=1.
    #[test]
    fn agentsearch_recall_cites_verifies_and_survives_a_crash() {
        let journal = temp_journal("agentsearch");
        let mut mem = CitedMemory::new(&journal);
        mem.remember("note", "dentist moved to Tuesday 3pm")
            .unwrap();
        mem.remember("note", "the backup drive is in the left drawer")
            .unwrap();
        mem.remember("note", "backup schedule runs nightly at 2am")
            .unwrap();
        if mem.warm_up() != "agentsearch" {
            let why = mem.unavailable_reason().unwrap_or("").to_owned();
            if std::env::var("PERCI_REQUIRE_AGENTSEARCH").is_ok_and(|v| v == "1") {
                panic!("AgentSearch required (PERCI_REQUIRE_AGENTSEARCH=1): {why}");
            }
            eprintln!("skipping: {why}");
            return;
        }
        let r = mem.recall("where is the backup drive", 5).unwrap();
        assert_eq!(r.backend, "agentsearch", "{:?}", r.warnings);
        assert!(r.complete);
        assert_eq!(r.hits[0].text, "the backup drive is in the left drawer");
        assert_eq!(r.hits[0].terms_matched, 2);
        assert_eq!(r.hits.len(), 2);
        for hit in &r.hits {
            let bytes = fs::read(&hit.citation.note_path).unwrap();
            assert_eq!(sha256_hex(&bytes), hit.citation.sha256);
            assert_eq!(hit.citation.note_line, NOTE_HEADER_LINES + 1);
        }
        // A note added after the first index refresh is found (index marked dirty).
        mem.remember("note", "the spare backup drive is at the office")
            .unwrap();
        let r2 = mem.recall("spare drive", 5).unwrap();
        assert_eq!(r2.hits[0].text, "the spare backup drive is at the office");
        // AgentSearch verify_evidence flags an edited note.
        let c = r.hits[0].citation.clone();
        assert!(mem.verify(&c).unwrap().ok);
        fs::write(&c.note_path, "edited").unwrap();
        assert!(!mem.verify(&c).unwrap().note_unchanged);
        // Child dies mid-session: recall falls back to substring, no crash.
        mem.kill_backend();
        let r3 = mem.recall("dentist", 5).unwrap();
        assert_eq!(r3.backend, "substring");
        assert_eq!(r3.hits[0].text, "dentist moved to Tuesday 3pm");
        assert!(!r3.warnings.is_empty());
    }

    #[test]
    fn saved_at_is_utc_civil_time() {
        assert_eq!(format_saved_at(0), "undated");
        assert_eq!(format_saved_at(1_760_711_460), "saved 2025-10-17 14:31 UTC");
    }

    #[test]
    fn recall_terms_drop_filler_and_keep_four_longest() {
        assert_eq!(
            recall_terms("where is the backup drive"),
            vec!["backup", "drive"]
        );
        assert!(recall_terms("what did I tell you about it").is_empty());
        assert_eq!(
            recall_terms("alpha bravo charlie delta echoes foxtrot").len(),
            4
        );
    }
}
