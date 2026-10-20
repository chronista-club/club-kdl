//! Robustness checks for the hand-written parser and writer.
//!
//! - Mutated KDL (the official spec cases with bytes inserted, removed,
//!   replaced, duplicated or truncated) must never panic, whether parsed into
//!   a tree or straight into derived structs, and whatever parses must
//!   survive write → parse unchanged.
//! - Arbitrary strings stored in struct fields must survive
//!   `to_string` → `from_str`, so the writer's quoting and escaping hold for
//!   any input, not just the spec examples.
//!
//! The inputs come from a fixed-seed generator, so failures reproduce. CI
//! runs a fast subset; for a deep run, scale the iteration counts up:
//!
//! ```sh
//! CLUB_KDL_FUZZ_SCALE=50 cargo test --release --test robustness
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use club_kdl::{Document, KdlDeserialize, KdlSerialize};

/// `base` scaled by `CLUB_KDL_FUZZ_SCALE` (default 1).
fn iterations(base: usize) -> usize {
    let scale = std::env::var("CLUB_KDL_FUZZ_SCALE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    base * scale.max(1)
}

/// xorshift64: a tiny deterministic generator, so no dev-dependency is needed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Fragments that tend to hit parser edge cases: delimiters, escapes,
/// comment and string openers, number prefixes, keywords, non-ASCII,
/// newline variants, disallowed code points.
const PIECES: &[&str] = &[
    "\"", "\\", "#", "/", "*", "{", "}", ";", "=", "(", ")", "\n", "\r", " ", "\t", "é", "日",
    "\u{2028}", "\u{85}", "\u{FEFF}", "\u{7}", "0x", "1_", ".", "-", "+", "e", "/-", "/*", "*/",
    "//", "\"\"\"", "#\"", "\"#", "\\u{", "#true", "#nan", "\\\n", "😀",
];

fn mutate(rng: &mut Rng, s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    for _ in 0..1 + rng.below(4) {
        let pos = rng.below(chars.len() + 1);
        match rng.below(5) {
            0 => {
                let piece = PIECES[rng.below(PIECES.len())];
                for (i, c) in piece.chars().enumerate() {
                    chars.insert((pos + i).min(chars.len()), c);
                }
            }
            1 => {
                if pos < chars.len() {
                    chars.remove(pos);
                }
            }
            2 => chars.truncate(pos),
            3 => {
                if pos < chars.len() {
                    chars[pos] = PIECES[rng.below(PIECES.len())].chars().next().unwrap();
                }
            }
            _ => {
                let a = rng.below(chars.len() + 1);
                let b = (a + rng.below(16)).min(chars.len());
                let segment: Vec<char> = chars[a..b].to_vec();
                for (i, c) in segment.into_iter().enumerate() {
                    chars.insert((pos + i).min(chars.len()), c);
                }
            }
        }
    }
    chars.into_iter().collect()
}

fn spec_inputs() -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/kdl-spec/input");
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| fs::read_to_string(p).unwrap())
        .collect()
}

/// A derived type with numeric, optional, nested and map fields, so typed
/// deserialization of garbage exercises range checks and missing-field paths.
#[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
#[kdl(document)]
struct Loose {
    #[kdl(children, name = "node")]
    nodes: Vec<Item>,
    #[kdl(child_map, name = "env")]
    env: HashMap<String, String>,
}

#[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
#[kdl(name = "node")]
struct Item {
    #[kdl(argument)]
    label: Option<String>,
    #[kdl(property)]
    port: Option<u16>,
    #[kdl(property)]
    ratio: Option<f64>,
    #[kdl(property)]
    flag: Option<bool>,
    // `arguments` would also collect `label`, so the numbers live in a child.
    #[kdl(child, unwrap_args, name = "nums")]
    nums: Vec<i64>,
}

