//! `cargo xtask rha schemas [--check]`: translate the registered record-shape
//! inventory into JSON Schema 2020-12 files at `.rha/schemas/`.
//!
//! The inventory is `xtask/tests/corpus/record-schema/INVENTORY.json`. The
//! registered markdown-schema supplement
//! (`xtask/tests/corpus/markdown-schema-supplement`) adds two optional
//! properties to the `markdown_corpus` family after its manifest, pinned
//! content digest and cited sources are verified. The committed schema files
//! are the byte-exact oracle: `--check` writes nothing and exits 1 if any
//! generated file is missing or differs. An inventory or supplement that is
//! not translatable is refused with exit 2.
//!
//! This is a port of the former `xtask/schema_codegen.py` (CHG-007.3); the
//! canonical form is that script's `json.dumps(indent=2, sort_keys=True,
//! ensure_ascii=False)` followed by one LF.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::util::sha256_hex;

const INVENTORY_PATH: &str = "xtask/tests/corpus/record-schema/INVENTORY.json";
const OUTPUT_DIR: &str = ".rha/schemas";
const CONTRACT_PATH: &str = "docs/architecture/record-schema-contract.md";
const SCHEMA_URI: &str = "https://json-schema.org/draft/2020-12/schema";
const INVENTORY_FORMAT: &str = "rha-observed-shapes-1";
const SUPPLEMENT_DIR: &str = "xtask/tests/corpus/markdown-schema-supplement";
const SUPPLEMENT_CONTENT_SHA256: &str =
    "e271498e5619751e3fd8d50eeee26728a7154d3b9cb451f9c75cffa37913b183";
const SUPPLEMENT_AMENDMENT_PATH: &str = "docs/architecture/markdown-record-shape-amendment.md";
/// The registration preserves the original generation location of the
/// supplement's prompt; decision `registered-prompt-archive-resolution` maps
/// exactly this path to its committed, byte-identical archive.
const REGISTERED_GENERATED_SOURCE_PATH: &str = "target/m2/markdown-schema-supplement-prompt.md";
const ARCHIVED_GENERATED_SOURCE_PATH: &str =
    "xtask/tests/corpus/write-prompts/CHG-007/markdown-schema-supplement-prompt.md";

/// The record families, in the (sorted) order they are generated.
pub const FAMILIES: [&str; 7] = [
    "acceptance",
    "evidence",
    "h4",
    "h5_conformance",
    "markdown_corpus",
    "policy",
    "task",
];

const JSON_TYPES: [&str; 7] = [
    "null", "boolean", "integer", "number", "string", "array", "object",
];

const OBJECT_MODES: [&str; 3] = ["closed_record", "open_map", "observation_only"];

type Pointer = Vec<String>;
type Object = Map<String, Value>;
type NodeMap = BTreeMap<Pointer, Object>;

fn fail<T>(message: impl Into<String>) -> Result<T> {
    Err(Error::new(message))
}

/// Runs the generator for the repository at `root`.
///
/// Without `check` it writes every schema and returns 0. With `check` it
/// writes nothing, prints each missing or differing file to stderr, and
/// returns 1 if there is any.
///
/// # Errors
/// Fails if the inventory or supplement is not translatable, or a schema
/// cannot be written.
pub fn run(root: &Path, check: bool) -> Result<u8> {
    let schemas = generate(root)?;
    if check {
        let failures = compare(root, &schemas);
        for failure in &failures {
            eprintln!("rha schemas: {failure}");
        }
        return Ok(u8::from(!failures.is_empty()));
    }
    write(root, &schemas)?;
    Ok(0)
}

/// Compares the generated schemas with the files under `root`; returns one
/// line per missing, unreadable or differing file, in family order.
///
/// # Errors
/// Fails if the inventory or supplement is not translatable.
pub fn check(root: &Path) -> Result<Vec<String>> {
    let schemas = generate(root)?;
    Ok(compare(root, &schemas))
}

/// The path of a family's generated schema under `root`.
#[must_use]
pub fn output_path(root: &Path, family: &str) -> PathBuf {
    root.join(OUTPUT_DIR).join(format!("{family}.schema.json"))
}

/// Generates the canonical bytes of every family's schema.
///
/// # Errors
/// Fails if the inventory or supplement is not translatable.
pub fn generate(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let inventory_path = root.join(INVENTORY_PATH);
    let inventory_bytes = read_bytes(&inventory_path)?;
    let inventory: Value = match std::str::from_utf8(&inventory_bytes)
        .map_err(|error| error.to_string())
        .and_then(|text| serde_json::from_str(text).map_err(|error| error.to_string()))
    {
        Ok(value) => value,
        Err(error) => {
            return fail(format!(
                "cannot parse {}: {error}",
                inventory_path.display()
            ));
        }
    };
    validate_inventory(&inventory)?;
    let mut documents = build_schemas(&inventory, &inventory_bytes)?;
    let supplement = Supplement::new(root);
    let (registration, additions) = supplement.load()?;
    apply_supplement(&mut documents, &registration, &additions)?;
    Ok(documents
        .into_iter()
        .map(|(family, document)| (family, canonical_json(&document)))
        .collect())
}

fn compare(root: &Path, schemas: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut failures = Vec::new();
    for (family, expected) in schemas {
        let path = output_path(root, family);
        if !path.is_file() {
            failures.push(format!("missing {}", path.display()));
            continue;
        }
        match std::fs::read(&path) {
            Err(error) => failures.push(format!("cannot read {}: {error}", path.display())),
            Ok(actual) if actual != *expected => {
                failures.push(format!("different {}", path.display()));
            }
            Ok(_) => {}
        }
    }
    failures
}

fn write(root: &Path, schemas: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    let written = std::fs::create_dir_all(root.join(OUTPUT_DIR)).and_then(|()| {
        schemas
            .iter()
            .try_for_each(|(family, bytes)| std::fs::write(output_path(root, family), bytes))
    });
    written.map_err(|error| Error::new(format!("cannot write schemas: {error}")))
}

fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|error| Error::new(format!("reading {}: {error}", path.display())))
}

// ---------------------------------------------------------------------------
// JSON Pointer, types and canonical form

