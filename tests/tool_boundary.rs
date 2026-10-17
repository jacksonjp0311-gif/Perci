//! The tool layer must not depend on the chat app (`perci-core` seam).

#[test]
fn tools_do_not_use_chat_modules() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tools");
    let forbidden = [
        "crate::chat",
        "crate::voice",
        "crate::deliberation",
        "crate::session",
        "ChatEngine",
    ];
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        for f in forbidden {
            assert!(!text.contains(f), "{} uses {f}", path.display());
        }
    }
    let verdict = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/verdict.rs"),
    )
    .unwrap();
    for f in forbidden {
        assert!(!verdict.contains(f), "verdict.rs uses {f}");
    }
}