#[test]
fn mutated_input_never_panics_and_round_trips() {
    let seeds = spec_inputs();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut parsed = 0usize;
    for i in 0..iterations(20_000) {
        let seed = rng.below(seeds.len());
        let src = mutate(&mut rng, &seeds[seed]);
        // A panic here fails the test with the offending input in the message
        // of the assertion below only if parsing returns; print it first.
        let result = std::panic::catch_unwind(|| Document::parse(&src).map(|d| d.into_owned()));
        let doc = match result {
            Ok(Ok(doc)) => doc,
            Ok(Err(_)) => continue,
            Err(_) => panic!("parser panicked on case {i}: {src:?}"),
        };
        parsed += 1;
        let written = doc.to_string();
        let again = Document::parse(&written)
            .unwrap_or_else(|e| panic!("case {i}: written text fails to parse: {e}\n{written:?}"));
        // Debug formatting compares NaN as equal to NaN.
        assert_eq!(
            format!("{again:?}"),
            format!("{doc:?}"),
            "case {i}: changed by write → parse\nsource: {src:?}"
        );
    }
    // Make sure the mutator still produces a healthy share of valid inputs,
    // so the round-trip half of this test keeps meaning something.
    assert!(
        parsed > iterations(20_000) / 10,
        "only {parsed} mutated inputs parsed"
    );
}

#[test]
fn mutated_input_into_structs_never_panics() {
    let mut seeds = spec_inputs();
    // Seeds shaped like `Loose`, so mutations often reach the typed paths.
    seeds.push(
        "node \"a\" port=80 ratio=0.5 flag=#true { nums 1 2; }\nnode port=70000\nenv { K \"v\" }\n"
            .to_string(),
    );
    seeds.push("node #null port=-1 ratio=#nan { nums 0x7fffffffffffffffffff -1; }\n".to_string());
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for i in 0..iterations(20_000) {
        let seed = rng.below(seeds.len());
        let src = mutate(&mut rng, &seeds[seed]);
        let result = std::panic::catch_unwind(|| club_kdl::from_str::<Loose>(&src).is_ok());
        assert!(result.is_ok(), "from_str panicked on case {i}: {src:?}");
    }
}

/// Characters chosen to stress quoting: KDL syntax, escapes, newline
/// variants, disallowed code points, BOM, non-BMP.
const CHAR_POOL: &[char] = &[
    'a', 'z', 'A', '0', '9', '-', '+', '.', '_', ' ', '\t', '\n', '\r', '"', '\\', '/', '#', '=',
    '(', ')', '{', '}', '[', ']', ';', ',', '\u{0}', '\u{7}', '\u{8}', '\u{b}', '\u{c}', '\u{7f}',
    '\u{85}', '\u{a0}', '\u{200e}', '\u{2028}', '\u{2029}', '\u{feff}', 'é', '日', '😀',
];

/// Strings that look like other KDL tokens when written bare.
const TRICKY: &[&str] = &[
    "", "true", "false", "null", "inf", "-inf", "nan", "#true", "1", "-1", "+1", ".5", "-.5",
    "1e3", "0x10", "-", "+", ".", "-.", "a b", "\"\"\"", "/-", "//", "/*",
];

fn random_string(rng: &mut Rng) -> String {
    if rng.below(4) == 0 {
        return TRICKY[rng.below(TRICKY.len())].to_string();
    }
    (0..rng.below(12))
        .map(|_| CHAR_POOL[rng.below(CHAR_POOL.len())])
        .collect()
}

#[test]
fn arbitrary_strings_survive_write_and_read() {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for i in 0..iterations(5_000) {
        let value = Loose {
            nodes: (0..rng.below(3))
                .map(|_| Item {
                    label: (rng.below(3) != 0).then(|| random_string(&mut rng)),
                    port: None,
                    ratio: None,
                    flag: None,
                    nums: vec![],
                })
                .collect(),
            // Map keys become node names, which need quoting too.
            env: (0..rng.below(3))
                .map(|_| (random_string(&mut rng), random_string(&mut rng)))
                .collect(),
        };
        let text = club_kdl::to_string(&value).unwrap();
        let back: Loose = club_kdl::from_str(&text)
            .unwrap_or_else(|e| panic!("case {i}: {e}\nvalue: {value:?}\ntext: {text:?}"));
        assert_eq!(back, value, "case {i}: text: {text:?}");
    }
}
