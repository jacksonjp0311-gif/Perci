"""B2E-PERCI-002: agent.rs `truncate` byte-sliced &t[..max] and panicked on multi-byte text.

White-box: the private helper is exercised by appending a test to a scratch copy
of src/agent.rs (the workspace is not modified).
"""
from _perci_b2e import fail, ok, probe

TEST = r'''
    #[test]
    fn truncate_is_char_safe() {
        let s = "cargo: tests passed … café résumé naïve ✓✓✓";
        for max in 0..s.len() + 2 {
            let out = super::truncate(s, max);
            assert!(out.chars().count() <= max + 1, "max={max} out={out}");
        }
    }
'''
proc = probe(TEST, append_to="src/agent.rs")
if proc.returncode != 0:
    fail("agent truncate panicked or did not build on multi-byte input")
ok("agent truncate never splits a UTF-8 character")
