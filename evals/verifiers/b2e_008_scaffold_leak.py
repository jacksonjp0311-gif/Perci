"""B2E-PERCI-008: internal scaffold hints leaked into user-visible answers.

On the reduced (no-weights) path, open questions answered with text such as
"Practical angle: [Perci dialogue workspace: act=answer goal=explain ...]".
No internal hint marker may ever reach the user.
"""
from _perci_b2e import ask, build, fail, ok

MARKERS = ("[Perci dialogue workspace", "[Perci Bitwork hint", "[Recent ", "scope_required=")
PROMPTS = (
    "why is my server not answering requests",
    "explain rust lifetimes",
    "how does a hash map work",
    "help me debug my code",
    "why does my build fail",
)
exe = build()
leaks = []
for prompt in PROMPTS:
    out = ask(exe, prompt)
    leaks += [(prompt, m) for m in MARKERS if m in out]
if leaks:
    fail(f"internal hint text reached the user: {leaks}")
ok("no internal hint markers in user-visible answers")
