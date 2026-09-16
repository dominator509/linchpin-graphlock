//! Mutation-based fuzz campaign over the product's pure parsing and validation
//! paths (DOD-038's fuzz term; the capability registry's GEN-027).
//!
//! WHY A SEEDED, IN-TEST FUZZER RATHER THAN A FUZZING FRAMEWORK
//!   * the repository has no fuzzing engine configured (GEN-027 recorded that as
//!     an absence, which this campaign replaces with executed evidence);
//!   * adding `cargo-fuzz`/`libfuzzer` would introduce a new dependency and a
//!     nightly-toolchain requirement for paths that are pure functions over
//!     `&str` -- the capability being tested is the parser, not the fuzzer;
//!   * a deterministic campaign is REPRODUCIBLE: the seed, the corpus and every
//!     finding are recorded, so a failure can be replayed exactly.
//!
//! WHAT IT DOES
//!   For each target it takes a seed corpus, applies mutation operators
//!   (byte flips, insertions, deletions, duplications, splices, and injections
//!   from a set of deliberately awkward characters including multi-byte and
//!   case-folding ones), and runs the function inside `catch_unwind`. A panic is
//!   a FINDING: the input is recorded and then SHRUNK by the same deterministic
//!   mutator until it stops reproducing, so the report names a small reproducer
//!   instead of a wall of noise.
//!
//! WHAT A FINDING MEANS
//!   Every target is a pure function over text. A panic is therefore never an
//!   acceptable answer for ANY input: these functions are reached from IPC with
//!   user-supplied text, and a panic inside a Tauri command is a failed request
//!   at best and an aborted process at worst. Returning `Err` for garbage is
//!   correct; unwinding is not.
//!
//! LABEL
//!   The campaign is ABBREVIATED relative to any full-scale fuzzing requirement:
//!   no duration, corpus size or iteration count for fuzzing is specified in the
//!   repository. The report says so and the run does NOT claim DOD-038.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Deterministic PRNG (xorshift64*). Small, dependency-free, and identical on
/// every platform, which is what makes a finding reproducible from its seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next() % bound as u64) as usize
        }
    }
}

/// Characters that have historically broken byte-indexed code: a multi-byte
/// uppercase whose lowercase is a different LENGTH, a letter that folds to ASCII,
/// a combining mark, a NUL, a BOM, and a few separators the guard logic keys on.
const AWKWARD_CHARS: [&str; 14] = [
    "İ", // U+0130: lowercases to "i̇", THREE bytes -- offsets shift when lowercased
    "K", // U+212A Kelvin: lowercases to ASCII "k", length changes
    "ß", // U+00DF: uppercase is "SS"
    "é", "ü", "\u{0301}", "\u{0}", "\u{feff}", "🦀", "=", ":", "\"", "{", "}",
];

const SEED_CORPUS: [&str; 12] = [
    "",
    " ",
    "1. A device comprising a valve seat.",
    "api_key=abcdef",
    "{\"token\":\"abc123\"}",
    "password: hunter2",
    "guaranteed patent",
    "2023-01-15",
    "REEL/FRAME 1234/0567",
    "Acknowledgement Receipt\nApplication Number: 17/123,456\nConfirmation Number: 1234",
    "The AI is the inventor of record.",
    "claims and specification text with [0042] anchors",
];

