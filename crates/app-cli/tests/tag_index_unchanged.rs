//! Revision differential for CHG-013 stage B, the tag index (plan revision 3, M12). It was
//! registered before any stage B code existed. `docs/architecture/tag-index-contract.md`
//! section 4: apart from tag index pages and the search index of their site, every rendered
//! output file is byte-identical once the build timestamp is removed.
//!
//! Inputs:
//! - The 60 registered markdown sites under `xtask/tests/corpus/markdown/sites/`, read by name
//!   from this file's golden manifest and never discovered at run time (stage A's lesson).
//! - 150 sites generated from a fixed seed. The generator decides which pages are tag indexes
//!   (front matter `index: tags`, quoted or not) and includes decoys that must not count:
//!   `index: other`, `index: Tags`, an empty `index:`, and tags with no index page.
//!
//! For every site, each HTML and JSON output file's digest is compared with the digests captured
//! with the pre-feature binary at registration:
//! - When a site has at least one tag and at least one tag index page, the index pages' files
//!   and that site's search index must CHANGE. This is the negative control.
//! - Every other file must be unchanged.
//!
//! `RHAWIKI_CAPTURE_TAG_GOLDEN=1` rewrites the golden file. That was done once, at the
//! registration commit.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

const GOLDEN: &str = include_str!("data/tag_index_unchanged/golden.json");
const GENERATED_SITES: usize = 150;
const SEED: u64 = 0x7a61_b013;

/// The golden file: `registered_inputs` maps a registered site name to its input page paths (read
/// by name); `outputs` maps "site/format/output path" to the digest of the timestamp-normalised
/// file.
struct Golden {
    registered_inputs: BTreeMap<String, Vec<String>>,
    outputs: BTreeMap<String, String>,
}

impl Golden {
    fn parse(text: &str) -> Self {
        let v: Value = serde_json::from_str(text).unwrap();
        let registered_inputs = v["registered_inputs"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, pages)| {
                let pages = pages
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p.as_str().unwrap().to_owned())
                    .collect();
                (k.clone(), pages)
            })
            .collect();
        let outputs = v["outputs"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, d)| (k.clone(), d.as_str().unwrap().to_owned()))
            .collect();
        Self {
            registered_inputs,
            outputs,
        }
    }

    fn to_json(&self) -> String {
        serde_json::to_string_pretty(&json!({
            "registered_inputs": self.registered_inputs,
            "outputs": self.outputs,
        }))
        .unwrap()
            + "\n"
    }
}

struct Site {
    name: String,
    pages: BTreeMap<String, String>,
    /// Page ids (paths without `.md`) that are tag indexes under the contract.
    index_pages: BTreeSet<String>,
    has_tags: bool,
}

struct OwnedTempDir(PathBuf);

impl Drop for OwnedTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp(tag: &str) -> OwnedTempDir {
    let path = std::env::temp_dir().join(format!("rha-tag-unchanged-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    OwnedTempDir(path)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn registered_dir() -> PathBuf {
    repo_root().join("xtask/tests/corpus/markdown/sites")
}

/// Discovery, used only when capturing the golden file.
fn discover_registered() -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(registered_dir()).unwrap() {
        let dir = entry.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        let mut pages = Vec::new();
        let mut stack = vec![dir.clone()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "md") {
                    pages.push(
                        p.strip_prefix(&dir)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/"),
                    );
                }
            }
        }
        pages.sort();
        out.insert(
            dir.file_name().unwrap().to_string_lossy().into_owned(),
            pages,
        );
    }
    out
}

fn registered_sites(inputs: &BTreeMap<String, Vec<String>>) -> Vec<Site> {
    inputs
        .iter()
        .map(|(name, pages)| Site {
            name: format!("registered/{name}"),
            pages: pages
                .iter()
                .map(|p| {
                    let text = fs::read_to_string(registered_dir().join(name).join(p))
                        .unwrap_or_else(|e| panic!("{name}/{p}: registered input missing: {e}"));
                    (p.clone(), text)
                })
                .collect(),
            index_pages: BTreeSet::new(),
            has_tags: false,
        })
        .collect()
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).unwrap()).unwrap()
    }
}

const TAG_WORDS: &[&str] = &[
    "alpha", "Beta", "beta", "gamma", "C++", "C", "Ünicode", "!!!", "Tags", "notes", "Rust",
];
/// (front-matter line, makes the page a tag index)
const INDEX_LINES: &[(&str, bool)] = &[
    ("index: tags\n", true),
    ("index: \"tags\"\n", true),
    ("index: 'tags'\n", true),
    ("index: other\n", false),
    ("index: Tags\n", false),
    ("index:\n", false),
];
const BODIES: &[&str] = &[
    "# Heading\n\nText.\n",
    "## Tags\n\nA heading whose slug collides.\n",
    "## tag-alpha\n\nAnother collision.\n",
    "Plain text with [[index]] link.\n",
    "- a list\n- of items\n",
    "> [!NOTE]\n> callout\n",
    "See [R1].\n\n[R1] Entry.\n",
    "",
];