fn parse_pointer(value: &Value, context: &str) -> Result<Pointer> {
    let Value::String(value) = value else {
        return fail(format!("{context}: JSON Pointer must be a string"));
    };
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let Some(rest) = value.strip_prefix('/') else {
        return fail(format!(
            "{context}: invalid JSON Pointer {}",
            py_repr_str(value)
        ));
    };
    let mut parts = Vec::new();
    for raw in rest.split('/') {
        let mut decoded = String::new();
        let mut characters = raw.chars();
        while let Some(character) = characters.next() {
            if character != '~' {
                decoded.push(character);
                continue;
            }
            match characters.next() {
                Some('0') => decoded.push('~'),
                Some('1') => decoded.push('/'),
                _ => {
                    return fail(format!(
                        "{context}: invalid JSON Pointer escape in {}",
                        py_repr_str(value)
                    ));
                }
            }
        }
        parts.push(decoded);
    }
    Ok(parts)
}

fn pointer(parts: &[String]) -> String {
    parts.iter().fold(String::new(), |mut out, part| {
        let _ = write!(out, "/{}", part.replace('~', "~0").replace('/', "~1"));
        out
    })
}

fn schema_type(types: &[String]) -> Value {
    match types {
        [single] => Value::String(single.clone()),
        _ => Value::Array(types.iter().cloned().map(Value::String).collect()),
    }
}

/// A non-empty list of unique JSON type names, or `None`.
fn json_type_list(value: Option<&Value>) -> Option<Vec<String>> {
    let items = value?.as_array()?;
    if items.is_empty() {
        return None;
    }
    let mut types = Vec::with_capacity(items.len());
    for item in items {
        let name = item.as_str()?;
        if !JSON_TYPES.contains(&name) || types.iter().any(|seen| seen == name) {
            return None;
        }
        types.push(name.to_owned());
    }
    Some(types)
}

fn string_list(value: Option<&Value>) -> Option<Vec<String>> {
    value?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

/// `json.dumps(value, ensure_ascii=False, allow_nan=False, indent=2,
/// sort_keys=True) + "\n"`, encoded as UTF-8.
#[must_use]
pub fn canonical_json(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    write_canonical(&mut out, value, 0);
    out.push('\n');
    out.into_bytes()
}

fn write_canonical(out: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(number) => out.push_str(&number_repr(number)),
        Value::String(text) => write_json_string(out, text),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                newline_indent(out, indent + 2);
                write_canonical(out, item, indent + 2);
            }
            newline_indent(out, indent);
            out.push(']');
        }
        Value::Object(object) => {
            if object.is_empty() {
                out.push_str("{}");
                return;
            }
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                newline_indent(out, indent + 2);
                write_json_string(out, key);
                out.push_str(": ");
                write_canonical(out, &object[key], indent + 2);
            }
            newline_indent(out, indent);
            out.push('}');
        }
    }
}

fn newline_indent(out: &mut String, indent: usize) {
    out.push('\n');
    out.extend(std::iter::repeat_n(' ', indent));
}

/// Python's `json` string encoding with `ensure_ascii=False`.
fn write_json_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            control if u32::from(control) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(control));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

fn number_repr(number: &serde_json::Number) -> String {
    if let Some(integer) = number.as_i64() {
        integer.to_string()
    } else if let Some(integer) = number.as_u64() {
        integer.to_string()
    } else {
        number
            .as_f64()
            .map_or_else(|| number.to_string(), py_float_repr)
    }
}

/// Python's `repr(float)`: the shortest round-trip digits, positional when
/// the decimal exponent is in -4..16, otherwise `d.ddde+XX`.
fn py_float_repr(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .to_owned();
    }
    if !value.is_finite() {
        // Python's json module refuses these with allow_nan=False; serde_json
        // cannot hold them.
        return value.to_string();
    }
    let scientific = format!("{value:e}");
    let (sign, unsigned) = match scientific.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", scientific.as_str()),
    };
    let (mantissa, exponent) = unsigned.split_once('e').unwrap_or((unsigned, "0"));
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let length = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    let point = exponent + 1;
    let body = if -4 < point && point <= 16 {
        if point <= 0 {
            format!(
                "0.{}{digits}",
                "0".repeat(usize::try_from(-point).unwrap_or(0))
            )
        } else if point >= length {
            format!(
                "{digits}{}.0",
                "0".repeat(usize::try_from(point - length).unwrap_or(0))
            )
        } else {
            let split = usize::try_from(point).unwrap_or(0);
            format!("{}.{}", &digits[..split], &digits[split..])
        }
    } else {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let exponent_sign = if exponent < 0 { '-' } else { '+' };
        format!("{first}{fraction}e{exponent_sign}{:02}", exponent.abs())
    };
    format!("{sign}{body}")
}