/// One mutation operator, named so the report can say HOW an input was produced.
///
/// MEASURED HARNESS WEAKNESS THIS FIXES: the first version ran 20 000 iterations,
/// reported zero findings, and MISSED a real panic one edit away
/// (`"İ" + "api_key=secretvalue"`). The cause was corpus dilution -- the pool
/// reached its cap within a few hundred iterations, so the probability of
/// mutating one of the twelve SEED strings (the only ones containing the key names
/// the panic needs) fell to about 2%. A campaign that cannot find a bug that close
/// to the surface is not evidence of absence.
fn mutate(rng: &mut Rng, input: &str, corpus: &[String]) -> (String, &'static str) {
    let mut bytes = input.as_bytes().to_vec();
    match rng.below(10) {
        0 => {
            if !bytes.is_empty() {
                let index = rng.below(bytes.len());
                bytes[index] = (rng.next() & 0xFF) as u8;
            }
            (String::from_utf8_lossy(&bytes).into_owned(), "byte-flip")
        }
        1 => {
            let index = rng.below(bytes.len() + 1);
            let ch = AWKWARD_CHARS[rng.below(AWKWARD_CHARS.len())];
            bytes.splice(index..index, ch.as_bytes().iter().copied());
            (
                String::from_utf8_lossy(&bytes).into_owned(),
                "awkward-char-insert",
            )
        }
        2 => {
            if !bytes.is_empty() {
                let index = rng.below(bytes.len());
                let len = 1 + rng.below((bytes.len() - index).min(8));
                bytes.drain(index..index + len);
            }
            (String::from_utf8_lossy(&bytes).into_owned(), "delete")
        }
        3 => {
            if !bytes.is_empty() {
                let index = rng.below(bytes.len());
                let len = 1 + rng.below((bytes.len() - index).min(16));
                let slice = bytes[index..index + len].to_vec();
                let at = rng.below(bytes.len() + 1);
                bytes.splice(at..at, slice);
            }
            (String::from_utf8_lossy(&bytes).into_owned(), "duplicate")
        }
        4 => {
            if !corpus.is_empty() {
                let other = &corpus[rng.below(corpus.len())];
                let cut = rng.below(bytes.len() + 1);
                let mut spliced = bytes[..cut].to_vec();
                spliced.extend_from_slice(other.as_bytes());
                bytes = spliced;
            }
            (
                String::from_utf8_lossy(&bytes).into_owned(),
                "splice-corpus",
            )
        }
        5 => {
            let ch = AWKWARD_CHARS[rng.below(AWKWARD_CHARS.len())];
            (format!("{input}{ch}"), "awkward-char-append")
        }
        6 => {
            let ch = AWKWARD_CHARS[rng.below(AWKWARD_CHARS.len())];
            (format!("{ch}{input}"), "awkward-char-prepend")
        }
        // The productive shape the first campaign under-sampled: an awkward
        // character inserted char-wise, which shifts every following byte offset
        // while keeping the string valid UTF-8.
        7 => {
            let ch = AWKWARD_CHARS[rng.below(AWKWARD_CHARS.len())];
            let split = rng.below(input.chars().count() + 1);
            let head: String = input.chars().take(split).collect();
            let tail: String = input.chars().skip(split).collect();
            (format!("{head}{ch}{tail}"), "awkward-char-insert-charwise")
        }
        8 => {
            let ch = AWKWARD_CHARS[rng.below(AWKWARD_CHARS.len())];
            let split = rng.below(input.chars().count() + 1);
            let head: String = input.chars().take(split).collect();
            let tail: String = input.chars().skip(split).collect();
            (format!("{head}{tail}{ch}"), "awkward-char-append-charwise")
        }
        _ => {
            let lower = input.to_lowercase();
            (lower, "lowercase")
        }
    }
}

/// A target: a named pure function over text.
struct Target {
    name: &'static str,
    run: fn(&str),
}

fn targets() -> Vec<Target> {
    vec![
        Target {
            name: "domain::scope::check_claim_text",
            run: |input| {
                let _ = domain::scope::check_claim_text(input);
            },
        },
        Target {
            name: "crash_reporter::RedactionPolicy::apply",
            // Only an ASCII secret is registered. MEASURED: registering "İ" here
            // scrubbed the case-folding trigger BEFORE the key-value pass ran, so
            // the campaign reported clean while a direct probe of the same code
            // panicked -- the fixture deleted the condition under test.
            run: |input| {
                let policy =
                    crash_reporter::RedactionPolicy::new().with_secret("registered-secret");
                let _ = policy.apply(input);
            },
        },
        Target {
            name: "evidence::InputHardener::sanitize_path",
            run: |input| {
                let _ = evidence::InputHardener::sanitize_path(input);
            },
        },
        Target {
            name: "evidence::InputHardener::sanitize_archive_entry",
            run: |input| {
                let _ = evidence::InputHardener::sanitize_archive_entry(input);
            },
        },
        Target {
            name: "evidence::LogRedactor::redact_with_report",
            run: |input| {
                let mut redactor = evidence::LogRedactor::new();
                redactor.register_secret("registered-secret".to_string());
                let _ = redactor.redact_with_report(input);
            },
        },
        Target {
            name: "patent::PatentLinter::lint_claims",
            run: |input| {
                let _ = patent::PatentLinter::lint_claims(input);
            },
        },
        Target {
            name: "patent::ReceiptImport::import",
            run: |input| {
                let _ = patent::ReceiptImport::import(input);
            },
        },
        Target {
            name: "patent::PackageBuilder::build_docx",
            run: |input| {
                let _ = patent::PackageBuilder::build_docx(input, input);
            },
        },
        Target {
            name: "domain::RuleSetAuthority::new",
            run: |input| {
                let _ = domain::RuleSetAuthority::new(input, input);
            },
        },
        Target {
            name: "commercialization::asset_readiness date validation",
            run: |input| {
                let record = commercialization::asset_readiness::TitleRecord {
                    kind: commercialization::asset_readiness::TitleRecordKind::InventorRecord,
                    effective_date: input.to_string(),
                    party_from: None,
                    party_to: Some("party".to_string()),
                    source: "fuzz".to_string(),
                    recordation_id: None,
                };
                let _ = commercialization::asset_readiness::assess_asset_readiness(
                    "fuzz asset",
                    &[record],
                    None,
                    &[],
                    &[],
                    &[],
                );
            },
        },
    ]
}

