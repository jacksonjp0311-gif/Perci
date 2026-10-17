//! Clean, neutral terminal presentation for the interactive Perci shell.
//!
//! Design goals: a compact welcome header, a simple `›` prompt, readable
//! replies, and dim status lines. One calm accent color (soft cyan) on top of
//! soft white / gray text.
//!
//! Color is disabled automatically for redirected output, when `NO_COLOR` is
//! set, or when `TERM=dumb`. It can be forced with `PERCI_COLOR=always|never`.
//! Machine-readable commands remain plain JSON (they never go through here).

use std::env;
use std::io::{self, IsTerminal, Write};
use std::time::Duration;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
/// The single accent color: calm, soft cyan.
const ACCENT: &str = "\x1b[38;2;102;204;230m";
/// Primary readable text: soft white.
const TEXT: &str = "\x1b[38;2;228;228;231m";
/// Secondary / status text: neutral gray.
const DIM: &str = "\x1b[38;2;138;138;146m";
const GOOD: &str = "\x1b[38;2;126;200;146m";
const WARN: &str = "\x1b[38;2;228;184;96m";
const ERROR: &str = "\x1b[38;2;236;120;120m";

#[derive(Clone, Copy, Debug)]
pub struct Ui {
    color: bool,
}

impl Ui {
    pub fn detect() -> Self {
        let mode = env::var("PERCI_COLOR")
            .unwrap_or_default()
            .to_ascii_lowercase();
        let wanted = match mode.as_str() {
            "always" | "1" | "true" => true,
            "never" | "0" | "false" => false,
            _ => {
                io::stdout().is_terminal()
                    && env::var_os("NO_COLOR").is_none()
                    && env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
            }
        };
        // On Windows consoles ANSI sequences only render once virtual terminal
        // processing is enabled; fall back to plain text if that fails.
        let color = wanted && enable_ansi();
        Self { color }
    }

    #[cfg(test)]
    fn with_color(color: bool) -> Self {
        Self { color }
    }

    fn paint(&self, style: &str, text: impl AsRef<str>) -> String {
        if self.color {
            format!("{style}{}{RESET}", text.as_ref())
        } else {
            text.as_ref().to_owned()
        }
    }

    /// Optionally clear the console before the welcome header.
    ///
    /// Typing `perci` in a shell should feel like any other CLI and keep the
    /// user's scrollback, so this only clears when `PERCI_CLEAR_SCREEN=1`
    /// (and never when output is plain / redirected).
    pub fn clear_stage(&self) {
        let wants_clear = env::var("PERCI_CLEAR_SCREEN")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if self.color && wants_clear {
            print!("\x1b[2J\x1b[3J\x1b[H");
            let _ = io::stdout().flush();
        }
    }

    /// Compact welcome header: name + version, one-line hint, dim status.
    pub fn banner(&self, backend: &str, cortex: &str) {
        // Semver + git build id, never a hard-coded string.
        let ver = perci::branding::version();
        let build = perci::branding::build_id();
        let shown = if build != ver {
            format!("v{build}")
        } else {
            format!("v{ver}")
        };
        let title = if self.color {
            format!("{BOLD}{ACCENT}Perci{RESET}")
        } else {
            "Perci".to_owned()
        };

        println!();
        println!(
            "  {} {}  {}",
            self.paint(ACCENT, "●"),
            title,
            self.paint(DIM, shown)
        );
        println!(
            "    {}",
            self.paint(TEXT, "Type a message, /help for commands, /exit to quit.")
        );

        // Cortex status can be long ("attached | repo X | warm daemon | cache N");
        // the first segment is enough for a status line. /status shows the rest.
        let cortex_short = cortex.split(" | ").next().unwrap_or(cortex).trim();
        println!(
            "    {}",
            self.paint(DIM, format!("{backend} · cortex {cortex_short}"))
        );
    }

    pub fn opening(&self, insight: &str) {
        let line = insight.trim();
        if line.is_empty() {
            return;
        }
        println!("\n    {}", self.paint(DIM, line));
    }

    pub fn prompt(&self) -> io::Result<()> {
        // Typed input stays in the terminal's default color.
        print!("\n{} ", self.paint(ACCENT, "›"));
        io::stdout().flush()
    }

    /// Ensure no color is left open after the user submits a line.
    pub fn reset_color(&self) {
        if self.color {
            print!("{RESET}");
            let _ = io::stdout().flush();
        }
    }

    pub fn response(&self, route: &str, text: &str, elapsed: Duration) {
        let mut lines = text.trim_end().lines();
        let first = lines.next().unwrap_or("");
        println!("\n{} {}", self.paint(ACCENT, "●"), self.paint(TEXT, first));
        for line in lines {
            if line.is_empty() {
                println!();
            } else {
                println!("  {}", self.paint(TEXT, line));
            }
        }
        let meta = format!(
            "{} · {:.1} ms",
            route.to_ascii_lowercase(),
            elapsed.as_secs_f64() * 1000.0
        );
        println!("  {}", self.paint(DIM, meta));
    }

    pub fn error(&self, text: &str) {
        eprintln!(
            "\n{} {}",
            self.paint(ERROR, "✗ error:"),
            self.paint(TEXT, text)
        );
    }

    pub fn section(&self, title: &str) {
        let heading = if self.color {
            format!("{BOLD}{ACCENT}{title}{RESET}")
        } else {
            title.to_owned()
        };
        println!("\n  {heading}");
    }

    pub fn row(&self, key: &str, value: impl AsRef<str>) {
        // Pad before painting so escape codes do not break column alignment.
        println!(
            "  {} {}",
            self.paint(DIM, format!("{key:<14}")),
            self.paint(TEXT, value)
        );
    }

    pub fn verdict(&self, pass: bool, text: &str) {
        let (mark, style) = if pass {
            ("✓ pass", GOOD)
        } else {
            ("! hold", WARN)
        };
        println!("  {}  {}", self.paint(style, mark), self.paint(TEXT, text));
    }
}

/// Enable ANSI escape processing on the Windows console (no-op elsewhere).
#[cfg(windows)]
fn enable_ansi() -> bool {
    use std::os::windows::io::AsRawHandle;

    type Handle = *mut std::ffi::c_void;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleMode(handle: Handle, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: Handle, mode: u32) -> i32;
    }

    let handle = io::stdout().as_raw_handle() as Handle;
    let mut mode: u32 = 0;
    // SAFETY: plain Win32 console calls on our own stdout handle; `mode` is a
    // valid, initialized out-pointer for the duration of the call.
    unsafe {
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }
        if mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0 {
            return true;
        }
        SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
    }
}

#[cfg(not(windows))]
fn enable_ansi() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_mode_contains_no_escape_sequences() {
        let ui = Ui::with_color(false);
        assert_eq!(ui.paint(ACCENT, "Perci"), "Perci");
        assert!(!ui.paint(DIM, "status").contains('\x1b'));
    }

    #[test]
    fn color_mode_resets_every_fragment() {
        let ui = Ui::with_color(true);
        let rendered = ui.paint(ACCENT, "Perci");
        assert!(rendered.starts_with(ACCENT));
        assert!(rendered.ends_with(RESET));
    }
}
