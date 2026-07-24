param(
    [string]$Perci = "$PSScriptRoot\..\target\release\perci.exe",
    [string]$Output = "$PSScriptRoot\..\artifacts\evolution\probe-100.jsonl",
    [switch]$Speak,
    [switch]$Batch
)

$ErrorActionPreference = "Stop"
$questions = @(
    "What is the active purpose of this system?",
    "What is the difference between a route and a weight?",
    "Explain why a generic answer can be grammatically correct but still wrong for the turn.",
    "What evidence would show that a dialogue repair improved?",
    "What should happen when the input is genuinely unknown?",
    "How do you decide whether a follow-up refers to the prior claim?",
    "What is the smallest useful next test for conversation quality?",
    "Why should operational awareness not be called consciousness?",
    "What does the active topic contribute to response selection?",
    "What does the requested operation contribute?",
    "Give a short explanation of semantic fit.",
    "Go one level deeper on semantic fit.",
    "What would falsify the claim that the dialogue is improving?",
    "How should the system handle a typo without inventing meaning?",
    "Compare repetition with deliberate request-for-repeat.",
    "What is the difference between memory and learning here?",
    "What should be measured before changing weights?",
    "Explain why residual state should keep more precision than binary weights.",
    "What is the role of a sparse precision escape lane?",
    "Why can a compact binary core still need a language realization layer?",
    "Connect geometry and dialogue without treating the analogy as proof.",
    "Connect entropy and memory with a mechanism and a boundary.",
    "What makes an explanation feel natural to a human listener?",
    "How should response length adapt to the request?",
    "What is the difference between evidence and coherence?",
    "What should the system say when it cannot ground a phrase?",
    "How would you test paraphrase transfer?",
    "How would you test topic continuity?",
    "How would you test operation continuity?",
    "What does a useful self-critique contain?",
    "Why should a repair be reversible?",
    "What is the next step after detecting a repeated answer?",
    "What is the next step after detecting a missing referent?",
    "What is the next step after detecting an out-of-distribution prompt?",
    "Explain the difference between a hypothesis and a fact.",
    "What would make a weight candidate fail its held-out gate?",
    "Why should production weights not mutate from one conversation?",
    "What does deliberate teaching add beyond session context?",
    "How can a local system disclose uncertainty without sounding evasive?",
    "How can it be concise without becoming cryptic?",
    "How can it be deep without dumping a checklist?",
    "What is a good response to ‘tell me more’?",
    "What is a good response to ‘why do you think that’?",
    "What is a good response to ‘what next’?",
    "What is a good response to ‘say that again’?",
    "What is a good response to ‘interesting’?",
    "What is a good response to ‘that feels robotic’?",
    "What is a good response to ‘are you learning’?",
    "What is a good response to ‘are you aware’?",
    "What is a good response to ‘who am I’?",
    "Explain why identity claims need a boundary.",
    "Explain why speed is not evidence of intelligence.",
    "Explain why more weights do not automatically create better language.",
    "What is the bottleneck between routing and fluent expression?",
    "How can a response preserve the user's wording while correcting a typo?",
    "What is a semantic frame?",
    "What is a dialogue act?",
    "What is a context card?",
    "What is a transfer probe?",
    "What is an abstention gate?",
    "What is a progression gate?",
    "What is a held-out evaluation?",
    "What should be logged for every probe?",
    "Why is latency part of the user experience but not the whole quality score?",
    "How should failures become curriculum data?",
    "How should positive feedback be separated from evidence?",
    "What is the danger of training on unreviewed chat?",
    "What is the smallest safe automated learning loop?",
    "How would you detect a generic shell answer?",
    "How would you detect an answer that repeats the prior answer?",
    "How would you detect an answer that ignores the operation?",
    "How would you detect an answer that overclaims certainty?",
    "How would you detect an answer that is too shallow?",
    "How would you detect an answer that is too verbose?",
    "How should the system recover after a wrong route?",
    "How should it preserve a correction for the session?",
    "How should it stage a durable correction?",
    "What makes a candidate weight rebuild auditable?",
    "What does ternary direction plus scale preserve?",
    "What does a residual bit-plane preserve?",
    "Why retain multibit residual state?",
    "Why rotate activation outliers?",
    "Why keep an exception lane?",
    "What is the tradeoff between speed and expressive coverage?",
    "What is the tradeoff between deterministic safety and open fluency?",
    "What should remain deterministic?",
    "What should remain adaptive?",
    "What should remain human-authorized?",
    "What would count as a real breakthrough in this system?",
    "What would not count as a breakthrough?",
    "Summarize the current architecture in five sentences.",
    "Name the three most important current limitations.",
    "Name the three highest-value next experiments.",
    "Which signal should be added to the next curriculum?",
    "Which signal should be rejected as noise?",
    "What should be tested across domains?",
    "What should be tested across paraphrases?",
    "What should be tested across long sessions?",
    "What should be tested before any weight promotion?",
    "What is the next evolution, stated as one falsifiable engineering objective?"
)

