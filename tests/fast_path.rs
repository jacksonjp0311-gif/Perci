//! Fast path and tool-mode determinism (B2E-PERCI-012, B2E-PERCI-013).

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn perci() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_perci"));
    cmd.env("PERCI_HOME", env!("CARGO_MANIFEST_DIR"))
        .env_remove("PERCI_DAEMON_TOKEN")
        .env_remove("PERCI_DAEMON_HOST")
        .env_remove("PERCI_DAEMON_PORT")
        .env_remove("PERCI_SESSION")
        .env_remove("PERCI_STATELESS");
    cmd
}

fn temp_data(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("perci-fastpath-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Daemon(Child);
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn daemon_ping_has_no_nagle_floor() {
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let data = temp_data("daemon");
    let _daemon = Daemon(
        perci()
            .arg("daemon")
            .env("PERCI_DAEMON_PORT", port.to_string())
            .env("PERCI_DATA", &data)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn daemon"),
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    let stream = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => break s,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("daemon never listened: {e}"),
        }
    };
    stream.set_nodelay(true).unwrap();
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);
    let mut samples = Vec::new();
    for i in 0..60 {
        let t = Instant::now();
        writer.write_all(b"{\"op\":\"ping\"}\n").unwrap();
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if i >= 10 {
            samples.push(t.elapsed());
        }
        assert!(line.contains("\"ok\":true"), "{line}");
    }
    samples.sort();
    let p50 = samples[samples.len() / 2];
    // Before the fix p50 was ~44 ms (Nagle + delayed ACK). 5 ms leaves CI slack.
    assert!(p50 < Duration::from_millis(5), "ping p50 = {p50:?}");
}

#[test]
fn stateless_ask_is_byte_identical_across_runs() {
    let data = temp_data("ask");
    let run = |q: &str| {
        let out = perci()
            .args(["ask", "--stateless", q])
            .env("PERCI_DATA", &data)
            .output()
            .expect("run perci ask");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };
    for q in [
        "why is my server not answering requests",
        "calculate 17 percent of 240",
    ] {
        let a = run(q);
        let b = run(q);
        assert!(!a.trim().is_empty());
        assert_eq!(a, b, "stateless ask changed between runs for {q:?}");
    }
    // Tool mode writes no session file.
    assert!(!data.join("memory/session.jsonl").exists());
}
