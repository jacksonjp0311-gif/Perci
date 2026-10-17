//! Where Perci finds its read-only assets and where it writes state.
//!
//! * **Home** (`PERCI_HOME`) is the repository checkout that holds `models/`,
//!   `config/`, `knowledge/` and `training/`. Perci only reads from it at
//!   runtime (developer lab commands may still write receipts there).
//! * **Data** (`PERCI_DATA`) holds everything Perci writes during normal use:
//!   memory, session, learning profile and logs. It defaults to a per-user
//!   folder outside the repository, so chatting never dirties `git status`
//!   and running `perci` from any folder never creates files in that folder.
//!
//! Resolution order for home: `PERCI_HOME` → the folder two levels above the
//! executable (`target/release/perci`) when it looks like a Perci checkout →
//! a walk up from the current directory → the current directory (unresolved).

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// File that marks a Perci checkout.
pub const HOME_MARKER: &str = "models/PACKAGE_MANIFEST.json";

/// How the home folder was found (reported by `perci doctor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeSource {
    Env,
    Executable,
    WorkingDirectory,
    /// Nothing looked like a Perci checkout; the current directory is used.
    Unresolved,
}

impl HomeSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Env => "PERCI_HOME",
            Self::Executable => "next to the executable",
            Self::WorkingDirectory => "found from the current directory",
            Self::Unresolved => "not found (using the current directory)",
        }
    }
}

/// True when `dir` looks like a Perci checkout.
pub fn looks_like_home(dir: &Path) -> bool {
    dir.join(HOME_MARKER).is_file()
        || (dir.join("Cargo.toml").is_file() && dir.join("models").is_dir())
}

/// Pure resolution logic (unit-testable; no globals).
pub fn resolve_home(
    env_home: Option<OsString>,
    exe: Option<PathBuf>,
    cwd: Option<PathBuf>,
) -> (PathBuf, HomeSource) {
    if let Some(value) = env_home.filter(|v| !v.is_empty()) {
        return (PathBuf::from(value), HomeSource::Env);
    }
    if let Some(exe) = exe {
        // target/release/perci(.exe) → repo; also allow bin/perci next to a checkout.
        for dir in exe.ancestors().skip(1).take(4) {
            if looks_like_home(dir) {
                return (dir.to_path_buf(), HomeSource::Executable);
            }
        }
    }
    if let Some(cwd) = cwd.clone() {
        for dir in cwd.ancestors().take(8) {
            if looks_like_home(dir) {
                return (dir.to_path_buf(), HomeSource::WorkingDirectory);
            }
        }
    }
    (
        cwd.unwrap_or_else(|| PathBuf::from(".")),
        HomeSource::Unresolved,
    )
}

fn home_cell() -> &'static (PathBuf, HomeSource) {
    static HOME: OnceLock<(PathBuf, HomeSource)> = OnceLock::new();
    HOME.get_or_init(|| {
        let exe = env::current_exe().ok();
        let cwd = env::current_dir().ok();
        let (path, source) = resolve_home(env::var_os("PERCI_HOME"), exe, cwd);
        #[cfg(test)]
        let (path, source) = if source == HomeSource::Unresolved {
            (
                PathBuf::from(env!("CARGO_MANIFEST_DIR")),
                HomeSource::WorkingDirectory,
            )
        } else {
            (path, source)
        };
        (path, source)
    })
}

/// The Perci checkout (read-only assets).
pub fn home() -> PathBuf {
    home_cell().0.clone()
}

pub fn home_source() -> HomeSource {
    home_cell().1
}

/// `home()/rel`, e.g. `paths::asset("models/perci-cognitive-v0.3.pwgt")`.
pub fn asset(rel: impl AsRef<Path>) -> PathBuf {
    home_cell().0.join(rel)
}

/// Default per-user data folder for this platform.
pub fn default_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(local) = env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()) {
            return PathBuf::from(local).join("Perci").join("data");
        }
    }
    if let Some(xdg) = env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(xdg).join("perci");
    }
    if let Some(home) = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .filter(|v| !v.is_empty())
    {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("perci");
    }
    env::temp_dir().join("perci-data")
}

/// Writable state folder (memory, session, logs). Not created until a writer needs it.
pub fn data() -> PathBuf {
    static DATA: OnceLock<PathBuf> = OnceLock::new();
    DATA.get_or_init(|| {
        if let Some(value) = env::var_os("PERCI_DATA").filter(|v| !v.is_empty()) {
            return PathBuf::from(value);
        }
        #[cfg(test)]
        {
            // Unit tests must never touch the real user's memory.
            env::temp_dir().join(format!("perci-test-data-{}", std::process::id()))
        }
        #[cfg(not(test))]
        {
            default_data_dir()
        }
    })
    .clone()
}