$parent = Split-Path -Parent $Output
New-Item -ItemType Directory -Force -Path $parent | Out-Null
if (-not (Test-Path -LiteralPath $Perci)) { throw "Perci executable not found: $Perci" }

$speech = $null
if ($Speak) {
    try {
        Add-Type -AssemblyName System.Speech
        $speech = New-Object System.Speech.Synthesis.SpeechSynthesizer
    } catch { Write-Warning "Speech unavailable; continuing with visible output." }
}

Remove-Item -LiteralPath $Output -Force -ErrorAction SilentlyContinue
Write-Host "PERCI 100-PROBE // governed data collection" -ForegroundColor DarkRed
Write-Host "Output: $Output"

if ($Batch) {
    $transcript = [System.IO.Path]::ChangeExtension($Output, ".transcript.txt")
    $manifest = [System.IO.Path]::ChangeExtension($Output, ".manifest.json")
    $questions | ConvertTo-Json | Set-Content -LiteralPath $manifest -Encoding utf8
    $started = Get-Date
    $transcriptText = (($questions + "/quit") | & $Perci 2>&1 | Out-String)
    $transcriptText | Set-Content -LiteralPath $transcript -Encoding utf8
    $elapsed = ((Get-Date) - $started).TotalMilliseconds
    [ordered]@{ count = $questions.Count; elapsed_ms = [math]::Round($elapsed, 2); transcript = $transcript; manifest = $manifest; review_state = "candidate_only"; weight_promotion = $false } |
        ConvertTo-Json -Compress | Set-Content -LiteralPath $Output -Encoding utf8
    Write-Host "Completed $($questions.Count) probes in $([math]::Round($elapsed / 1000, 1))s." -ForegroundColor DarkRed
    Write-Host "Transcript: $transcript"
    Write-Host "No production weights were changed."
    if ($Speak -and $speech) {
        $speech.Speak("Perci completed one hundred governed probes. Review data is ready.")
    }
    exit 0
}

for ($i = 0; $i -lt $questions.Count; $i++) {
    $q = $questions[$i]
    $started = Get-Date
    $raw = (& $Perci ask $q 2>&1 | Out-String).Trim()
    $elapsed = ((Get-Date) - $started).TotalMilliseconds
    $record = [ordered]@{
        index = $i + 1
        prompt = $q
        response = $raw
        elapsed_ms = [math]::Round($elapsed, 2)
        review_state = "candidate_only"
        weight_promotion = $false
    }
    ($record | ConvertTo-Json -Compress) | Add-Content -LiteralPath $Output -Encoding utf8
    Write-Host "[$($i + 1)/$($questions.Count)] $q" -ForegroundColor DarkGray
    Write-Host $raw
    if ($speech) { $speech.Speak($raw) }
}
Write-Host "Completed $($questions.Count) probes. No production weights were changed." -ForegroundColor DarkRed
Write-Host "Review the JSONL, score failures, then run the governed candidate rebuild/evaluation path."