/// Deterministic adversarial pairings: the small space that random mutation only
/// samples by luck.
///
/// MEASURED WHY THIS EXISTS: a deliberate mutation proof that reintroduced the
/// lowercasing-offset bug came back SURVIVED -- 20 000 random iterations across
/// ten targets never produced `"İ" + "api_key=secretvalue"`, so the campaign's
/// silence proved nothing about that class. Every awkward character inserted at
/// every character position of every seed is only a few thousand inputs, so it is
/// enumerated exhaustively before the random phase instead of hoped for.
fn adversarial_inputs() -> Vec<(String, &'static str)> {
    let mut inputs: Vec<(String, &'static str)> = Vec::new();
    for seed in SEED_CORPUS {
        for ch in AWKWARD_CHARS {
            inputs.push((format!("{ch}{seed}"), "adversarial-prepend"));
            inputs.push((format!("{seed}{ch}"), "adversarial-append"));
            let chars: Vec<char> = seed.chars().collect();
            for split in 0..=chars.len() {
                let head: String = chars[..split].iter().collect();
                let tail: String = chars[split..].iter().collect();
                inputs.push((format!("{head}{ch}{tail}"), "adversarial-insert"));
            }
        }
    }
    inputs
}

struct Finding {
    target: String,
    operator: String,
    input: String,
    panic: String,
}

fn describe_panic(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// Run one input, returning the panic message when it unwinds.
fn run_catching(target: &Target, input: &str) -> Option<String> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(|| (target.run)(input)));
    std::panic::set_hook(previous);
    result.err().map(describe_panic)
}

