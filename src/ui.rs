//! Dark-blood terminal presentation for the standalone Perci shell.
//!
//! Color is automatically disabled for redirected output and can be forced with
//! `PERCI_COLOR=always|never`. Machine-readable commands remain plain JSON.

use std::env;
use std::io::{self, IsTerminal, Write};
use std::time::Duration;

const RESET: &str = "\x1b[0m";
const BLOOD: &str = "\x1b[38;2;177;18;38m";
const BRIGHT_BLOOD: &str = "\x1b[38;2;255;51;79m";
/// YOU prompt + the words `dark-blood` only (hollow-grave cyber purple).
const GRAVE_PURPLE: &str = "\x1b[38;2;168;92;220m";
const BONE: &str = "\x1b[38;2;232;216;216m";
const ASH: &str = "\x1b[38;2;128;106;112m";
const IRON: &str = "\x1b[38;2;193;156;160m";
const GOOD: &str = "\x1b[38;2;117;190;130m";
const WARN: &str = "\x1b[38;2;225;164;74m";

#[derive(Clone, Copy, Debug)]
pub struct BloodUi {
    color: bool,
}

impl BloodUi {
    pub fn detect() -> Self {
        let mode = env::var("PERCI_COLOR")
            .unwrap_or_default()
            .to_ascii_lowercase();
        let color = match mode.as_str() {
            "always" | "1" | "true" => true,
            "never" | "0" | "false" => false,
            _ => io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none(),
        };
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

    /// Clear the host console so the Dark-Blood banner owns the top of the window.
    pub fn clear_stage(&self) {
        print!("\x1b[2J\x1b[3J\x1b[H");
        let _ = io::stdout().flush();
    }

    pub fn banner(&self, backend: &str, cortex: &str) {
        // Version always from Cargo.toml via branding — never a hard-coded string.
        let ver = perci::branding::version();
        // Fixed monospaced diamond (centered under the title row).
        // Only " dark-blood" is purple; spacing matches a single string:
        // "          ◆" / "         ╱ ╲" / "        ◆   ◆   PERCI dark-blood"
        println!(
            "{}",
            self.paint(
                BLOOD,
                "╭──────────────────────────────────────────────────────╮"
            )
        );
        println!(
            "{} {}",
            self.paint(BLOOD, "│"),
            self.paint(BRIGHT_BLOOD, "          ◆")
        );
        println!(
            "{} {}",
            self.paint(BLOOD, "│"),
            self.paint(BRIGHT_BLOOD, "         ╱ ╲")
        );
        // 8 spaces + ◆ + 3 spaces + ◆ + 3 spaces + PERCI + space + dark-blood
        print!(
            "{} {}",
            self.paint(BLOOD, "│"),
            self.paint(BRIGHT_BLOOD, "        ◆   ◆   PERCI")
        );
        println!("{}", self.paint(GRAVE_PURPLE, " dark-blood"));
        println!(
            "{} {}",
            self.paint(BLOOD, "│"),
            self.paint(BRIGHT_BLOOD, "         ╲ ╱")
        );
        println!(
            "{} {}",
            self.paint(BLOOD, "│"),
            self.paint(BRIGHT_BLOOD, "          ◆")
        );
        println!(
            "{} {} {} {}",
            self.paint(BLOOD, "│"),
            self.paint(BRIGHT_BLOOD, "◆  P E R C I"),
            self.paint(BRIGHT_BLOOD, format!("v{ver}")),
            self.paint(IRON, "// governed sparse cognition")
        );
        println!(
            "{}",
            self.paint(
                BLOOD,
                "╰──────────────────────────────────────────────────────╯"
            )
        );
        println!(
            "  {} {}",
            self.paint(ASH, "CORE"),
            self.paint(BONE, backend)
        );
        println!("  {} {}", self.paint(ASH, "MEM "), self.paint(IRON, cortex));
        // "Perci vX.Y.Z · " iron/red family; "dark-blood" purple only.
        println!(
            "  {} {} {}",
            self.paint(ASH, "BRAND"),
            self.paint(IRON, format!("Perci v{ver} · ")),
            self.paint(GRAVE_PURPLE, "dark-blood"),
        );
        println!(
            "  {}",
            self.paint(ASH, "type /help · /think · /concise · /deep · /trace · /quit")
        );
    }

    pub fn opening(&self, insight: &str) {
        let line = insight.trim();
        if line.is_empty() {
            return;
        }
        println!(
            "\n  {} {}",
            self.paint(ASH, "◆"),
            self.paint(IRON, line)
        );
    }

    pub fn prompt(&self) -> io::Result<()> {
        // Human prompt + typed input share purple. Leave color open (no RESET)
        // so what the user types matches `◉ YOU  ›`; call `reset_color` after read.
        print!("\n  ");
        if self.color {
            print!("{GRAVE_PURPLE}◉ YOU  › ");
        } else {
            print!("◉ YOU  › ");
        }
        io::stdout().flush()
    }

    /// End the open purple input color after the user submits a line.
    pub fn reset_color(&self) {
        if self.color {
            print!("{RESET}");
            let _ = io::stdout().flush();
        }
    }

    pub fn response(&self, route: &str, text: &str, elapsed: Duration) {
        let timing = format!(
            "// {} · {:.2} ms verified elapsed",
            route.to_ascii_uppercase(),
            elapsed.as_secs_f64() * 1000.0
        );
        println!(
            "\n  {} {}",
            self.paint(BRIGHT_BLOOD, "◆ PERCI"),
            self.paint(ASH, timing)
        );
        println!("  {}", self.paint(BONE, text));
    }

    pub fn error(&self, text: &str) {
        eprintln!(
            "  {} {}",
            self.paint(BRIGHT_BLOOD, "× FAULT"),
            self.paint(IRON, text)
        );
    }

    pub fn section(&self, title: &str) {
        println!(
            "\n{} {}",
            self.paint(BLOOD, "━━"),
            self.paint(BRIGHT_BLOOD, title)
        );
    }

    pub fn row(&self, key: &str, value: impl AsRef<str>) {
        println!("  {:<14} {}", self.paint(ASH, key), self.paint(BONE, value));
    }

    pub fn verdict(&self, pass: bool, text: &str) {
        let (mark, style) = if pass { ("PASS", GOOD) } else { ("HOLD", WARN) };
        println!("  {}  {}", self.paint(style, mark), self.paint(BONE, text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_mode_contains_no_escape_sequences() {
        let ui = BloodUi::with_color(false);
        assert_eq!(ui.paint(BLOOD, "Perci"), "Perci");
    }

    #[test]
    fn color_mode_resets_every_fragment() {
        let ui = BloodUi::with_color(true);
        let rendered = ui.paint(BLOOD, "Perci");
        assert!(rendered.starts_with("\x1b[38;2;177;18;38m"));
        assert!(rendered.ends_with(RESET));
    }
}
