"""B2E-PERCI-003: over-broad "not answering / won't answer" patterns hijacked technical questions.

A technical question such as "why is my server not answering requests" was routed
to the Feedback (apology) act. Genuine complaints aimed at Perci must still be Feedback.
"""
from _perci_b2e import fail, ok, probe

TEST = r'''
use perci::voice::{detect_dialogue_act, DialogueAct};

#[test]
fn technical_prompts_are_not_feedback() {
    for prompt in [
        "why is my server not answering requests",
        "the api is not answering health checks after deploy",
        "my dns resolver won't answer queries",
        "postgres wont answer on port 5432",
        "the printers aren't answering pings",
        "why is the daemon not answering on localhost",
    ] {
        assert_ne!(detect_dialogue_act(prompt), DialogueAct::Feedback, "{prompt}");
    }
}

#[test]
fn real_complaints_stay_feedback() {
    for prompt in [
        "you're not answering the question",
        "you didn't answer what i asked",
        "stop dodging the question",
    ] {
        assert_eq!(detect_dialogue_act(prompt), DialogueAct::Feedback, "{prompt}");
    }
}
'''
proc = probe(TEST)
if proc.returncode != 0:
    fail("technical prompts were routed to Feedback, or real complaints were not")
ok("feedback detection is anchored to complaints about Perci")