/// Shrink a failing input deterministically: drop characters from the end while
/// the failure still reproduces, then from the front, then per byte.
fn shrink(target: &Target, input: &str) -> String {
    let mut best = input.to_string();
    loop {
        let mut improved = false;
        for candidate in [
            best.chars().skip(1).collect::<String>(),
            best.chars()
                .rev()
                .skip(1)
                .collect::<String>()
                .chars()
                .rev()
                .collect(),
            best.chars()
                .take(best.chars().count() / 2)
                .collect::<String>(),
        ] {
            if candidate.len() < best.len() && run_catching(target, &candidate).is_some() {
                best = candidate;
                improved = true;
            }
        }
        if !improved {
            return best;
        }
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[test]
fn mutation_fuzz_campaign_finds_no_panics_in_parsing_paths() {
    let seed = env_usize("LINCHPIN_FUZZ_SEED", 0x5EED_1234_ABCD) as u64;
    let iterations = env_usize("LINCHPIN_FUZZ_ITERATIONS", 2_000);
    let max_seconds = env_usize("LINCHPIN_FUZZ_SECONDS", 180) as u64;

    let mut rng = Rng::new(seed);
    let mut corpus: Vec<String> = SEED_CORPUS.iter().map(|s| (*s).to_string()).collect();
    let targets = targets();
    let mut findings: Vec<Finding> = Vec::new();
    let mut per_target: Vec<serde_json::Value> = Vec::new();
    let started = Instant::now();
    let deadline = started + Duration::from_secs(max_seconds);

    let adversarial = adversarial_inputs();
    for target in &targets {
        let target_started = Instant::now();
        let mut executed = 0usize;

        // Deterministic phase first: no seed, no luck, every pairing.
        for (input, operator) in &adversarial {
            if Instant::now() >= deadline {
                break;
            }
            executed += 1;
            if let Some(panic) = run_catching(target, input) {
                findings.push(Finding {
                    target: target.name.to_string(),
                    operator: (*operator).to_string(),
                    input: input.clone(),
                    panic,
                });
            }
        }

        'iterations: for _ in 0..iterations {
            if Instant::now() >= deadline {
                break 'iterations;
            }
            // Half the iterations start from a SEED string, because the seed
            // strings carry the key names and literals the targets actually act
            // on; the other half explore the grown corpus.
            let base = if rng.below(2) == 0 {
                SEED_CORPUS[rng.below(SEED_CORPUS.len())].to_string()
            } else {
                corpus[rng.below(corpus.len())].clone()
            };
            let (input, operator) = mutate(&mut rng, &base, &corpus);
            executed += 1;
            if let Some(panic) = run_catching(target, &input) {
                let reduced = shrink(target, &input);
                findings.push(Finding {
                    target: target.name.to_string(),
                    operator: operator.to_string(),
                    input: reduced.clone(),
                    panic,
                });
                // Keep the reproducer in the corpus: a mutation that reaches new
                // behaviour is worth exploring further.
                if corpus.len() < 512 {
                    corpus.push(reduced);
                }
            } else if corpus.len() < 512 && !input.is_empty() {
                corpus.push(input);
            }
        }
        per_target.push(serde_json::json!({
            "target": target.name,
            "iterations": executed,
            "seconds": (target_started.elapsed().as_secs_f64() * 100.0).round() / 100.0,
        }));
    }

    let report = serde_json::json!({
        "campaign": "mutation-fuzz",
        "label": "ABBREVIATED",
        "covers": ["DOD-038", "GEN-027"],
        "harness": "apps/desktop/src-tauri/tests/fuzz_campaign.rs",
        "seed": seed,
        "iterations_per_target": iterations,
        "max_seconds": max_seconds,
        "actual_seconds": (started.elapsed().as_secs_f64() * 100.0).round() / 100.0,
        "adversarial_inputs_per_target": adversarial.len(),
        "targets": per_target,
        "corpus_size": corpus.len(),
        "findings": findings.iter().map(|finding| serde_json::json!({
            "target": finding.target,
            "operator": finding.operator,
            "input": finding.input,
            "panic": finding.panic,
        })).collect::<Vec<_>>(),
        "finding_count": findings.len(),
        "label_reason": "no fuzzing duration, corpus size or iteration count is specified in the repository, so this campaign is labeled ABBREVIATED and does not claim DOD-038 at full scale",
        "limits": [
            "pure functions over text; no network, filesystem or IPC transport is fuzzed",
            "deterministic seeded mutations, not coverage-guided; GEN-025 remains an accurate absence",
            "no grammar model of the input languages; GEN-026 remains an accurate absence",
        ],
    });

    let evidence = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.agent/evidence/fuzz");
    assert!(
        evidence
            .parent()
            .is_some_and(|parent| parent.ends_with(".agent/evidence")),
        "the evidence path escaped the repository: {}",
        evidence.display()
    );
    std::fs::create_dir_all(&evidence).expect("create evidence dir");
    std::fs::write(
        evidence.join("report.json"),
        serde_json::to_string_pretty(&report).expect("serialize") + "\n",
    )
    .expect("write report");

    println!(
        "fuzz: {} targets, seed {seed}, {} iteration slots, {} finding(s), corpus {}",
        targets.len(),
        iterations,
        findings.len(),
        corpus.len()
    );
    for finding in &findings {
        println!(
            "fuzz FINDING {} [{}]: {:?} -> {}",
            finding.target, finding.operator, finding.input, finding.panic
        );
    }

    assert!(
        findings.is_empty(),
        "the fuzz campaign found {} panic(s); the first is {} on {:?}",
        findings.len(),
        findings[0].target,
        findings[0].input
    );
}

/// Kept out of the campaign above so the test file has no unused imports when the
/// evidence directory is the only thing written.
#[allow(dead_code)]
fn unused(_path: PathBuf, _nanos: u128) {
    let _ = SystemTime::now().duration_since(UNIX_EPOCH);
}