/// `data()/rel`.
pub fn data_file(rel: impl AsRef<Path>) -> PathBuf {
    data().join(rel)
}

/// `data()/logs/name`.
pub fn log_file(name: &str) -> PathBuf {
    data().join("logs").join(name)
}

/// First bytes of a Git LFS pointer file.
pub const LFS_POINTER_PREFIX: &[u8] = b"version https://git-lfs";

/// True when `path` is a Git LFS pointer instead of the real payload.
pub fn is_lfs_pointer(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 23];
    match file.read(&mut head) {
        Ok(n) => n == LFS_POINTER_PREFIX.len() && head[..n] == *LFS_POINTER_PREFIX,
        Err(_) => false,
    }
}

/// Marker text in errors that mean "weights/config problem" (exit code 2).
pub const CONFIG_ERROR_TAG: &str = "[perci-config]";

/// Error for a weight file that was never downloaded from Git LFS.
pub fn lfs_pointer_error(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "{CONFIG_ERROR_TAG} {} is a Git LFS pointer, not the real weights. Run: cd \"{}\" && git lfs pull",
            path.display(),
            home().display()
        ),
    )
}

/// Fail with a clear `git lfs pull` hint when `path` is an LFS pointer.
pub fn ensure_not_lfs_pointer(path: &Path) -> io::Result<()> {
    if is_lfs_pointer(path) {
        return Err(lfs_pointer_error(path));
    }
    Ok(())
}

/// Legacy state that older builds wrote into `<home>/memory/`.
const LEGACY_MEMORY_FILES: &[&str] = &[
    "perci.jsonl",
    "session.jsonl",
    "interaction-learning.jsonl",
    "dialogue-profile.json",
];

/// Copy (never move) memory written by older builds into `data()/memory` the
/// first time the new data folder is used. Returns the copied file names.
pub fn migrate_legacy_memory() -> io::Result<Vec<String>> {
    let legacy = asset("memory");
    let target = data_file("memory");
    if !legacy.is_dir() || target.exists() || legacy == target {
        return Ok(Vec::new());
    }
    let mut copied = Vec::new();
    for name in LEGACY_MEMORY_FILES {
        let src = legacy.join(name);
        if src.is_file() {
            fs::create_dir_all(&target)?;
            fs::copy(&src, target.join(name))?;
            copied.push((*name).to_owned());
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_home_wins() {
        let (p, s) = resolve_home(
            Some("/opt/perci".into()),
            Some("/x/target/release/perci".into()),
            Some("/tmp".into()),
        );
        assert_eq!(p, PathBuf::from("/opt/perci"));
        assert_eq!(s, HomeSource::Env);
    }

    #[test]
    fn exe_relative_home_is_found() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let exe = repo.join("target").join("release").join("perci");
        let (p, s) = resolve_home(None, Some(exe), Some(env::temp_dir()));
        assert_eq!(p, repo);
        assert_eq!(s, HomeSource::Executable);
    }

    #[test]
    fn cwd_walk_finds_home_from_a_subfolder() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (p, s) = resolve_home(None, None, Some(repo.join("src")));
        assert_eq!(p, repo);
        assert_eq!(s, HomeSource::WorkingDirectory);
    }

    #[test]
    fn unrelated_folder_is_unresolved() {
        let tmp = env::temp_dir().join("perci-paths-unresolved-test");
        let _ = fs::create_dir_all(&tmp);
        let (p, s) = resolve_home(None, Some(tmp.join("perci")), Some(tmp.clone()));
        assert_eq!(s, HomeSource::Unresolved);
        assert_eq!(p, tmp);
    }

    #[test]
    fn lfs_pointer_is_detected_with_pull_hint() {
        let dir = env::temp_dir().join(format!("perci-lfs-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let pointer = dir.join("w.pwgt");
        fs::write(
            &pointer,
            "version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 12\n",
        )
        .unwrap();
        let real = dir.join("real.pwgt");
        fs::write(&real, b"PERCIW03\0\0\0\0").unwrap();
        assert!(is_lfs_pointer(&pointer));
        assert!(!is_lfs_pointer(&real));
        let err = ensure_not_lfs_pointer(&pointer).unwrap_err().to_string();
        assert!(err.contains("git lfs pull"), "{err}");
        assert!(err.contains(CONFIG_ERROR_TAG));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_data_dir_is_isolated_from_the_user() {
        if env::var_os("PERCI_DATA").is_none() {
            assert!(data().starts_with(env::temp_dir()));
        }
    }
}
