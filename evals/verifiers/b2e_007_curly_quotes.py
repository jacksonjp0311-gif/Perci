"""B2E-PERCI-007: curly apostrophes (’) routed differently from ASCII ones (')."""
from _perci_b2e import fail, ok, probe

TEST = r'''
use perci::text_normalize::normalize_for_routing;
use perci::voice::{detect_dialogue_act, detect_social};

#[test]
fn curly_quotes_route_like_ascii() {
    assert_eq!(normalize_for_routing("I\u{2019}m tracking the build"), "i'm tracking the build");
    assert_eq!(detect_social("how\u{2019}s it going today"), detect_social("how's it going today"));
    assert_eq!(
        detect_dialogue_act("seems like you\u{2019}re stuck again"),
        detect_dialogue_act("seems like you're stuck again")
    );
}
'''
proc = probe(TEST)
if proc.returncode != 0:
    fail("curly apostrophes are not normalised before routing")
ok("curly and straight apostrophes route the same")
