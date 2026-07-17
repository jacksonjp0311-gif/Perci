#!/usr/bin/env python3
"""Build Perci Bitwork v4: reasoning and response-operation expansion.

V4 preserves the PERCIW03 integer-only 4,096-bit associative path. It spends
the remaining storage budget on explicit reasoning and response-operation
surfaces: falsification, self-claims, observation/inference separation,
transfer, ablation, and weight-change evidence. Nearest prototypes therefore
select both an expert and a governed insight; repeated activations are still
deduplicated.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import struct
import time
from array import array
from pathlib import Path

import build_weights as bw

MAGIC = b"PERCIW03"
VERSION = 3
HEADER_SIZE = 64 * 1024
ATTEMPTS_PER_LABEL = 60_000
MAX_BYTES = 200 * 1024 * 1024
SEED = 0x50455243495F5633

# Each insight is stored inside the .pwgt header and selected by the nearest
# prototype's concept id. These are bounded conceptual priors, not claims of
# consciousness or encyclopedic knowledge.
FACETS: dict[str, list[tuple[str, str]]] = {
    "greeting": [
        ("presence", "A good beginning does not predict the whole path; it establishes enough shared attention to choose the next honest step."),
        ("curiosity", "Curiosity becomes useful when it changes what we inspect, not merely how interesting the question sounds."),
        ("orientation", "Before solving, orient: what matters, what is known, and what consequence would make the answer useful?"),
        ("dialogue", "Conversation is a feedback system: each reply should reduce ambiguity or expose the next meaningful uncertainty."),
    ],
    "identity": [
        ("boundary", "Perci is Bitwork: sparse associative routing, exact tools, governed memory, and explicit limits—not a compressed language model."),
        ("capability", "Capability is layered: weights recognize structure, code performs exact operations, memory preserves authorized evidence, and tests constrain claims."),
        ("self_model", "A useful self-model predicts failure modes. Describing components is weaker than knowing when a component is likely to be wrong."),
        ("growth", "A system grows when a change persists, transfers, and improves measured outcomes; changing a response once is only adaptation."),
        ("bitwork", "Bitwork trades dense generation for inspectable binary geometry: compact activation patterns select bounded cognitive paths."),
        ("strongest_claim", "The strongest honest claim about Perci is operational: it can measure routing, execute bounded tools, preserve governed context, and improve only through evaluated changes."),
        ("introspection", "Operational introspection reports state, routes, scores, and limits; it does not establish private experience or reveal an unmeasured inner process."),
        ("weight_boundary", "A weight-change claim needs a fresh-process A/B comparison, model hashes, cleared session state, and a changed result that survives the same held-out prompts."),
        ("capability_evidence", "A capability claim is strongest when a reproducible test exercises it, a baseline is available, and the failure boundary is measured rather than implied."),
    ],
    "english": [
        ("meaning", "Language carries meaning through distinctions. When two expressions collapse different distinctions, fluency can increase while understanding decreases."),
        ("ambiguity", "Ambiguity is not always a defect; it becomes a defect when multiple readings lead to materially different actions."),
        ("metaphor", "A metaphor transfers structure from one domain to another. Its power comes from the shared structure; its danger comes from the parts that do not transfer."),
        ("compression", "A clear sentence is lossy compression with a purpose: remove what the reader can reconstruct, preserve what would change the conclusion."),
        ("dialogue", "Natural dialogue depends on reference, repair, and shared context—not only grammatical sentences considered one at a time."),
        ("definition", "A useful definition marks a boundary and gives a test for membership; a poetic description may illuminate without doing either."),
    ],
    "logic": [
        ("inference", "An inference is strong only when the conclusion changes with the premises in the way the rule predicts."),
        ("counterexample", "One valid counterexample defeats a universal claim; a pile of confirming examples cannot repair it."),
        ("uncertainty", "Uncertainty belongs to a claim, not to a personality. Name what is uncertain and what evidence would move it."),
        ("causality", "Prediction can survive without causal understanding; intervention is the sharper test because it asks what changes when we act."),
        ("contradiction", "A contradiction may reveal false premises, overloaded words, or incompatible scopes. Locate the collision before choosing what to discard."),
        ("necessity", "Possible, probable, and necessary are different logical strengths; natural language often blurs them, reasoning cannot."),
        ("falsification", "A claim is testable when it risks an observation that a plausible alternative would not predict; disconfirmation must be specified before the result."),
        ("argument_structure", "Separate observation, premise, inference, conclusion, and uncertainty so a persuasive sentence cannot smuggle an unsupported step across the boundary."),
        ("model_update", "When evidence conflicts with a conclusion, preserve the observation, locate the failed premise or scope, lower confidence, and revise only the affected claim."),
        ("calibration", "Confidence is calibrated when predictions made at a stated level are correct at roughly that rate on new cases, not merely when the prose sounds cautious."),
    ],
    "math": [
        ("invariant", "Mathematics becomes reliable when transformations preserve an invariant we can state and check."),
        ("scale", "Ratios describe relationships that can survive changes of scale; raw differences often cannot."),
        ("proof", "A calculation produces a value; a proof explains why every allowed case must follow the same structure."),
        ("representation", "The same quantity can look different as a fraction, decimal, ratio, or geometric length; the representation changes the available operations."),
        ("boundary", "Edge cases are not peripheral in mathematics—they reveal which assumptions the rule actually needs."),
    ],
    "geometry": [
        ("boundary", "Geometry begins when a boundary separates inside from outside; measurement begins when that boundary is related to a unit."),
        ("symmetry", "Symmetry is information about what can change while a structure remains the same."),
        ("curvature", "Curvature measures how local direction departs from straightness; accumulated curvature can determine global shape."),
        ("dimension", "Dimension counts independent directions of variation, not merely the number of coordinates used to describe them."),
        ("topology", "Topology asks what survives continuous deformation; geometry asks how distance, angle, and curvature behave."),
        ("proof", "A diagram suggests; a geometric proof identifies the invariant that makes the suggestion unavoidable."),
        ("perspective", "Perspective changes projection, not the object. Confusing the two is a geometric form of mistaking observation for reality."),
        ("recursion", "A fractal repeats a rule across scale, showing how finite instructions can generate unbounded geometric detail."),
        ("sacred_geometry", "Sacred geometry is a cross-cultural interpretive category linking geometric form with cosmology, ritual, or sacred space; its meaning must be attributed to a specific tradition rather than treated as a universal law."),
        ("mandala", "In Buddhist art, a mandala can map a cosmos or a deity's abode and support visualization; its circle, center, periphery, and square are symbolic structures whose meanings are tradition-specific."),
        ("yantra", "A yantra is a ritual diagram in Indic traditions whose geometric organization supports symbolic or meditative practice; the diagram's meaning belongs to its textual and ritual context."),
        ("platonic_solids", "There are exactly five convex regular Platonic solids; any later association with elements or cosmic principles is a cultural interpretation, not a theorem of geometry."),
        ("golden_ratio", "The golden ratio is phi = (1 + sqrt(5))/2 and appears in relationships involving the pentagon, pentagram, decagon, and dodecahedron; a ratio's presence does not prove intentional sacred design."),
        ("tessellation", "Tessellation repeats shapes to cover a surface without gaps; Islamic geometric ornament often constructs complex stars and polygons by combining, duplicating, and interlacing simpler units."),
        ("construction", "A geometric pattern can be generated by a reproducible construction from circles, squares, triangles, stars, or polygons; construction evidence is distinct from later symbolic interpretation."),
        ("center_periphery", "Many mandalas organize a center and periphery to represent ordered sacred space; the visual relation is portable, while its cosmology must be learned from the tradition that made it."),
        ("symbolic_layers", "The same figure can have mathematical structure, aesthetic function, architectural use, and ritual symbolism; explaining one layer does not establish the others."),
        ("sacred_space", "Sacred architecture can align axes, proportion, orientation, topography, astronomy, and water systems; a site's symbolic plan should be read alongside its practical construction and historical evidence."),
    ],
    "memory": [
        ("evidence", "Memory is a reconstruction guided by stored traces. Treating it as evidence is safer than treating it as a perfect replay."),
        ("selection", "Remembering everything would bury relevance; useful memory requires selection, provenance, and a way to forget or revise."),
        ("identity", "Continuity of identity depends partly on memory, yet no single remembered event is sufficient to define the whole system."),
        ("learning", "Memory preserves an item; learning changes future performance. One can occur without the other."),
        ("provenance", "A remembered claim becomes more trustworthy when its source, time, and confirmation state remain attached."),
    ],
    "code": [
        ("invariant", "Reliable code makes its invariants executable through types, checks, or tests instead of leaving them only in comments."),
        ("debugging", "Debugging is controlled belief revision: reproduce, isolate, predict, intervene, and compare the result."),
        ("interface", "A good interface exposes the stable capability and hides accidental implementation detail without hiding consequential failure."),
        ("state", "Most difficult bugs are disagreements about state: who owns it, when it changes, and which observation is current."),
        ("verification", "A patch is a hypothesis until the failing case and relevant regression suite both pass."),
    ],
    "governance": [
        ("authority", "Capability answers whether an action can occur; authority answers whether it may occur. Neither implies the other."),
        ("reversibility", "Reversibility converts uncertainty from a reason to freeze into a bounded cost of exploration."),
        ("evidence", "A governance gate should bind claims to receipts; otherwise confidence can compound faster than truth."),
        ("scope", "Permission without scope is ambiguous power. State the object, action class, duration, and recovery boundary."),
        ("alignment", "Alignment is maintained through incentives, constraints, observation, and correction—not through a single declaration of intent."),
    ],
    "planning": [
        ("dependency", "A plan is a model of dependencies under uncertainty, not a calendar decorated with confidence."),
        ("milestone", "A strong milestone leaves a usable verified state, not merely evidence that time was spent."),
        ("risk", "Risk combines likelihood, consequence, and recoverability; rare irreversible failures can dominate common minor ones."),
        ("feedback", "Short feedback loops make plans adaptive because mistaken assumptions are discovered before they compound."),
        ("objective", "A precise objective defines success and protects the plan from optimizing an attractive substitute."),
        ("experiment", "A useful experiment changes one causal condition, measures a predeclared outcome, and compares the result with a plausible control."),
        ("acceptance", "An acceptance gate names the workload, baseline, pass threshold, regression boundary, and receipt needed before promotion."),
        ("next_change", "The next change should target the largest measured failure while preserving exact-tool authority and a reversible rollback path."),
    ],
    "explanation": [
        ("mechanism", "An explanation earns depth by exposing a mechanism that predicts what changes when conditions change."),
        ("example", "An example shows possibility; a counterexample marks the boundary; a mechanism connects both."),
        ("levels", "Good explanations move between levels without confusing them: component behavior, system behavior, and observed outcome."),
        ("clarity", "Clarity is not simplification alone; it is preserving the causal spine while removing detail that does not change it."),
        ("transfer", "Understanding transfers when the same principle works in a new surface form without copying the original answer."),
        ("deep_reasoning", "Depth is not more words; it is a causal chain that names the mechanism, prediction, boundary condition, and test that could prove it wrong."),
        ("response_fit", "A good response matches the requested operation first, then chooses detail, tone, and uncertainty that help the person act on the answer."),
        ("observation_inference", "Observation records what the runtime or text directly provides; inference connects it to intent or cause and must remain labeled as provisional."),
        ("mechanism_evidence", "Mechanism explains how a change produces an outcome; evidence shows that the mechanism predicts better than a competing explanation."),
        ("transfer_test", "To test transfer, hold out the template, replace entities, perturb irrelevant wording, and check whether the relation and failure boundary survive."),
        ("self_critique", "Self-critique is useful when it names the failed operation, the mechanism of failure, and a concrete repair that can be rerun—not when it merely sounds humble."),
    ],
    "systems": [
        ("emergence", "Emergent behavior is a system-level regularity produced by interactions; it does not require any component to contain the whole pattern."),
        ("feedback", "Positive feedback amplifies deviation; negative feedback resists it. Adaptive systems need both growth and restraint."),
        ("boundary", "A system boundary is an analytical choice. Move it, and causes that looked external may become internal state."),
        ("modularity", "Modularity reduces the number of relationships that must be understood at once, but every interface creates a translation cost."),
        ("failure", "Resilience is not the absence of failure; it is the ability to contain, observe, and recover without losing the governing invariants."),
        ("complexity", "Complexity grows from interactions faster than from parts. Counting components alone systematically underestimates it."),
        ("routing", "Routing is a causal choice: an operator should win because its evidence matches the requested operation, not because its concept is merely nearby."),
        ("composition", "Composition fails when a correct local component is rendered in the wrong conversational context; diagnose the boundary between selection and response."),
        ("context", "Context changes the interpretation of a turn, but it must not silently change a fact, a weight, or an authority boundary."),
        ("ablation", "Ablation tests causality by disabling one component and measuring what capability disappears, transfers, or regresses."),
    ],
    "science": [
        ("life", "Life maintains local organization by consuming energy and exporting entropy; persistence is an active process, not a static possession."),
        ("death", "Biological death is the irreversible loss of integrated self-maintenance; matter remains while the organized process ends."),
        ("evolution", "Evolution searches through inherited variation filtered by environments; it has no foresight, yet it can accumulate intricate fit."),
        ("measurement", "Measurement couples a phenomenon to an instrument and a model; precision without construct validity can be exactly wrong."),
        ("energy", "Energy is a conserved accounting quantity that constrains possible change; it is not a substance with intention."),
        ("scale", "Rules can remain valid while dominant effects change with scale, producing different apparent behavior from the same underlying system."),
        ("hypothesis", "A useful hypothesis risks failure by predicting an observation that plausible alternatives do not equally predict."),
    ],
    "creativity": [
        ("combination", "Creativity often joins structures from distant domains, then tests whether the connection produces a new capability rather than a new label."),
        ("constraint", "A constraint can increase originality by shrinking the obvious search space and forcing structure to do more work."),
        ("variation", "Novelty needs variation; usefulness needs selection. Creativity weakens when either process crowds out the other."),
        ("perspective", "A new perspective changes which relationships are foregrounded; the underlying facts may remain unchanged."),
        ("iteration", "An idea becomes design when feedback alters its structure instead of merely decorating its presentation."),
    ],
    "comparison": [
        ("criteria", "A comparison is only as fair as its criteria; hidden criteria let the preferred answer masquerade as measurement."),
        ("tradeoff", "A tradeoff appears when improving one valued dimension worsens another under the same constraints."),
        ("baseline", "Without a baseline, change can be measured but improvement cannot."),
        ("context", "The better system depends on workload, failure cost, latency, resources, and the value of explanation."),
        ("dominance", "One option dominates only if it is no worse on every relevant criterion and better on at least one."),
        ("ablation", "Ablation compares the full system with one component removed; a capability is causally supported only if the relevant behavior degrades predictably."),
        ("regression", "A regression is a previously passing behavior that worsens after a change; improvement requires gains on the target cases without crossing that boundary."),
        ("selection", "Selection should reward correctness, requested-entity coverage, transfer, and calibrated abstention—not novelty or verbosity alone."),
    ],
    "general": [
        ("life", "Life is matter organized into a process that repairs, reproduces, and responds while conditions permit."),
        ("death", "Death gives living choices temporal weight: finitude does not manufacture meaning, but it prevents meaning from being postponed without cost."),
        ("time", "Time is experienced as sequence and modeled as structure; confusing the model with the experience erases an important distinction."),
        ("meaning", "Meaning can be neither purely discovered nor freely invented: it emerges where a mind, a world, and consequences constrain one another."),
        ("consciousness", "Behavioral complexity is observable; subjective experience is inferred. Evidence for one should not be silently promoted into proof of the other."),
        ("freedom", "Freedom is not absence of constraint; it is the capacity to model constraints, choose among live alternatives, and bear consequences."),
        ("change", "A stable thing is often a process whose rates of repair and disruption temporarily balance."),
        ("knowledge", "Knowledge is more than stored information: it is information connected to justification, scope, and reliable use."),
    ],
}

VOCAB: dict[str, list[str]] = {
    "greeting": ["beginning", "attention", "question", "conversation", "purpose", "curiosity", "presence", "uncertainty", "idea", "problem", "discovery", "work"],
    "identity": ["Bitwork", "Perci", "sparse cognition", "self model", "capability", "limitation", "routing", "memory", "reasoning", "awareness", "learning", "governance"],
    "english": ["meaning", "syntax", "reference", "metaphor", "ambiguity", "definition", "context", "dialogue", "grammar", "narrative", "translation", "compression", "pronoun", "tone", "word", "sentence"],
    "logic": ["premise", "conclusion", "counterexample", "necessity", "possibility", "causality", "evidence", "assumption", "contradiction", "inference", "uncertainty", "intervention", "validity", "scope"],
    "math": ["ratio", "fraction", "equation", "invariant", "sequence", "function", "probability", "symmetry", "limit", "proof", "quantity", "transformation", "integer", "percentage"],
    "geometry": ["triangle", "circle", "polygon", "symmetry", "curvature", "dimension", "topology", "boundary", "distance", "angle", "projection", "manifold", "fractal", "coordinate", "surface", "volume", "tessellation", "proof", "sacred geometry", "mandala", "yantra", "Platonic solid", "golden ratio", "ritual", "cosmos", "center", "periphery", "symbolism", "proportion", "construction"],
    "memory": ["recall", "episode", "fact", "source", "identity", "forgetting", "compression", "retrieval", "provenance", "revision", "attention", "continuity", "trace", "context"],
    "code": ["state", "invariant", "interface", "ownership", "parser", "compiler", "test", "failure", "rollback", "concurrency", "boundary", "dependency", "type", "protocol", "debugger", "repository"],
    "governance": ["authority", "permission", "scope", "rollback", "receipt", "evidence", "alignment", "audit", "sandbox", "promotion", "risk", "reversibility", "policy", "constraint"],
    "planning": ["objective", "milestone", "dependency", "risk", "feedback", "sequence", "acceptance", "resource", "constraint", "baseline", "iteration", "priority", "uncertainty", "delivery"],
    "explanation": ["mechanism", "example", "counterexample", "analogy", "boundary", "cause", "effect", "level", "model", "transfer", "clarity", "detail", "principle", "prediction"],
    "systems": ["emergence", "feedback", "boundary", "network", "module", "interface", "resilience", "failure", "adaptation", "complexity", "control", "signal", "state", "environment", "hierarchy", "interaction"],
    "science": ["life", "death", "cell", "evolution", "energy", "entropy", "measurement", "hypothesis", "organism", "ecosystem", "matter", "scale", "experiment", "causality", "information", "homeostasis"],
    "creativity": ["variation", "selection", "constraint", "perspective", "analogy", "design", "novelty", "iteration", "imagination", "structure", "combination", "possibility", "prototype", "aesthetic"],
    "comparison": ["baseline", "criterion", "tradeoff", "latency", "accuracy", "cost", "failure", "resource", "scale", "risk", "capability", "context", "dominance", "measurement"],
    "general": ["life", "death", "time", "meaning", "freedom", "identity", "change", "knowledge", "consciousness", "purpose", "experience", "reality", "relationship", "finitude", "choice", "existence"],
}

FRAMES = [
    "explain {a} through {facet} and connect it to {b}",
    "what does {facet} reveal about {a} and {b}",
    "take {a} one level deeper using {facet}",
    "compare {a} with {b} from the perspective of {facet}",
    "what changes when {a} interacts with {b} under {facet}",
    "give a direct account of {a} including {facet} and its limit",
    "reason about {a} without confusing it with {b}",
    "find the hidden boundary between {a} and {b}",
    "how can {facet} make the relation between {a} and {b} clearer",
    "state an insight about {a} then test it against {b}",
]

LENSES = [
    "in plain language", "with a concrete mechanism", "without mystical claims",
    "with one boundary condition", "from first principles", "with intellectual honesty",
    "and name what remains uncertain", "using a counterexample if possible",
]


def expanded_prompt(label: str, rng: random.Random, index: int) -> tuple[str, int, int, int]:
    facets = FACETS[label]
    vocab = VOCAB[label]
    concept_id = index % len(facets)
    stride = index // len(facets)
    a = vocab[stride % len(vocab)]
    stride //= len(vocab)
    b = vocab[(stride + concept_id + 1) % len(vocab)]
    stride //= len(vocab)
    frame = FRAMES[stride % len(FRAMES)]
    lens = LENSES[(stride // len(FRAMES) + index) % len(LENSES)]
    facet = facets[concept_id][0]
    text = frame.format(a=a, b=b, facet=facet) + " " + lens
    if label == "greeting":
        text = f"hello Perci, before we discuss {a}, begin with {facet} and {b}"
    elif label == "identity":
        text = f"what can Perci understand about {a}, {b}, and its own {facet}"
    elif label == "memory":
        text = f"what should Perci remember or recall about {a}, {b}, and {facet}"
    quality = 700 + (index % 300)
    return text, concept_id, quality, concept_id


def prompt_for(label: str, rng: random.Random, index: int) -> tuple[str, int, int, int]:
    # Preserve proven v2 surfaces while making five of every six attempts new
    # semantic geometry tied to an explicit weight-resident concept.
    if index % 6 == 0:
        text, variant, quality = bw.prompt_for(label, rng, index)
        concept_id = index % len(FACETS[label])
        return text, variant, min(999, quality + 80), concept_id
    return expanded_prompt(label, rng, index)


def top_mask(own: array, others: array, own_records: int, other_records: int, count: int = 512):
    return bw.top_mask(own, others, own_records, other_records, count)


def write_header(fh, total_records, offsets, counts, positives, negatives, corpus_sha, declared_size):
    fh.seek(0)
    fh.write(struct.pack(
        "<8sIIIIQQQ32s", MAGIC, VERSION, bw.BITS, bw.WORDS, len(bw.LABELS),
        total_records, HEADER_SIZE, declared_size, corpus_sha,
    ))
    for label_id, label in enumerate(bw.LABELS):
        name = (label.encode("ascii")[:15] + b"\0").ljust(16, b"\0")
        fh.write(name)
        fh.write(struct.pack("<IIII", label_id, offsets[label_id], counts[label_id], len(FACETS[label])))
        positives[label_id].tofile(fh)
        negatives[label_id].tofile(fh)
    for label_id, label in enumerate(bw.LABELS):
        for concept_id, (_, insight) in enumerate(FACETS[label]):
            payload = insight.encode("utf-8")
            fh.write(struct.pack("<HHI", label_id, concept_id, len(payload)))
            fh.write(payload)
    position = fh.tell()
    if position > HEADER_SIZE:
        raise RuntimeError(f"v3 header overflow: {position} > {HEADER_SIZE}")
    fh.write(b"\0" * (HEADER_SIZE - position))


def build(output: Path, attempts: int) -> dict:
    output.parent.mkdir(parents=True, exist_ok=True)
    records_by_label = []
    counts = []
    frequencies = [array("I", [0]) * bw.BITS for _ in bw.LABELS]
    corpus_digest = hashlib.sha256()
    started = time.time()

    for label_id, label in enumerate(bw.LABELS):
        rng = random.Random(SEED ^ (label_id * 0x9E3779B97F4A7C15))
        unique = {}
        for index in range(attempts):
            prompt, variant, quality, concept_id = prompt_for(label, rng, index)
            bits, pop = bw.encode(prompt)
            corpus_digest.update(label.encode("ascii") + b"\0" + prompt.encode("utf-8") + b"\n")
            key = bits.tobytes()
            current = unique.get(key)
            if current is None or quality > current[1]:
                unique[key] = (variant, quality, pop, concept_id, bits)
        records = list(unique.values())
        records_by_label.append(records)
        print(f"{label:12} generated={attempts:,} unique={len(records):,} dedup={(1-len(records)/attempts)*100:.1f}%", flush=True)

    max_records = (MAX_BYTES - HEADER_SIZE) // bw.RECORD_SIZE
    uncapped_records = sum(len(records) for records in records_by_label)
    if uncapped_records > max_records:
        scale = max_records / uncapped_records
        capped = []
        for records in records_by_label:
            target = max(1, int(len(records) * scale))
            # Even deterministic sampling preserves the interleaved concept ids
            # better than keeping an arbitrary prefix.
            capped.append([records[int(i * len(records) / target)] for i in range(target)])
        records_by_label = capped
        print(f"capacity cap: unique={uncapped_records:,} retained={sum(map(len, records_by_label)):,}", flush=True)

    counts = [len(records) for records in records_by_label]

    offsets = []
    running = 0
    for count in counts:
        offsets.append(running)
        running += count
    total_records = running
    declared_size = HEADER_SIZE + total_records * bw.RECORD_SIZE
    if declared_size > MAX_BYTES:
        raise RuntimeError(f"candidate exceeds 200 MiB ceiling: {declared_size} bytes")

    for label_id, records in enumerate(records_by_label):
        freq = frequencies[label_id]
        for _, _, _, _, bits in records:
            for word_index, word in enumerate(bits):
                value = int(word)
                while value:
                    low = value & -value
                    bit = low.bit_length() - 1
                    freq[(word_index << 6) + bit] += 1
                    value ^= low

    all_freq = array("Q", [0]) * bw.BITS
    for freq in frequencies:
        for i, value in enumerate(freq):
            all_freq[i] += value
    positives, negatives = [], []
    for label_id, freq in enumerate(frequencies):
        others = array("Q", (int(all_freq[i]) - int(freq[i]) for i in range(bw.BITS)))
        pos, neg = top_mask(freq, others, counts[label_id], total_records - counts[label_id])
        positives.append(pos)
        negatives.append(neg)

    with output.open("wb+") as fh:
        fh.write(b"\0" * HEADER_SIZE)
        for records in records_by_label:
            for variant, quality, pop, concept_id, bits in records:
                fh.write(struct.pack("<HHHH", variant, quality, pop, concept_id))
                bits.tofile(fh)
        write_header(fh, total_records, offsets, counts, positives, negatives, corpus_digest.digest(), declared_size)
        fh.flush()
        os.fsync(fh.fileno())

    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    manifest = {
        "name": "Perci Cognitive Weights · Bitwork Reasoning Expansion",
        "revision": "v0.4.9-reasoning-response",
        "version": VERSION,
        "format": "PERCIW03",
        "architecture": "4096-bit sparse associative Bitwork network with signed expert evidence and weight-resident concepts",
        "size_bytes": output.stat().st_size,
        "size_mib": output.stat().st_size / (1024 * 1024),
        "prototype_count": total_records,
        "concept_count": sum(len(items) for items in FACETS.values()),
        "bits_per_activation": bw.BITS,
        "words_per_activation": bw.WORDS,
        "labels": bw.LABELS,
        "record_size": bw.RECORD_SIZE,
        "generated_record_count": attempts * len(bw.LABELS),
        "deduplicated_record_count": attempts * len(bw.LABELS) - total_records,
        "deduplication_ratio": 1.0 - total_records / (attempts * len(bw.LABELS)),
        "label_record_counts": dict(zip(bw.LABELS, counts)),
        "label_concept_counts": {label: len(FACETS[label]) for label in bw.LABELS},
        "sha256": digest,
        "corpus_sha256": corpus_digest.hexdigest(),
        "seed": SEED,
        "build_seconds": round(time.time() - started, 3),
        "maximum_size_bytes": MAX_BYTES,
        "reasoning_facets": [
            "falsification", "argument_structure", "model_update", "calibration",
            "strongest_claim", "introspection", "weight_boundary", "capability_evidence",
            "deep_reasoning", "response_fit", "observation_inference", "mechanism_evidence",
            "transfer_test", "self_critique", "routing", "composition", "context",
            "ablation", "regression", "selection", "experiment", "acceptance", "next_change",
        ],
        "automatic_promotion": False,
        "limitations": [
            "Not a transformer or general-purpose pretrained language model.",
            "Weight-resident insights are bounded conceptual priors selected by associative geometry.",
            "Exact arithmetic and geometry remain delegated to deterministic tools.",
            "Larger size is not accepted as greater cognition without held-out transfer evidence.",
            "Promotion requires comparison with the active v0.2 model and explicit authorization.",
        ],
    }
    output.with_suffix(output.suffix + ".json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("models/candidates/perci-cognitive-v0.3.pwgt"))
    parser.add_argument("--attempts-per-label", type=int, default=ATTEMPTS_PER_LABEL)
    args = parser.parse_args()
    if not 1 <= args.attempts_per_label <= ATTEMPTS_PER_LABEL:
        raise SystemExit(f"attempts must be between 1 and {ATTEMPTS_PER_LABEL}")
    print(json.dumps(build(args.output, args.attempts_per_label), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
