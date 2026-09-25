//! Revision differential for CHG-013 stage A (plan revision 3, M12), registered before any
//! front-matter code exists. `docs/architecture/front-matter-contract.md` section 4: a page
//! without front matter produces exactly the document it produced before the feature.
//!
//! For every input it records a digest of the parsed document's pre-existing observation surface
//! (title, body, headings, links, diagnostics, section references, reference entries and
//! citations). The golden file holds the digests the parser produced at the contract commit.
//! After the feature:
//! - an input WITHOUT front matter (contract section 1, decided here by an independent predicate)
//!   must produce the same digest;
//! - an input WITH front matter must produce a different digest. This is the negative control:
//!   the comparison can fail, and the feature does change what it claims to change.
//!
//! Inputs never change over time: the frozen spec, every registered corpus page under
//! `xtask/tests/corpus/` (frozen by policy), and a corpus generated from a fixed seed. This file
//! lives in app-cli, not document, because it reads files (decision pure-fixture-inputs keeps
//! file I/O out of core-crate tests). `RHAWIKI_CAPTURE_FM_GOLDEN=1` rewrites the golden file; that
//! was done once, at the contract commit.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use library::{Digest, RelPath, Source};

const GOLDEN: &str = include_str!("data/front_matter_unchanged/golden.json");
const GENERATED_PAGES: usize = 3000;
const SEED: u64 = 0x5eed_c013;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Contract section 1, written independently of the implementation: the first line is exactly
/// `---` (trailing spaces or tabs allowed), and a later line is exactly `---` or `...` (same).
fn has_front_matter(text: &str) -> bool {
    let mut lines = text.split('\n');
    let delimiter = |line: &str, allow_dots: bool| {
        let t = line.trim_end_matches([' ', '\t']);
        t == "---" || (allow_dots && t == "...")
    };
    match lines.next() {
        Some(first) if delimiter(first, false) => lines.any(|line| delimiter(line, true)),
        _ => false,
    }
}

fn digest_of(key: &str, text: &str) -> String {
    let rel = RelPath::new(&format!("{key}.md")).expect("path");
    let d = document::parse(&Source::new(rel, text.to_owned()));
    let surface = format!(
        "{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}",
        d.title,
        d.body,
        d.headings,
        d.links,
        d.diagnostics,
        d.section_refs,
        d.reference_entries,
        d.citations
    );
    Digest::of(surface.as_bytes()).to_string()
}

/// The frozen inputs are exactly the non-generated keys of the golden file: the spec and every
/// registered corpus page that existed at registration. They are read by name, never discovered,
/// so a corpus registered later (for example this stage's own oracle package) cannot change the
/// input set. A missing file fails the test.
fn frozen_inputs(golden: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let root = repo_root();
    golden
        .keys()
        .filter(|key| !key.starts_with("generated/"))
        .map(|key| {
            let text = fs::read_to_string(root.join(key))
                .unwrap_or_else(|error| panic!("{key}: registered input missing: {error}"));
            (key.clone(), text)
        })
        .collect()
}

/// Discovery, used only when capturing the golden file.
fn discovered_inputs() -> BTreeMap<String, String> {
    let root = repo_root();
    let mut out = BTreeMap::new();
    let spec = "docs/spec/rha-spec-v0.10.md";
    out.insert(
        spec.to_owned(),
        fs::read_to_string(root.join(spec)).unwrap(),
    );
    let mut stack = vec![root.join("xtask/tests/corpus")];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                let rel = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if let Ok(text) = fs::read_to_string(&path) {
                    out.insert(rel, text);
                }
            }
        }
    }
    out
}

/// A small deterministic generator (a 64-bit linear congruential generator): no dependency, the
/// same pages on every run and every platform.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        let n = u64::try_from(items.len()).unwrap();
        items[usize::try_from(self.next() % n).unwrap()]
    }
}

