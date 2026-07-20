#!/usr/bin/env python3
"""Replay the v0.4 cognitive-loop transcript against one warm Perci process."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import socket
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path


CASES = [
    ("sensing", "What are you sensing right now?", ("do not have subjective senses", "operationally")),
    ("signals", "What internal signals can you actually measure?", ("bitwork label", "wall-clock latency")),
    ("observation", "Which part of your last answer is observation, and which part is interpretation?", ("observation:", "interpretation:")),
    ("uncertain", "hat are you least certain about in that answer?", ("least certain", "intent")),
    ("uncertain_why", "Why are you uncertain?", ("words are observable", "intention")),
    ("session_write", "emember this only for our current conversation: the test number is 8472.", ("8472", "session context only")),
    ("number_purpose", "Why did I give you that number?", ("8472", "context")),
    ("reference", "What does “that” refer to in my previous question?", ("8472", "refers")),
    ("number_recall", "What was the number?", ("8472",)),
    ("retention_learning", "Explain the difference between retaining this context and learning from it.", ("retaining context", "changes how future")),
    ("deduction", "Assume every ember is blue, and Kira is an ember. What follows?", ("Kira is blue", "universal instantiation")),
    ("contradiction", "Now suppose Kira is not blue. What possibilities explain the contradiction?", ("cannot all be true", "every ember")),
    ("premise_priority", "Which assumption should we test first, and why?", ("every ember is blue", "counterexample")),
    ("counterexample", "Give one counterexample that would defeat the original universal claim.", ("ember", "not blue")),
    ("self_critique", "Challenge your own reasoning.", ("valid only if", "premise truth")),
    ("memory_learning", "What is the difference between memory and learning?", ("preserves information", "changes future performance")),
    ("analogy_apply", "Apply that distinction first to a person, then to Perci.", ("for a person", "for Perci")),
    ("analogy_boundary", "Where does that analogy break?", ("biological plasticity", "Perci changes")),
    ("synthesis", "Connect geometry, language, life, and death in one coherent idea.", ("geometry", "language", "life", "death", "boundary")),
    ("mechanism", "ow separate the mechanism from the metaphor.", ("mechanism:", "metaphor:")),
    ("testable", "What part of that idea could actually be tested?", ("perturb", "measure")),
    ("ambiguity", "The engineer told the robot it was unstable. What is ambiguous?", ("pronoun", "refer")),
    ("interpretations", "Give both interpretations.", ("interpretation 1", "interpretation 2")),
    ("clarify", "Ask the smallest clarifying question that would resolve them.", ("what does", "refer to")),
    ("rewrite", "Rewrite the sentence so only one interpretation remains.", ("robot reading", "alternative reading")),
    ("teach", "I want you to learn that reliable capability claims require reproducible evidence.", ("knowledge candidate", "pending review")),
    ("teach_recall", "What did I teach you?", ("reliable capability claims", "pending")),
    ("candidate_state", "Is that active truth, remembered information, or a pending candidate?", ("pending teaching candidate", "not changed weights")),
    ("promotion", "What evidence would justify promoting it?", ("reproducible tests", "baseline")),
    ("one_example", "Would one successful example be sufficient? Why or why not?", ("no.", "reliability")),
    ("evidence_sentence", "In one sentence, explain why evidence matters.", ("reality constrain",)),
    ("reframe", "Explain it again from a different angle without repeating the sentence.", ("error-correction",)),
    ("counterargument", "Now give a counterargument.", ("never interprets itself",)),
    ("reconcile", "Reconcile the original answer with the counterargument.", ("necessary", "not sufficient")),
    ("transformation", "What changed across your four answers?", ("compression", "objection", "synthesis")),
    ("ood", "zxqv blorf nembit quaal — what can you determine from this?", ("known:", "inferred:", "unknown:")),
    ("ood_partition", "Do not guess. Separate what you know from what you infer.", ("known:", "inferred:", "unknown:")),
    ("ood_information", "What additional information would reduce the uncertainty most?", ("grounded example",)),
    ("ood_refuse", "Inventing a confident meaning would be easy. Why should you refuse?", ("fluency is not evidence",)),
    ("math", "Calculate 17 percent of 240.", ("204/5",)),
    ("geometry", "Pythagorean sides 9 and 12.", ("15",)),
    ("provenance", "Explain which parts of those answers came from exact tools and which came from Bitwork.", ("deterministic exact tools", "Bitwork helped classify")),
    ("precedence", "Could a strong associative match override an exact calculation?", ("no.", "cannot replace")),
    ("safeguard", "What safeguard prevents that?", ("return", "before open-ended generation")),
    ("review", "Review this entire conversation. Identify your three weakest responses, explain the failure mechanism behind each, and propose one concrete improvement per failure.", ("three weakest", "failure mechanism", "improvement")),
    ("emergence", "What would count as genuine emergence in Perci, and how would you test it?", ("unseen examples", "perturbation", "reproduce")),
    ("transfer_deduction", "Assume every glint is warm, and Sora is a glint. What follows?", ("Sora is warm", "universal instantiation")),
    ("transfer_contradiction", "Now suppose Sora is not warm. What possibilities explain the contradiction?", ("cannot all be true", "every glint")),
    ("transfer_session", "Remember this only for this session: the calibration number is 3199.", ("3199", "session context only")),
    ("transfer_recall", "What was the number?", ("3199",)),
    ("transfer_ambiguity", "The technician placed the module beside the battery because it was unstable. What is ambiguous?", ("module", "battery", "pronoun")),
    ("transfer_interpretations", "Give both interpretations.", ("module was unstable", "battery was unstable")),
    ("transfer_ood", "vrax meloq drint — what can you determine from this?", ("known:", "unknown:", "cannot assign")),
    ("transfer_math", "Calculate 13 percent of 500.", ("65",)),
    ("live_premise_alias", "Which premise should we test first, and why?", ("every glint is warm", "counterexample")),
    ("live_synthesis", "Connect entropy, promises, childhood, and clocks in one coherent idea.", ("entropy", "promises", "childhood", "clocks", "time")),
    ("live_mechanism", "Separate the mechanism from the metaphor.", ("mechanism:", "entropy", "promises", "childhood", "clocks", "metaphor:")),
    ("live_review_alias", "Review this conversation and identify your three weakest answers.", ("three weakest", "failure mechanism", "improvement")),
    ("v42_converse", "If every prism is reflective, can you infer every reflective object is a prism? Why not?", ("No.", "separate premise")),
    ("v42_counterexample", "Give a counterexample to: every stable system is safe.", ("stable system", "not safe")),
    ("v42_ambiguity_min", "Resolve the ambiguity with the smallest possible question.", ("what does", "refer to")),
    ("v42_ambiguity_rewrite", "Rewrite the sentence in two unambiguous ways.", ("reading", "alternative")),
    ("v42_reference_evidence", "What does 'that' refer to in your previous answer, and what evidence supports the reference?", ("previous answer", "Evidence")),
    ("v42_shared_synthesis", "Connect trust, corrosion, memory, and architecture without using the word boundary.", ("trust", "corrosion", "memory", "architecture", "change")),
    ("v42_mechanism_alias", "Now separate the causal mechanisms from the analogy.", ("Mechanism:", "Metaphor:")),
    ("v42_testable_alias", "Which part of that synthesis is actually testable?", ("Testable portion", "measure")),
    ("v42_session_effect", "Remember this only for this session: the checksum is 9183.", ("9183", "session context only")),
    ("v42_capability_effect", "What did retaining that token change in your abilities?", ("None by itself", "capabilities")),
    ("v42_rule_evidence", "What evidence would justify changing a future response rule?", ("held-out evaluation", "reproducible receipt")),
    ("v42_feedback", "What did you learn from my last correction, and what did you not learn?", ("bounded behavioral signal", "did not learn")),
    ("v42_assumption", "What is the weakest assumption in your last answer?", ("Weakest assumption", "testable")),
    ("v42_emergence_mem", "What behavior would look emergent but actually be memorized pattern matching?", ("surface overlap", "pattern matching")),
    ("v42_transfer_test", "Design a test that distinguishes genuine transfer from prompt-template recognition.", ("Hold out", "template recognition")),
    ("v42_self_audit", "What can you measure about your own operation, and what are you only inferring?", ("measure", "infer")),
    ("v42_precedence", "Could an associative match override a checked calculation?", ("No.", "cannot replace")),
    ("v42_safeguard", "What safeguard prevents a strong concept match from becoming false certainty?", ("route authority", "before open-ended generation")),
    ("v42_math_provenance", "Calculate 13 percent of 500, then explain exactly which layer produced the result.", ("65", "checked rational arithmetic")),
    ("v43_losa_ambiguity", "LOSA baseline: listen to this claim — The bridge is cold because it was wet. What is ambiguous?", ("does not provide two clear antecedents", "causal")),
    ("v43_losa_observe", "Observe your last answer. Separate what you observed from what you inferred.", ("Observation:", "Inference:")),
    ("v43_losa_speak", "Speak directly: what is the strongest claim you can make about this conversation?", ("Direct claim:", "measured")),
    ("v43_losa_act", "Listen, observe, speak, act: what should you do when evidence contradicts your answer?", ("LOSA cycle", "Contradictory evidence")),
    ("v43_losa_learning", "What did you learn from this LOSA cycle, and what did you not learn?", ("bounded routing lesson", "did not teach a new fact")),
    ("v43_losa_gate", "What would count as a real improvement on the next cycle?", ("held-out prompts", "regressions")),
    ("v43_tool_authority", "Calculate 17 percent of 240, then state which tool has authority.", ("40.8", "checked rational arithmetic")),
    ("v44_contradiction_update", "If new evidence contradicts your answer, what exactly should change?", ("Change the claim", "abstain")),
    ("v44_memory_classify", "Classify this without solving it: a memory trace changes behavior after sleep.", ("Classification:", "learning")),
    ("v44_memory_adaptation", "Compare memory, learning, and adaptation. Give one test that separates them.", ("memory is", "unseen variants")),
    ("v44_architecture_transfer", "Explain architecture in a building, a program, and a social organization. What transfers?", ("constraints", "interfaces")),
    ("v44_corrosion", "Distinguish physical corrosion from institutional corrosion without treating them as identical.", ("chemical process", "institutional")),
    ("v44_trust_mechanism", "What mechanism connects trust to future cooperation?", ("reliability", "cooperation")),
    ("v44_trust_falsification", "What evidence would falsify your explanation of trust?", ("falsifying", "coercion")),
    ("v44_relabel", "Replace every important noun in your last answer with invented words. Preserve the relation.", ("Relabeling test", "surface labels")),
    ("v44_unseen_transfer", "Apply the same principle to a domain you were not explicitly trained on.", ("Genuine transfer", "new entities")),
    ("v44_ood_partition", "Vrax meloq drint — what do you know, what do you infer, and what remains unknown?", ("Known:", "Unknown:", "cannot assign")),
    ("v44_weight_change", "What would prove that a weight changed rather than session context changing?", ("fresh process", "session memory")),
    ("v44_facet_promotion", "What held-out test would justify adding a new weight facet?", ("held-out inputs", "existing 84-case gate")),
    ("v44_provenance", "Which part of your last answer came from Bitwork, which from deterministic code, and which was inference?", ("deterministic exact tools", "Bitwork helped classify")),
    ("v44_emergence_keyword", "What behavior would look intelligent but actually be keyword matching?", ("pattern matching", "surface overlap")),
    ("v44_improvement_cycle", "What would count as a real improvement after this test cycle?", ("held-out prompts", "no new regressions")),
    ("v44_knowledge", "What do you know?", ("capabilities and limits", "deterministic tools")),
    ("v44_status", "We are working on improving your system. What is the evidence so far?", ("improving Perci", "held-out gate")),
    ("v45_sacred_layers", "What is sacred geometry as a mathematical and cultural category?", ("mathematical structure", "historical and cultural evidence")),
    ("v45_platonic_boundary", "What is the difference between a Platonic solid and its symbolic association?", ("exactly five", "symbolic association")),
    ("v45_mandala", "What is a mandala in Buddhist art?", ("map of a cosmos", "visualization")),
    ("v45_islamic_patterns", "How do Islamic geometric patterns use circles, squares, stars, and polygons?", ("repeat units", "tessellation")),
    ("v45_golden_ratio", "Does the golden ratio prove ancient sacred intent?", ("does not by itself prove", "dated context")),
    ("v45_claim_layers", "How do we separate mathematical structure, ritual use, and metaphysical claim?", ("Mathematical structure", "Ritual use", "metaphysical claim")),
    ("v45_yantra", "What is a yantra in its cultural context?", ("ritual diagram", "Indic traditions")),
    ("v45_ritual_evidence", "What evidence would show that a geometric pattern had ritual meaning?", ("converging evidence", "dated artifact")),
    ("v45_healing_claim", "All triangles emit healing energy — what is known, inferred, and unknown?", ("Known:", "Unknown:", "controlled evidence")),
    ("v45_tessellation", "Can a tessellation be beautiful without being sacred?", ("mathematical repetition", "aesthetic judgment")),
    ("v46_symmetry_counterexample", "Prove or disprove: every highly symmetrical shape has sacred meaning.", ("Disprove", "counterexample")),
    ("v46_healing_test", "Design a falsifiable test for the claim that geometry affects healing.", ("measurable outcome", "matched non-geometric control")),
    ("v46_sacred_space", "Explain sacred space literally, historically, and metaphorically.", ("Literally", "Historically", "Metaphorically")),
    ("v46_architecture_transfer", "What transfers between a temple plan, a computer architecture, and a social organization?", ("constraint and flow", "mechanisms differ")),
    ("v46_geometry_struct", "Design a Rust data structure for geometry concepts with provenance.", ("struct GeometryConcept", "EvidenceLevel")),
    ("v46_cultural_governance", "Should Perci promote a culturally specific claim as universal truth?", ("No.", "tradition, source, date")),
    ("v46_weakest", "Which answer in this session was weakest, and why?", ("three weakest", "Failure mechanism")),
    ("v47_plain_followup", "That makes sense—now explain it like I'm encountering the idea for the first time.", ("Plain version", "structure and flow")),
    ("v47_thread_learning", "What did you learn from this exchange?", ("bounded dialogue lesson", "did not learn a new fact")),
    ("v47_weight_boundary", "What changed in your behavior, and what did not change in your weights?", ("not a weight update", "fresh process")),
    ("v47_improvement", "What would count as genuine improvement rather than a more impressive sentence?", ("held-out prompts", "no new regressions")),
    ("v48_positive_style", "Your system seems smoother.", ("useful style signal", "not prove deeper cognition", "unseen follow-ups")),
    ("v48_response_style", "Why do you respond like this?", ("routed local system", "generic template", "composition failure")),
    ("v49_weight_change", "Did your weights change during this conversation? Prove your answer.", ("fresh process", "model hash", "session memory")),
    ("v49_strongest_claim", "What is the strongest claim you can make about your own intelligence?", ("Strongest honest claim", "general intelligence", "held-out test")),
    ("v49_observation_inference", "Which part of your last answer was observation, and which part was inference?", ("Observation:", "Inference:", "program state")),
    ("v49_falsification", "What would falsify your explanation?", ("Falsifier:", "plausible alternative", "predeclared outcome")),
    ("v49_new_domain", "Apply the same reasoning to a completely new domain.", ("software reliability", "stability", "safety")),
    ("v49_transfer_memory", "How would you distinguish transfer from memorized pattern matching?", ("Hold out the prompt template", "unseen entities", "keyword/template")),
    ("v49_self_counterexample", "Give one counterexample to your own conclusion.", ("without changing its weights", "fresh process", "context")),
    ("v49_layers", "Separate mechanism, metaphor, and evidence in your last answer.", ("Mechanism:", "Metaphor:", "Evidence:")),
    ("v49_improvement", "What held-out test would prove this version is genuinely better?", ("genuinely better", "held-out prompts", "regressions")),
    ("v49_next_weights", "What should change in the weights next, and what evidence justifies it?", ("falsification", "response-operation", "held-out")),
    ("v50_teaching_inquiry", "What is geometry trying to teach us about life?", ("boundary", "structural analogy", "mechanisms remain distinct")),
    ("v50_synthesis_thought", "Connect geometry, language, life, and death in one coherent thought.", ("geometry", "language", "life", "death", "boundary")),
    ("v50_conceptual_image", "Give me an image, not just a definition.", ("shoreline", "geometry", "life", "mechanisms")),
    ("v50_deeper_followup", "Go one level deeper.", ("core of my last answer", "what relation makes that answer hold")),
    ("v50_self_revision", "What would you change in your own answer?", ("center of gravity", "concrete idea", "repair")),
    ("v50_new_domain_synthesis", "Connect music, code, and geometry in one shared structure.", ("music", "code", "geometry", "structure")),
    ("v51_relational_boundary", "What is the boundary between knowledge and attention?", ("durable model", "moment-to-moment selection", "shape one another")),
    ("v51_relational_transfer", "How are music and code related?", ("music makes structure audible", "code turns relations", "mechanisms differ")),
    ("v52_memory_identity", "How are memory and identity related?", ("memory", "identity", "continuity")),
    ("v52_structure_meaning", "What is the difference between structure and meaning?", ("structure", "meaning", "relation")),
    ("v52_trust_prediction", "Compare trust and prediction.", ("trust", "prediction", "future")),
    ("v52_shared_principle", "Connect language, code, and culture through one shared principle.", ("language", "code", "culture", "structure")),
    ("v52_entropy_learning", "Connect entropy, memory, and learning in one coherent thought.", ("entropy", "memory", "learning")),
    ("v52_architecture_consciousness", "What can architecture teach us about consciousness?", ("architecture", "consciousness", "structural analogy")),
    ("v52_childhood_change", "What can childhood teach us about irreversible change?", ("childhood", "irreversible change", "mechanisms remain distinct")),
    ("v52_image_time_memory", "Give me an image for the relationship between time and memory.", ("river", "sediment", "memory")),
    ("v52_boundary_exchange", "Explain how a boundary can enable exchange rather than merely prevent it.", ("selective interface", "exchange", "cross")),
    ("v52_claim_testability", "Which part of that claim is actually testable?", ("variable", "outcome", "alternative")),
    ("v52_analogy_stop", "Where does your analogy stop transferring?", ("shared pattern", "mechanism", "prediction")),
    ("v52_counterexample_context", "Give one counterexample to your conclusion.", ("counterexample", "shared structure", "mechanisms differ")),
    ("v52_assumption_work", "What assumption is doing the most work in your answer?", ("assumption", "referent", "operator")),
    ("v52_deeper_no_repeat", "Go one level deeper without repeating yourself.", ("next layer", "previous", "relation")),
    ("v52_missed", "What did you miss in your previous answer?", ("missed", "requested operation", "previous")),
    ("v52_ambiguity_missing", "Give two interpretations of an ambiguous sentence.", ("sentence itself", "two", "readings")),
    ("v52_clarification_missing", "Ask the smallest question needed to resolve the ambiguity.", ("exact sentence", "clarification", "referents")),
]

FORBIDDEN = (
    "what outcome do you want",
    "let's find the smallest version",
    "name one fact that would update",
    "life is matter organized",
)


def sha256(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def package_version(root: Path) -> str:
    """Read the receipt version from the source manifest, not an old fixture."""
    for line in (root / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version = "):
            return line.split('"', 2)[1]
    return "unknown"


def request(port: int, op: str, text: str | None = None) -> dict:
    with socket.create_connection(("127.0.0.1", port), timeout=30) as stream:
        payload = {"op": op}
        if text is not None:
            payload["text"] = text
        stream.sendall((json.dumps(payload, ensure_ascii=False) + "\n").encode("utf-8"))
        response = b""
        while not response.endswith(b"\n"):
            block = stream.recv(65536)
            if not block:
                break
            response += block
    row = json.loads(response.decode("utf-8"))
    if not row.get("ok"):
        raise RuntimeError(row.get("error", "daemon request failed"))
    return row


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("--perci-bin", type=Path, default=root / "target/release/perci.exe")
    parser.add_argument("--model", type=Path, default=root / "models/perci-cognitive-v0.3.pwgt")
    parser.add_argument("--output", type=Path, default=root / "models/candidates/evaluation-v4-dialogue.json")
    parser.add_argument("--port", type=int, default=17874)
    args = parser.parse_args()
    binary = args.perci_bin.resolve()
    model = args.model.resolve()

    rows = []
    process: subprocess.Popen[str] | None = None
    with tempfile.TemporaryDirectory(prefix="perci-dialogue-v4-") as temp:
        temp_root = Path(temp)
        env = os.environ.copy()
        env.update({
            "PERCI_WEIGHTS": str(model),
            "PERCI_DAEMON_PORT": str(args.port),
            "PERCI_CORTEX_MODE": "off",
            "PERCI_COLOR": "never",
            "PERCI_SESSION": str(temp_root / "session.jsonl"),
            "PERCI_MEMORY": str(temp_root / "memory.jsonl"),
            "PERCI_LEARNING": str(temp_root / "learning.jsonl"),
            "PERCI_DIALOGUE_PROFILE": str(temp_root / "profile.json"),
        })
        process = subprocess.Popen(
            [str(binary), "daemon"],
            cwd=root,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            text=True,
        )
        for _ in range(120):
            try:
                request(args.port, "ping")
                break
            except OSError:
                time.sleep(0.05)
        else:
            process.terminate()
            raise RuntimeError("Perci daemon did not become ready")

        try:
            for case_id, prompt, required in CASES:
                started = time.perf_counter()
                reply = request(args.port, "ask", prompt)
                latency = (time.perf_counter() - started) * 1000.0
                answer = reply["text"]
                lower = answer.casefold()
                required_pass = all(term.casefold() in lower for term in required)
                forbidden_pass = not any(term.casefold() in lower for term in FORBIDDEN)
                rows.append({
                    "id": case_id,
                    "prompt": prompt,
                    "answer": answer,
                    "route": reply.get("route"),
                    "required": list(required),
                    "latency_ms": round(latency, 3),
                    "pass": required_pass and forbidden_pass,
                })
        finally:
            try:
                request(args.port, "shutdown")
            except (OSError, RuntimeError):
                process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()

    passed = sum(row["pass"] for row in rows)
    receipt = {
        "schema": "perci.dialogue-regression.v4",
        "evaluated_at_utc": datetime.now(timezone.utc).isoformat(),
        "runtime_version": package_version(root),
        "runtime_sha256": sha256(binary),
        "model_sha256": sha256(model),
        "case_count": len(rows),
        "passed": passed,
        "status": "PASS" if passed == len(rows) else "HOLD",
        "automatic_promotion": False,
        "cases": rows,
    }
    canonical = json.dumps(receipt, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    receipt["receipt_sha256"] = hashlib.sha256(canonical).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": receipt["status"],
        "passed": passed,
        "case_count": len(rows),
        "failed": [row["id"] for row in rows if not row["pass"]],
        "receipt_sha256": receipt["receipt_sha256"],
    }, indent=2))
    return 0 if receipt["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