/// Python's `repr()` of a JSON value, for messages.
fn py_repr(value: &Value) -> String {
    match value {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Number(number) => number_repr(number),
        Value::String(text) => py_repr_str(text),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(py_repr).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(object) => format!(
            "{{{}}}",
            object
                .iter()
                .map(|(key, value)| format!("{}: {}", py_repr_str(key), py_repr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Python's `repr()` of a string, for messages.
fn py_repr_str(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for character in text.chars() {
        let code = u32::from(character);
        match character {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other if other == quote => {
                out.push('\\');
                out.push(other);
            }
            _ if code < 0x20 || code == 0x7f => {
                let _ = write!(out, "\\x{code:02x}");
            }
            other if code < 0x7f || py_printable(other) => out.push(other),
            _ if code <= 0xff => {
                let _ = write!(out, "\\x{code:02x}");
            }
            _ if code <= 0xffff => {
                let _ = write!(out, "\\u{code:04x}");
            }
            _ => {
                let _ = write!(out, "\\U{code:08x}");
            }
        }
    }
    out.push(quote);
    out
}

/// An approximation of Python's `str.isprintable` for non-ASCII characters:
/// control, format, separator and private-use characters are escaped.
fn py_printable(character: char) -> bool {
    let code = u32::from(character);
    !(character.is_control()
        || matches!(
            code,
            0xa0 | 0xad
                | 0x061c
                | 0x1680
                | 0x180e
                | 0x2000..=0x200f
                | 0x2028..=0x202f
                | 0x205f..=0x206f
                | 0x3000
                | 0xd800..=0xf8ff
                | 0xfeff
                | 0xfff9..=0xfffb
                | 0xf0000..
        ))
}

fn safe_relative_path(value: Option<&str>, repr: &str, context: &str) -> Result<String> {
    match value {
        Some(text)
            if !text.is_empty()
                && !text.contains('\\')
                && !text.starts_with('/')
                && text
                    .split('/')
                    .all(|part| !part.is_empty() && part != "." && part != "..") =>
        {
            Ok(text.to_owned())
        }
        _ => fail(format!("{context}: unsafe relative path {repr}")),
    }
}

fn sha256_digest(value: Option<&str>, context: &str) -> Result<String> {
    match value {
        Some(text)
            if text.len() == 64
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) =>
        {
            Ok(text.to_owned())
        }
        _ => fail(format!("{context}: expected a lowercase SHA-256 digest")),
    }
}

fn toml_repr(value: Option<&toml::Value>) -> String {
    match value {
        None => "None".to_owned(),
        Some(toml::Value::String(text)) => py_repr_str(text),
        Some(other) => {
            serde_json::to_value(other).map_or_else(|_| other.to_string(), |v| py_repr(&v))
        }
    }
}

// ---------------------------------------------------------------------------
// The registered markdown-schema supplement

struct Supplement {
    root: PathBuf,
    dir: PathBuf,
}

impl Supplement {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            dir: root.join(SUPPLEMENT_DIR),
        }
    }

    fn registration_path(&self) -> PathBuf {
        self.dir.join("registration.toml")
    }

    fn sums_path(&self) -> PathBuf {
        self.dir.join("SHA256SUMS")
    }

    fn additions_path(&self) -> PathBuf {
        self.dir.join("ADDITIONS.json")
    }

    fn registered_source_path(&self, relative: &str) -> PathBuf {
        // The registration preserves the original generation location; this
        // durable archive is the committed source used by fresh checkouts.
        if relative == REGISTERED_GENERATED_SOURCE_PATH {
            self.root.join(ARCHIVED_GENERATED_SOURCE_PATH)
        } else {
            self.root.join(relative)
        }
    }

    fn payload_paths(&self) -> Result<BTreeSet<String>> {
        let durable = std::fs::symlink_metadata(&self.dir)
            .is_ok_and(|metadata| metadata.file_type().is_dir());
        if !durable {
            return fail(format!(
                "markdown schema supplement is not a durable directory: {}",
                self.dir.display()
            ));
        }
        let mut payloads = BTreeSet::new();
        self.walk(&self.dir, &mut payloads)?;
        Ok(payloads)
    }

    /// `os.walk(topdown=True, followlinks=False)`: every entry of a directory
    /// is checked, directories first, before any subdirectory is entered.
    fn walk(&self, directory: &Path, payloads: &mut BTreeSet<String>) -> Result<()> {
        let walk_error = |error: std::io::Error| {
            Error::new(format!("walking markdown schema supplement: {error}"))
        };
        let mut directories = Vec::new();
        let mut files = Vec::new();
        for entry in std::fs::read_dir(directory).map_err(walk_error)? {
            let entry = entry.map_err(walk_error)?;
            let path = entry.path();
            // os.walk classifies with a symlink-following stat.
            if path.is_dir() {
                directories.push(path);
            } else {
                files.push(path);
            }
        }
        directories.sort();
        files.sort();
        for path in directories.iter().chain(&files) {
            if path.is_symlink() {
                return fail(format!(
                    "symlink in markdown schema supplement: {}",
                    path.display()
                ));
            }
            if !path.is_dir() && !path.is_file() {
                return fail(format!("non-file supplement entry: {}", path.display()));
            }
        }
        for path in &files {
            let relative = path
                .strip_prefix(&self.dir)
                .map(|relative| {
                    relative
                        .components()
                        .map(|component| component.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .unwrap_or_default();
            if relative != "registration.toml" && relative != "SHA256SUMS" {
                payloads.insert(relative);
            }
        }
        for path in &directories {
            self.walk(path, payloads)?;
        }
        Ok(())
    }

    fn load_registration(&self) -> Result<toml::Table> {
        let path = self.registration_path();
        let bytes = read_bytes(&path)?;
        let registration: toml::Table = match std::str::from_utf8(&bytes)
            .map_err(|error| error.to_string())
            .and_then(|text| toml::from_str(text).map_err(|error| error.to_string()))
        {
            Ok(table) => table,
            Err(error) => return fail(format!("cannot parse {}: {error}", path.display())),
        };

        let text = |key: &str| registration.get(key).and_then(toml::Value::as_str);
        if text("content_sha256") != Some(SUPPLEMENT_CONTENT_SHA256) {
            return fail("markdown schema supplement registration content_sha256 is not pinned");
        }
        if text("family") != Some("markdown_corpus") {
            return fail("markdown schema supplement registration has an unknown family");
        }
        if registration
            .get("after_data")
            .and_then(toml::Value::as_bool)
            != Some(true)
        {
            return fail("markdown schema supplement registration must declare after_data = true");
        }
        if text("grading_authority_ledger_id").is_none() {
            return fail("markdown schema supplement registration has no ledger id");
        }
        Ok(registration)
    }

    fn verify_manifest(&self, registration: &toml::Table) -> Result<()> {
        let sums_bytes = read_bytes(&self.sums_path())?;
        if sha256_hex(&sums_bytes) != SUPPLEMENT_CONTENT_SHA256 {
            return fail("markdown schema supplement SHA256SUMS content digest is not pinned");
        }
        if !sums_bytes.ends_with(b"\n") {
            return fail("markdown schema supplement SHA256SUMS must end with LF");
        }
        let sums_text = match std::str::from_utf8(&sums_bytes) {
            Ok(text) => text,
            Err(error) => {
                return fail(format!(
                    "markdown schema supplement SHA256SUMS is not UTF-8: {error}"
                ));
            }
        };

        let mut listed = BTreeSet::new();
        let mut previous: Option<String> = None;
        for line in sums_text[..sums_text.len() - 1].split('\n') {
            let characters: Vec<char> = line.chars().collect();
            if line.is_empty()
                || line.contains('\r')
                || characters.len() < 66
                || characters[64..66] != [' ', ' ']
            {
                return fail(format!(
                    "malformed markdown schema supplement SHA256SUMS line: {}",
                    py_repr_str(line)
                ));
            }
            let digest_text: String = characters[..64].iter().collect();
            let path_text: String = characters[66..].iter().collect();
            let digest = sha256_digest(Some(&digest_text), "SHA256SUMS digest")?;
            let relative = safe_relative_path(
                Some(&path_text),
                &py_repr_str(&path_text),
                "SHA256SUMS path",
            )?;
            if previous
                .as_ref()
                .is_some_and(|previous| *previous >= relative)
            {
                return fail(format!(
                    "markdown schema supplement SHA256SUMS is not strictly sorted at {relative}"
                ));
            }
            previous = Some(relative.clone());
            if listed.contains(&relative) {
                return fail(format!(
                    "duplicate markdown schema supplement payload: {relative}"
                ));
            }

            let path = self.dir.join(&relative);
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    return fail(format!(
                        "reading listed supplement payload {relative}: {error}"
                    ));
                }
            };
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return fail(format!(
                    "listed supplement payload is not a durable file: {relative}"
                ));
            }
            if sha256_hex(&read_bytes(&path)?) != digest {
                return fail(format!(
                    "markdown schema supplement payload digest mismatch: {relative}"
                ));
            }
            listed.insert(relative);
        }

        let actual = self.payload_paths()?;
        if listed != actual {
            return fail(format!(
                "markdown schema supplement inventory differs from durable payload set: \
                 listed {}, actual {}",
                listed.len(),
                actual.len()
            ));
        }
        if registration
            .get("content_sha256")
            .and_then(toml::Value::as_str)
            != Some(SUPPLEMENT_CONTENT_SHA256)
        {
            return fail("markdown schema supplement registration does not match SHA256SUMS");
        }
        Ok(())
    }

    fn load(&self) -> Result<(toml::Table, Object)> {
        let registration = self.load_registration()?;
        self.verify_manifest(&registration)?;

        let path = self.additions_path();
        let bytes = read_bytes(&path)?;
        let additions: Value = match std::str::from_utf8(&bytes)
            .map_err(|error| error.to_string())
            .and_then(|text| serde_json::from_str(text).map_err(|error| error.to_string()))
        {
            Ok(value) => value,
            Err(error) => return fail(format!("cannot parse {}: {error}", path.display())),
        };

        let expected_keys = ["after_data", "family", "optional", "source_paths"];
        let Some(additions) = additions.as_object().filter(|object| {
            object.len() == expected_keys.len()
                && expected_keys.iter().all(|key| object.contains_key(*key))
        }) else {
            return fail("markdown schema supplement ADDITIONS.json has malformed top-level keys");
        };
        if additions["family"] != "markdown_corpus" {
            return fail("markdown schema supplement ADDITIONS.json has an unknown family");
        }
        if additions["after_data"] != Value::Bool(true) {
            return fail("markdown schema supplement ADDITIONS.json must declare after_data");
        }

        let optional = match additions["optional"].as_array() {
            Some(optional) if optional.len() == 2 => optional,
            _ => {
                return fail(
                    "markdown schema supplement must contain exactly two optional properties",
                );
            }
        };
        let mut seen_paths = BTreeSet::new();
        for (index, addition) in optional.iter().enumerate() {
            let context = format!("ADDITIONS.json optional[{index}]");
            let Some(addition) = addition.as_object().filter(|object| {
                object.len() == 2
                    && object.contains_key("instance_path")
                    && object.contains_key("types")
            }) else {
                return fail(format!("{context}: malformed optional property"));
            };
            let path = parse_pointer(
                &addition["instance_path"],
                &format!("{context}.instance_path"),
            )?;
            if path.is_empty() || !seen_paths.insert(path) {
                return fail(format!("{context}: duplicate or root instance path"));
            }
            if json_type_list(addition.get("types")).is_none() {
                return fail(format!("{context}.types: malformed JSON type list"));
            }
        }

        let sources = &additions["source_paths"];
        let registered_sources = registration.get("sources");
        let (Some(_), Some(registered_sources)) = (
            sources.as_array(),
            registered_sources.and_then(toml::Value::as_array),
        ) else {
            return fail("markdown schema supplement source inventory is malformed");
        };
        let mut normalized_sources = Vec::new();
        for (index, source) in registered_sources.iter().enumerate() {
            let context = format!("registration.sources[{index}]");
            let Some(source) = source.as_table().filter(|table| {
                table.len() == 2 && table.contains_key("path") && table.contains_key("sha256")
            }) else {
                return fail(format!("{context}: malformed source entry"));
            };
            let relative = safe_relative_path(
                source["path"].as_str(),
                &toml_repr(source.get("path")),
                &format!("{context}.path"),
            )?;
            let digest = sha256_digest(source["sha256"].as_str(), &format!("{context}.sha256"))?;
            let path = self.registered_source_path(&relative);
            if path.is_symlink() || !path.is_file() {
                return fail(format!("{context}: source is not a durable file"));
            }
            if sha256_hex(&read_bytes(&path)?) != digest {
                return fail(format!("{context}: source digest mismatch"));
            }
            let mut entry = Object::new();
            entry.insert("path".to_owned(), Value::String(relative));
            entry.insert("sha256".to_owned(), Value::String(digest));
            normalized_sources.push(Value::Object(entry));
        }
        if *sources != Value::Array(normalized_sources) {
            return fail("ADDITIONS.json source_paths do not match registration.sources");
        }

        Ok((registration, additions.clone()))
    }
}

fn apply_supplement(
    documents: &mut BTreeMap<String, Value>,
    registration: &toml::Table,
    additions: &Object,
) -> Result<()> {
    let Some(document) = documents.get_mut("markdown_corpus") else {
        return fail("markdown schema supplement has no markdown_corpus base schema");
    };
    let optional = additions["optional"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    for (index, addition) in optional.iter().enumerate() {
        let context = format!("ADDITIONS.json optional[{index}]");
        let parts = parse_pointer(
            &addition["instance_path"],
            &format!("{context}.instance_path"),
        )?;
        let Some((leaf, parents)) = parts.split_last() else {
            return fail(format!("{context}: malformed leaf path"));
        };
        let mut node = &mut *document;
        for segment in parents {
            if segment == "*" {
                let traversable = node.get("type").is_some_and(|kind| kind == "array")
                    && node.get("items").is_some_and(Value::is_object);
                if !traversable {
                    return fail(format!(
                        "{context}: wildcard does not traverse an array schema"
                    ));
                }
                node = &mut node["items"];
            } else {
                let present = node.get("type").is_some_and(|kind| kind == "object")
                    && node
                        .get("properties")
                        .and_then(Value::as_object)
                        .is_some_and(|properties| properties.contains_key(segment));
                if !present {
                    return fail(format!(
                        "{context}: instance path is absent from the base schema"
                    ));
                }
                node = &mut node["properties"][segment.as_str()];
            }
        }

        let Some(parent) = node.as_object_mut().filter(|_| leaf != "*" && leaf != "{}") else {
            return fail(format!("{context}: malformed leaf path"));
        };
        let closed = parent.get("type").is_some_and(|kind| kind == "object")
            && parent.get("additionalProperties") == Some(&Value::Bool(false))
            && parent.get("properties").is_some_and(Value::is_object);
        if !closed {
            return fail(format!("{context}: addition parent is not a closed object"));
        }
        if parent["properties"]
            .as_object()
            .is_some_and(|properties| properties.contains_key(leaf))
        {
            return fail(format!(
                "{context}: addition leaf already exists in the base schema"
            ));
        }
        if parent
            .get("required")
            .and_then(Value::as_array)
            .is_some_and(|required| required.iter().any(|key| key == leaf.as_str()))
        {
            return fail(format!("{context}: optional addition cannot be required"));
        }
        let types = json_type_list(addition.get("types")).unwrap_or_default();
        let mut property = Object::new();
        property.insert("type".to_owned(), schema_type(&types));
        if let Some(properties) = parent.get_mut("properties").and_then(Value::as_object_mut) {
            properties.insert(leaf.clone(), Value::Object(property));
        }
    }

    record_amendment(document, registration, additions)
}

/// Stamps the markdown schema with the registered amendment it carries.
fn record_amendment(
    document: &mut Value,
    registration: &toml::Table,
    additions: &Object,
) -> Result<()> {
    let Some(document) = document.as_object_mut() else {
        return fail("markdown schema base is not an object");
    };
    if document.contains_key("x-rha-amendment") {
        return fail("markdown schema base already contains x-rha-amendment");
    }
    let cites_amendment = additions["source_paths"].as_array().is_some_and(|sources| {
        sources
            .iter()
            .any(|source| source["path"] == SUPPLEMENT_AMENDMENT_PATH)
    });
    if !cites_amendment {
        return fail("markdown schema supplement does not cite its amendment contract");
    }
    let text = |key: &str| {
        registration
            .get(key)
            .and_then(toml::Value::as_str)
            .map_or(Value::Null, |text| Value::String(text.to_owned()))
    };
    let mut amendment = Object::new();
    amendment.insert(
        "contractpath".to_owned(),
        Value::String(SUPPLEMENT_AMENDMENT_PATH.to_owned()),
    );
    amendment.insert("ledgerid".to_owned(), text("grading_authority_ledger_id"));
    amendment.insert("content_sha256".to_owned(), text("content_sha256"));
    amendment.insert(
        "after_data".to_owned(),
        Value::Bool(
            registration
                .get("after_data")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false),
        ),
    );
    document.insert("x-rha-amendment".to_owned(), Value::Object(amendment));
    Ok(())
}

// ---------------------------------------------------------------------------
// Inventory translation

struct SelectionVariant {
    path: Pointer,
    selection: Object,
    branches: BTreeMap<String, NodeMap>,
}

struct KeyRule {
    path: Pointer,
    first: String,
    second: String,
}

struct SchemaBuilder<'a> {
    family: &'a str,
    nodes: NodeMap,
    extension_by_path: BTreeMap<Pointer, Object>,
    /// Extension paths in inventory order.
    extension_order: Vec<Pointer>,
    selection_variants: Vec<SelectionVariant>,
    rules: Vec<KeyRule>,
}

impl<'a> SchemaBuilder<'a> {
    fn new(family: &'a str, detail: &Value) -> Result<Self> {
        let Some(detail) = detail.as_object() else {
            return fail(format!("{family}: expected an object"));
        };
        let nodes = node_map(
            detail.get("nodes").unwrap_or(&Value::Null),
            &format!("{family}.nodes"),
        )?;
        if !nodes.contains_key(&Vec::new()) {
            return fail(format!("{family}: inventory has no root node"));
        }
        let mut builder = Self {
            family,
            nodes,
            extension_by_path: BTreeMap::new(),
            extension_order: Vec::new(),
            selection_variants: Vec::new(),
            rules: Vec::new(),
        };
        builder.validate_extensions(detail.get("extensions"))?;
        builder.validate_variants(detail.get("variants"))?;
        Ok(builder)
    }

    fn validate_extensions(&mut self, extensions: Option<&Value>) -> Result<()> {
        let empty = Vec::new();
        let extensions = match extensions {
            None => &empty,
            Some(Value::Array(extensions)) => extensions,
            Some(_) => return fail(format!("{}.extensions: expected a list", self.family)),
        };
        for (index, extension) in extensions.iter().enumerate() {
            let context = format!("{}.extensions[{index}]", self.family);
            let Some(extension) = extension.as_object() else {
                return fail(format!("{context}: expected an object"));
            };
            let path = parse_pointer(
                extension.get("path").unwrap_or(&Value::Null),
                &format!("{context}.path"),
            )?;
            if self.extension_by_path.contains_key(&path) {
                return fail(format!(
                    "{context}: duplicate path {}",
                    py_repr_str(&pointer(&path))
                ));
            }
            if json_type_list(extension.get("types")).is_none() {
                return fail(format!(
                    "{context}: types must be a non-empty list of unique JSON types"
                ));
            }
            match extension.get("values") {
                None | Some(Value::Null | Value::Array(_)) => {}
                Some(_) => return fail(format!("{context}.values: expected a list or null")),
            }
            match extension.get("required_in_parent") {
                None | Some(Value::Null | Value::Bool(_)) => {}
                Some(_) => {
                    return fail(format!(
                        "{context}.required_in_parent: expected boolean or null"
                    ));
                }
            }
            self.extension_order.push(path.clone());
            self.extension_by_path.insert(path, extension.clone());
        }
        Ok(())
    }

    fn validate_variants(&mut self, variants: Option<&Value>) -> Result<()> {
        let empty = Vec::new();
        let variants = match variants {
            None => &empty,
            Some(Value::Array(variants)) => variants,
            Some(_) => return fail(format!("{}.variants: expected a list", self.family)),
        };
        let mut seen = BTreeSet::new();
        for (index, variant) in variants.iter().enumerate() {
            let context = format!("{}.variants[{index}]", self.family);
            let Some(variant) = variant.as_object() else {
                return fail(format!("{context}: expected an object"));
            };
            let path = parse_pointer(
                variant.get("path").unwrap_or(&Value::Null),
                &format!("{context}.path"),
            )?;
            if !seen.insert(path.clone()) {
                return fail(format!(
                    "{context}: duplicate path {}",
                    py_repr_str(&pointer(&path))
                ));
            }
            if path.is_empty() {
                return fail(format!(
                    "{context}: a variant cannot replace the root schema"
                ));
            }
            if !self.nodes.contains_key(&path) {
                return fail(format!(
                    "{context}: path {} is absent from pooled nodes",
                    py_repr_str(&pointer(&path))
                ));
            }

            if variant
                .get("rule")
                .is_some_and(|rule| rule == "exactly_one_key_present")
            {
                match string_list(variant.get("keys")).as_deref() {
                    Some([first, second]) => self.rules.push(KeyRule {
                        path,
                        first: first.clone(),
                        second: second.clone(),
                    }),
                    _ => {
                        return fail(format!(
                            "{context}: exactly_one_key_present requires two keys"
                        ));
                    }
                }
                continue;
            }

            let Some(selection) = variant.get("selection").and_then(Value::as_object) else {
                return fail(format!(
                    "{context}: selection is required for a schema variant"
                ));
            };
            validate_selection(selection, &context)?;

            let Some(branches) = variant.get("branches").and_then(Value::as_object) else {
                return fail(format!("{context}: branches must be an object"));
            };
            let expected: BTreeSet<&str> = ["then", "else"]
                .iter()
                .filter_map(|field| selection[*field].as_str())
                .collect();
            let actual: BTreeSet<&str> = branches.keys().map(String::as_str).collect();
            if actual != expected {
                return fail(format!(
                    "{context}: branches do not match selector branch names"
                ));
            }

            let mut normalized_branches = BTreeMap::new();
            for branch_name in expected {
                let branch_context = format!("{context}.branches[{}]", py_repr_str(branch_name));
                let branch = &branches[branch_name];
                if !branch.is_object() {
                    return fail(format!("{branch_context}: expected an object"));
                }
                let branch_nodes = node_map(branch, &format!("{branch_context}.nodes"))?;
                if !branch_nodes.contains_key(&path) {
                    return fail(format!(
                        "{branch_context}: missing branch root {}",
                        py_repr_str(&pointer(&path))
                    ));
                }
                normalized_branches.insert(branch_name.to_owned(), branch_nodes);
            }

            self.selection_variants.push(SelectionVariant {
                path,
                selection: selection.clone(),
                branches: normalized_branches,
            });
        }
        Ok(())
    }

    fn has_node(&self, path: &Pointer, nodes: &NodeMap) -> bool {
        nodes.contains_key(path) || self.extension_by_path.contains_key(path)
    }

    fn is_direct_child(parent: &[String], child: &[String]) -> bool {
        child.len() == parent.len() + 1 && child[..parent.len()] == *parent
    }

    fn property_names(
        &self,
        path: &Pointer,
        node: &Object,
        nodes: &NodeMap,
    ) -> Result<Vec<String>> {
        let mut names = BTreeSet::new();
        if node
            .get("object_mode")
            .is_some_and(|mode| mode == "closed_record")
        {
            names.extend(string_list(node.get("allowed_keys")).unwrap_or_default());
        }
        let structural = |segment: &str| segment == "*" || segment == "{}";
        for child in nodes.keys().chain(self.extension_by_path.keys()) {
            if Self::is_direct_child(path, child) {
                let segment = &child[child.len() - 1];
                if !structural(segment) {
                    names.insert(segment.clone());
                }
            }
        }
        if let Some(name) = names.iter().find(|name| structural(name)) {
            return fail(format!(
                "{}{}: reserved structural key {}",
                self.family,
                pointer(path),
                py_repr_str(name)
            ));
        }
        Ok(names.into_iter().collect())
    }

    fn effective_required(&self, path: &Pointer, node: &Object) -> Vec<String> {
        let mut required: BTreeSet<String> = string_list(node.get("required_keys"))
            .unwrap_or_default()
            .into_iter()
            .collect();
        for extension_path in &self.extension_order {
            if !Self::is_direct_child(path, extension_path) {
                continue;
            }
            let name = &extension_path[extension_path.len() - 1];
            match self.extension_by_path[extension_path].get("required_in_parent") {
                Some(Value::Bool(true)) => {
                    required.insert(name.clone());
                }
                Some(Value::Bool(false)) => {
                    required.remove(name);
                }
                _ => {}
            }
        }
        required.into_iter().collect()
    }

    fn apply_enum(schema: &mut Object, types: &[String], extension: Option<&Object>) {
        let Some(values) = extension
            .and_then(|extension| extension.get("values"))
            .filter(|values| !values.is_null())
        else {
            return;
        };
        append_all_of(
            schema,
            serde_json::json!({
                "if": {"type": schema_type(types)},
                "then": {"enum": values},
            }),
        );
    }

    fn selector_schema(selection: &Object) -> Value {
        if selection["operator"] == "any_key_present" {
            let keys: Vec<Value> = selection["keys"]
                .as_array()
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .map(|key| serde_json::json!({"required": [key]}))
                .collect();
            return serde_json::json!({"type": "object", "anyOf": keys});
        }
        let type_key = selection["type_key"].as_str().unwrap_or_default();
        let mut properties = Object::new();
        properties.insert(
            type_key.to_owned(),
            serde_json::json!({"type": selection["type"]}),
        );
        serde_json::json!({
            "type": "object",
            "anyOf": [
                {"required": [selection["key"]]},
                {"required": [type_key], "properties": properties},
            ],
        })
    }

    fn variant_conditionals(
        &self,
        path: &Pointer,
        schema: &mut Object,
        disabled: &BTreeSet<Pointer>,
    ) -> Result<()> {
        for variant in &self.selection_variants {
            let variant_path = &variant.path;
            if !Self::is_direct_child(path, variant_path) || disabled.contains(variant_path) {
                continue;
            }
            let property_name = variant_path[variant_path.len() - 1].clone();
            let selection = &variant.selection;
            let selector = Self::selector_schema(selection);
            let then_name = selection["then"].as_str().unwrap_or_default();
            let else_name = selection["else"].as_str().unwrap_or_default();

            let mut inner_disabled = disabled.clone();
            inner_disabled.insert(variant_path.clone());
            let then_schema =
                self.render_node(variant_path, &variant.branches[then_name], &inner_disabled)?;
            let else_schema =
                self.render_node(variant_path, &variant.branches[else_name], &inner_disabled)?;

            let property = |value: Value| {
                let mut properties = Object::new();
                properties.insert(property_name.clone(), value);
                Value::Object(properties)
            };
            append_all_of(
                schema,
                serde_json::json!({
                    "if": {
                        "required": [property_name],
                        "properties": property(selector),
                    },
                    "then": {"properties": property(then_schema)},
                    "else": {"properties": property(else_schema)},
                }),
            );
        }
        Ok(())
    }

    fn key_rule(schema: &mut Object, rule: &KeyRule) {
        let (first, second) = (&rule.first, &rule.second);
        append_all_of(
            schema,
            serde_json::json!({
                "if": {
                    "type": "object",
                    "not": {
                        "anyOf": [
                            {"required": [first]},
                            {"required": [second]},
                        ],
                    },
                },
                "then": {"required": [first]},
            }),
        );
        append_all_of(
            schema,
            serde_json::json!({
                "if": {
                    "type": "object",
                    "required": [first, second],
                },
                "then": {"not": {}},
            }),
        );
    }

    fn render_node(
        &self,
        path: &Pointer,
        nodes: &NodeMap,
        disabled: &BTreeSet<Pointer>,
    ) -> Result<Value> {
        if self
            .selection_variants
            .iter()
            .any(|variant| variant.path == *path)
            && !disabled.contains(path)
        {
            return Ok(Value::Object(Object::new()));
        }

        let raw_node = nodes.get(path);
        let extension = self.extension_by_path.get(path);
        let types = match (extension, raw_node) {
            (Some(extension), _) => json_type_list(extension.get("types")),
            (None, Some(node)) => json_type_list(node.get("types")),
            (None, None) => {
                return fail(format!(
                    "{}: no inventory node at {}",
                    self.family,
                    py_repr_str(&pointer(path))
                ));
            }
        }
        .unwrap_or_default();
        let mode = raw_node
            .and_then(|node| node.get("object_mode"))
            .and_then(Value::as_str);
        let enforced = raw_node
            .and_then(|node| node.get("enforced"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let mut schema = Object::new();
        schema.insert("type".to_owned(), schema_type(&types));

        if !enforced {
            Self::apply_enum(&mut schema, &types, extension);
            return Ok(Value::Object(schema));
        }

        match (mode, raw_node) {
            (Some("closed_record"), Some(node)) => {
                let mut properties = Object::new();
                for name in self.property_names(path, node, nodes)? {
                    let mut child_path = path.clone();
                    child_path.push(name.clone());
                    if !self.has_node(&child_path, nodes) {
                        return fail(format!(
                            "{}{}: allowed key {} has no child observation or extension",
                            self.family,
                            pointer(path),
                            py_repr_str(&name)
                        ));
                    }
                    let child = self.render_node(&child_path, nodes, disabled)?;
                    properties.insert(name, child);
                }
                schema.insert("properties".to_owned(), Value::Object(properties));
                let required = self.effective_required(path, node);
                if !required.is_empty() {
                    schema.insert(
                        "required".to_owned(),
                        Value::Array(required.into_iter().map(Value::String).collect()),
                    );
                }
                schema.insert("additionalProperties".to_owned(), Value::Bool(false));
            }
            (Some("open_map"), Some(node)) => {
                let additional = if node["map_value_domain"] == "string" {
                    serde_json::json!({"type": "string"})
                } else {
                    Value::Bool(true)
                };
                schema.insert("additionalProperties".to_owned(), additional);
            }
            _ => {}
        }

        if types.iter().any(|kind| kind == "array") {
            let mut item_path = path.clone();
            item_path.push("*".to_owned());
            let unconstrained = raw_node
                .and_then(|node| node.get("item_domain"))
                .is_some_and(|domain| domain == "unconstrained_unobserved");
            let items = if !unconstrained && self.has_node(&item_path, nodes) {
                self.render_node(&item_path, nodes, disabled)?
            } else {
                Value::Object(Object::new())
            };
            schema.insert("items".to_owned(), items);
        }

        Self::apply_enum(&mut schema, &types, extension);
        self.variant_conditionals(path, &mut schema, disabled)?;
        for rule in &self.rules {
            if rule.path == *path {
                Self::key_rule(&mut schema, rule);
            }
        }
        Ok(Value::Object(schema))
    }

    fn render(&self) -> Result<Value> {
        self.render_node(&Vec::new(), &self.nodes, &BTreeSet::new())
    }
}

/// Validates a variant's selector operator and its branch names.
fn validate_selection(selection: &Object, context: &str) -> Result<()> {
    let operator = selection.get("operator");
    match operator.and_then(Value::as_str) {
        Some("any_key_present") => {
            if string_list(selection.get("keys")).is_none_or(|keys| keys.is_empty()) {
                return fail(format!("{context}: any_key_present requires string keys"));
            }
        }
        Some("key_present_or_type") => {
            for field in ["key", "type_key", "type"] {
                if !selection.get(field).is_some_and(Value::is_string) {
                    return fail(format!(
                        "{context}: {field} is required for key_present_or_type"
                    ));
                }
            }
            let selected = selection["type"].as_str().unwrap_or_default();
            if !JSON_TYPES.contains(&selected) {
                return fail(format!("{context}: unsupported selector type"));
            }
        }
        _ => {
            return fail(format!(
                "{context}: unsupported selector operator {}",
                py_repr(operator.unwrap_or(&Value::Null))
            ));
        }
    }

    for field in ["then", "else"] {
        if !selection.get(field).is_some_and(Value::is_string) {
            return fail(format!("{context}: selection.{field} must be a string"));
        }
    }
    Ok(())
}

fn append_all_of(schema: &mut Object, clause: Value) {
    if let Value::Array(clauses) = schema
        .entry("allOf")
        .or_insert_with(|| Value::Array(Vec::new()))
    {
        clauses.push(clause);
    }
}

fn node_map(value: &Value, context: &str) -> Result<NodeMap> {
    let Some(value) = value.as_object() else {
        return fail(format!("{context}: expected an object"));
    };
    let mut result = NodeMap::new();
    for (raw_path, node) in value {
        let path = parse_pointer(&Value::String(raw_path.clone()), &format!("{context} path"))?;
        if result.contains_key(&path) {
            return fail(format!(
                "{context}: duplicate decoded path {}",
                py_repr_str(&pointer(&path))
            ));
        }
        let Some(node) = node.as_object() else {
            return fail(format!("{context}{raw_path}: expected an object"));
        };
        validate_node(node, &format!("{context}{raw_path}"))?;
        result.insert(path, node.clone());
    }
    Ok(result)
}

fn validate_node(node: &Object, context: &str) -> Result<()> {
    let Some(types) = json_type_list(node.get("types")) else {
        return fail(format!(
            "{context}: types must be a non-empty list of unique JSON types"
        ));
    };
    if node
        .get("enforced")
        .is_some_and(|enforced| !enforced.is_boolean())
    {
        return fail(format!("{context}: enforced must be boolean"));
    }

    let mode = node.get("object_mode").filter(|mode| !mode.is_null());
    if let Some(mode) = mode {
        if !mode
            .as_str()
            .is_some_and(|mode| OBJECT_MODES.contains(&mode))
        {
            return fail(format!(
                "{context}: unsupported object_mode {}",
                py_repr(mode)
            ));
        }
        if !types.iter().any(|kind| kind == "object") {
            return fail(format!("{context}: object_mode requires object in types"));
        }
    }

    match mode.and_then(Value::as_str) {
        Some("closed_record") => {
            let mut lists = Vec::new();
            for field in ["required_keys", "allowed_keys"] {
                let Some(list) = string_list(node.get(field)) else {
                    return fail(format!("{context}: {field} must be a list of strings"));
                };
                lists.push(list);
            }
            let allowed: BTreeSet<&String> = lists[1].iter().collect();
            if !lists[0].iter().all(|key| allowed.contains(key)) {
                return fail(format!(
                    "{context}: required_keys is not contained in allowed_keys"
                ));
            }
        }
        Some("open_map") => {
            if node.get("required_keys") != Some(&Value::Array(Vec::new())) {
                return fail(format!("{context}: open_map required_keys must be []"));
            }
            if node.get("allowed_keys").is_some_and(|keys| !keys.is_null()) {
                return fail(format!("{context}: open_map allowed_keys must be null"));
            }
            if !node
                .get("map_value_domain")
                .and_then(Value::as_str)
                .is_some_and(|domain| domain == "any" || domain == "string")
            {
                return fail(format!(
                    "{context}: open_map has an unsupported map_value_domain"
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_inventory(inventory: &Value) -> Result<()> {
    let Some(inventory) = inventory.as_object() else {
        return fail("inventory root must be an object");
    };
    let format = inventory.get("format");
    if format.and_then(Value::as_str) != Some(INVENTORY_FORMAT) {
        return fail(format!(
            "inventory format must be {}, got {}",
            py_repr_str(INVENTORY_FORMAT),
            py_repr(format.unwrap_or(&Value::Null))
        ));
    }
    if !inventory
        .get("source_revision")
        .is_some_and(Value::is_string)
    {
        return fail("inventory source_revision must be a string");
    }
    let Some(families) = inventory.get("families").and_then(Value::as_object) else {
        return fail("inventory families must be an object");
    };
    let missing: Vec<&str> = FAMILIES
        .iter()
        .copied()
        .filter(|family| !families.contains_key(*family))
        .collect();
    let mut extra: Vec<&str> = families
        .keys()
        .map(String::as_str)
        .filter(|family| !FAMILIES.contains(family))
        .collect();
    extra.sort_unstable();
    if !missing.is_empty() {
        return fail(format!(
            "inventory is missing families: {}",
            missing.join(", ")
        ));
    }
    if !extra.is_empty() {
        return fail(format!(
            "inventory has unsupported families: {}",
            extra.join(", ")
        ));
    }
    Ok(())
}

fn title_for(family: &str) -> String {
    let label = match family {
        "h4" => "H4",
        "h5_conformance" => "H5 conformance",
        "markdown_corpus" => "Markdown corpus",
        other => other,
    };
    format!("RHA {label} record schema")
}

fn build_schemas(inventory: &Value, inventory_bytes: &[u8]) -> Result<BTreeMap<String, Value>> {
    let inventory_sha256 = sha256_hex(inventory_bytes);
    let mut schemas = BTreeMap::new();
    for family in FAMILIES {
        let builder = SchemaBuilder::new(family, &inventory["families"][family])?;
        let root_schema = builder.render()?;
        let mut document = Object::new();
        let mut text = |key: &str, value: &str| {
            document.insert(key.to_owned(), Value::String(value.to_owned()));
        };
        text("$schema", SCHEMA_URI);
        text("title", &title_for(family));
        text(
            "description",
            "Structural record-shape schema translated from the registered \
             RHA observation inventory; it is not an evaluator.",
        );
        text("x-rha-contract", CONTRACT_PATH);
        text("x-rha-family", family);
        text("x-rha-inventory-format", INVENTORY_FORMAT);
        text("x-rha-inventory-sha256", &inventory_sha256);
        document.insert(
            "x-rha-source-revision".to_owned(),
            inventory["source_revision"].clone(),
        );
        if let Value::Object(root_schema) = root_schema {
            document.extend(root_schema);
        }
        schemas.insert(family.to_owned(), Value::Object(document));
    }
    Ok(schemas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_repr_matches_python() {
        for (value, expected) in [
            (1.0, "1.0"),
            (0.5, "0.5"),
            (-2.25, "-2.25"),
            (1e16, "1e+16"),
            (1e15, "1000000000000000.0"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (1.5e-7, "1.5e-07"),
            (123.456, "123.456"),
            (-0.0, "-0.0"),
            (1.2345e100, "1.2345e+100"),
        ] {
            assert_eq!(py_float_repr(value), expected, "{value}");
        }
    }

    #[test]
    fn canonical_json_matches_python_layout() {
        let value = serde_json::json!({"b": [1, {}, [], "\u{1}\"é"], "a": {"x": null, "c": true}});
        let expected = "{\n  \"a\": {\n    \"c\": true,\n    \"x\": null\n  },\n  \"b\": [\n    1,\n    {},\n    [],\n    \"\\u0001\\\"é\"\n  ]\n}\n";
        assert_eq!(
            String::from_utf8(canonical_json(&value)).as_deref(),
            Ok(expected)
        );
    }

    #[test]
    fn pointer_round_trip_and_refusals() {
        let parsed = parse_pointer(&Value::from("/a~1b/~0c/*"), "t");
        assert_eq!(
            parsed.as_ref().map(|parts| pointer(parts)).ok().as_deref(),
            Some("/a~1b/~0c/*")
        );
        assert!(parse_pointer(&Value::from("a"), "t").is_err());
        assert!(parse_pointer(&Value::from("/a~2"), "t").is_err());
        assert!(parse_pointer(&Value::Null, "t").is_err());
    }

    #[test]
    fn repr_matches_python() {
        assert_eq!(py_repr_str("a'b"), "\"a'b\"");
        assert_eq!(py_repr_str("a'\"b"), "'a\\'\"b'");
        assert_eq!(py_repr_str("x\ny"), "'x\\ny'");
        assert_eq!(py_repr(&Value::Null), "None");
        assert_eq!(py_repr(&serde_json::json!([1, true])), "[1, True]");
    }
}