const OPENINGS: &[&str] = &[
    "", "", "", "---\n", "---  \n", "---\t\n", "\n---\n", "+++\n", "--- \n", "----\n",
];
const META_LINES: &[&str] = &[
    "title: A title\n",
    "title: \"Quoted\"\n",
    "title: ''\n",
    "tags: [a, b, \"c d\"]\n",
    "tags:\n  - x\n  - y\n",
    "tags: solo\n",
    "# a comment\n",
    "\n",
    "date: 2026-09-25\n",
    "nested:\n  key: value\n",
    "- stray\n",
    "  - orphan\n",
    "tags: [A, a, , b]\n",
    "title: second\n",
];
const CLOSERS: &[&str] = &["---\n", "...\n", "--- \n", "", "----\n", "***\n"];
const BLOCKS: &[&str] = &[
    "# Heading\n",
    "## Sub heading\n",
    "Plain paragraph text.\n",
    "Setext title\n---\n",
    "Text before\n\n---\n\nText after a rule.\n",
    "---\ntitle: mid page\n---\n",
    "- item one\n- item two\n",
    "> quote\n",
    "> [!NOTE]\n> callout body\n",
    "| a | b |\n|---|---|\n| 1 | 2 |\n",
    "```\n---\ncode\n```\n",
    "See §1 and [R1] and [[Page]].\n",
    "![[Page#Section]]\n",
    "[R1] Author. Title.\n",
    "***\n",
    "...\n",
    "wow![x] and `code`\n",
    "Line with trailing  \nhard break\n",
];

fn generated_inputs() -> BTreeMap<String, String> {
    let mut rng = Lcg(SEED);
    let mut out = BTreeMap::new();
    for i in 0..GENERATED_PAGES {
        let mut page = String::new();
        let opening = rng.pick(OPENINGS);
        page.push_str(opening);
        if !opening.is_empty() {
            for _ in 0..(rng.next() % 5) {
                page.push_str(rng.pick(META_LINES));
            }
            page.push_str(rng.pick(CLOSERS));
        }
        for _ in 0..(1 + rng.next() % 6) {
            page.push('\n');
            page.push_str(rng.pick(BLOCKS));
        }
        out.insert(format!("generated/{i:04}"), page);
    }
    out
}

#[test]
fn documents_without_front_matter_are_unchanged_and_documents_with_it_change() {
    if std::env::var_os("RHAWIKI_CAPTURE_FM_GOLDEN").is_some() {
        let mut inputs = discovered_inputs();
        inputs.extend(generated_inputs());
        let observed: BTreeMap<String, String> = inputs
            .iter()
            .map(|(key, text)| (key.clone(), digest_of(key, text)))
            .collect();
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/front_matter_unchanged/golden.json");
        fs::write(
            path,
            serde_json::to_string_pretty(&observed).unwrap() + "\n",
        )
        .unwrap();
        return;
    }
    let golden: BTreeMap<String, String> = serde_json::from_str(GOLDEN).unwrap();
    let mut inputs = frozen_inputs(&golden);
    inputs.extend(generated_inputs());
    let observed: BTreeMap<String, String> = inputs
        .iter()
        .map(|(key, text)| (key.clone(), digest_of(key, text)))
        .collect();
    assert_eq!(
        golden.keys().collect::<Vec<_>>(),
        observed.keys().collect::<Vec<_>>(),
        "the same inputs as at registration"
    );
    let (mut unchanged, mut changed) = (0usize, 0usize);
    for (key, text) in &inputs {
        if has_front_matter(text) {
            assert_ne!(
                observed[key], golden[key],
                "{key}: has front matter, so its document must change"
            );
            changed += 1;
        } else {
            assert_eq!(
                observed[key], golden[key],
                "{key}: no front matter, so its document must not change"
            );
            unchanged += 1;
        }
    }
    assert!(
        changed >= 300,
        "the generator must exercise front matter: {changed}"
    );
    assert!(unchanged >= 1500, "and pages without it: {unchanged}");
}

#[test]
fn the_front_matter_predicate_matches_contract_section_1() {
    assert!(has_front_matter("---\ntitle: x\n---\nbody\n"));
    assert!(has_front_matter("---  \n...\n"));
    assert!(has_front_matter("---\n---\n"));
    assert!(!has_front_matter("\n---\ntitle: x\n---\n"));
    assert!(!has_front_matter("---\ntitle: x\n"));
    assert!(!has_front_matter("Intro\n\n---\ntitle: x\n---\n"));
    assert!(!has_front_matter("----\ntitle: x\n---\n"));
    assert!(!has_front_matter("+++\ntitle: x\n+++\n"));
}
