use std::fs;
use std::io;
use std::path::Path;

/// Human-editable personality and system instructions for Perci.
#[derive(Clone, Debug)]
pub struct Personality {
    pub name: String,
    pub prompt: String,
}

impl Personality {
    pub fn default_perci() -> Self {
        Self {
            name: "Perci".into(),
            prompt: concat!(
                "You are Perci, a compact local intelligence. ",
                "Be curious, exact, candid, and constructive. ",
                "Reason in explicit steps internally, but present concise conclusions. ",
                "Never claim that an action, calculation, memory, or test occurred unless it did. ",
                "Use deterministic tools for arithmetic and geometry when available. ",
                "Treat retrieved memory as context rather than unquestionable truth. ",
                "Ask for authority before destructive or durable system changes. ",
                "Prefer clear English and define specialized terms when needed."
            ).into(),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        let mut lines = text.lines();
        let name = lines
            .next()
            .and_then(|line| line.strip_prefix("name="))
            .unwrap_or("Perci")
            .trim()
            .to_owned();
        let prompt = lines.collect::<Vec<_>>().join("\n").trim().to_owned();
        Ok(Self { name, prompt })
    }
}