fn generated_sites() -> Vec<Site> {
    let mut rng = Lcg(SEED);
    let mut sites = Vec::new();
    for s in 0..GENERATED_SITES {
        let mut pages = BTreeMap::new();
        let mut index_pages = BTreeSet::new();
        let mut has_tags = false;
        let n_pages = 2 + rng.below(6);
        for p in 0..n_pages {
            let path = if rng.below(4) == 0 {
                format!("sub/p{p}.md")
            } else {
                format!("p{p}.md")
            };
            let mut text = String::new();
            let use_front_matter = rng.below(3) != 0;
            if use_front_matter {
                text.push_str("---\n");
                if rng.below(2) == 0 {
                    text.push_str(&format!("title: Page {p}\n"));
                }
                let n_tags = rng.below(4);
                if n_tags > 0 {
                    let tags: Vec<&str> = (0..n_tags)
                        .map(|_| TAG_WORDS[rng.below(TAG_WORDS.len())])
                        .collect();
                    text.push_str(&format!("tags: [{}]\n", tags.join(", ")));
                    has_tags = true;
                }
                if rng.below(3) == 0 {
                    let (line, is_index) = INDEX_LINES[rng.below(INDEX_LINES.len())];
                    text.push_str(line);
                    if is_index {
                        index_pages.insert(path.trim_end_matches(".md").to_owned());
                    }
                }
                text.push_str("---\n");
            }
            text.push_str(BODIES[rng.below(BODIES.len())]);
            pages.insert(path, text);
        }
        sites.push(Site {
            name: format!("generated/{s:03}"),
            pages,
            index_pages,
            has_tags,
        });
    }
    sites
}

fn normalise(text: &str) -> String {
    let mut out = String::new();
    for line in text.split_inclusive('\n') {
        if line.starts_with("<footer>Built at ") {
            out.push_str("<footer>Built at *</footer>\n");
        } else {
            out.push_str(line);
        }
    }
    while let Some(start) = out.find("\"built_at\":") {
        let value_start = start + "\"built_at\":".len();
        let rest = &out[value_start..];
        if rest.starts_with("\"*\"") {
            break;
        }
        let value_len = rest.find([',', '}']).unwrap_or(rest.len());
        out.replace_range(value_start..value_start + value_len, "\"*\"");
    }
    out
}

fn files(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(root, &path, out);
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, fs::read_to_string(&path).unwrap());
        }
    }
}

fn build_site(site: &Site) -> BTreeMap<String, String> {
    let src = temp("src");
    for (path, text) in &site.pages {
        let p = src.0.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    let mut out = BTreeMap::new();
    for format in ["html", "json"] {
        let dst = temp(format);
        let status = Command::new(env!("CARGO_BIN_EXE_rhawiki"))
            .args(["build", "--root"])
            .arg(&src.0)
            .args(["--out"])
            .arg(&dst.0)
            .args(["--format", format])
            .output()
            .expect("rhawiki build must start");
        assert!(
            status.status.success(),
            "{} {format}: {}",
            site.name,
            String::from_utf8_lossy(&status.stderr)
        );
        let mut produced = BTreeMap::new();
        files(&dst.0, &dst.0, &mut produced);
        for (path, text) in produced {
            out.insert(
                format!("{}/{format}/{path}", site.name),
                library::Digest::of(normalise(&text).as_bytes()).to_string(),
            );
        }
    }
    out
}

/// Whether an output key belongs to a tag index page or to the search index of its site.
fn may_change(site: &Site, key: &str) -> bool {
    if !site.has_tags || site.index_pages.is_empty() {
        return false;
    }
    let rest = key.strip_prefix(&format!("{}/", site.name)).unwrap();
    let (_format, path) = rest.split_once('/').unwrap();
    if path == "assets/search-index.json" {
        return true;
    }
    let page = path.trim_end_matches(".html").trim_end_matches(".json");
    site.index_pages.contains(page)
}

#[test]
fn only_tag_index_pages_and_their_search_index_change() {
    if std::env::var_os("RHAWIKI_CAPTURE_TAG_GOLDEN").is_some() {
        let registered_inputs = discover_registered();
        let mut sites = registered_sites(&registered_inputs);
        sites.extend(generated_sites());
        let mut outputs = BTreeMap::new();
        for site in &sites {
            outputs.extend(build_site(site));
        }
        let golden = Golden {
            registered_inputs,
            outputs,
        };
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/tag_index_unchanged/golden.json");
        fs::write(path, golden.to_json()).unwrap();
        return;
    }
    let golden = Golden::parse(GOLDEN);
    let mut sites = registered_sites(&golden.registered_inputs);
    sites.extend(generated_sites());
    let (mut changed, mut unchanged) = (0usize, 0usize);
    let mut observed_keys = BTreeSet::new();
    for site in &sites {
        for (key, digest) in build_site(site) {
            let old = golden
                .outputs
                .get(&key)
                .unwrap_or_else(|| panic!("{key}: not a registered output"));
            if may_change(site, &key) {
                assert_ne!(&digest, old, "{key}: a tag index output must change");
                changed += 1;
            } else {
                assert_eq!(&digest, old, "{key}: must not change");
                unchanged += 1;
            }
            observed_keys.insert(key);
        }
    }
    let registered_keys: BTreeSet<String> = golden.outputs.keys().cloned().collect();
    assert_eq!(
        observed_keys, registered_keys,
        "the same outputs as at registration"
    );
    assert!(
        changed >= 60,
        "the generator must exercise tag indexes: {changed}"
    );
    assert!(unchanged >= 1000, "and everything else: {unchanged}");
}
