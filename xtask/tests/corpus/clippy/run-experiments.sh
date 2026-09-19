#!/usr/bin/env bash
# Runs the CHG-001 Clippy experiments and writes one record per experiment:
# the literal command, the fixture, the revision, the exit status, and the
# captured stderr and stdout. It records observations only; ADR-0002 states
# what they mean.
#
# Usage (from the repository root): xtask/tests/corpus/clippy/run-experiments.sh evidence/w1-clippy
set -u

out=${1:?usage: run-experiments.sh OUT_DIR}
root=$(git rev-parse --show-toplevel)
corpus="$root/xtask/tests/corpus/clippy"
mkdir -p "$out"
out=$(cd "$out" && pwd)
unset CLIPPY_CONF_DIR

revision=$(git -C "$root" rev-parse HEAD)
dirty=$([ -n "$(git -C "$root" status --porcelain)" ] && echo yes || echo no)
clippy_version=$(cd "$root" && cargo clippy --version)

# record ID FIXTURE_DIR PURPOSE COMMAND...
record() {
    local id=$1 dir=$2 purpose=$3
    shift 3
    local file="$out/$id.txt" stdout stderr status
    stdout=$(mktemp) stderr=$(mktemp)
    (cd "$dir" && cargo clean -q 2>/dev/null; "$@" >"$stdout" 2>"$stderr")
    status=$?
    {
        echo "experiment:   $id"
        echo "purpose:      $purpose"
        echo "fixture:      ${dir#"$root"/}"
        echo "preparation:  cargo clean in the fixture; CLIPPY_CONF_DIR unset"
        echo "command:      $*"
        echo "clippy:       $clippy_version"
        echo "revision:     $revision (working tree dirty: $dirty)"
        echo "recorded_at:  $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo "exit_status:  $status"
        echo "--- stderr ---"
        cat "$stderr"
        echo "--- stdout ---"
        cat "$stdout"
    } >"$file"
    rm -f "$stdout" "$stderr"
    echo "$id: exit $status -> ${file#"$root"/}"
}

record 1-core-seeded-disallowed-method "$corpus/core-seeded" \
    "a core crate with the template calls std::time::SystemTime::now()" \
    cargo clippy -p core-a

record 2-discovery-adapter-same-call "$corpus/discovery" \
    "an adapter with no local clippy.toml makes the same call" \
    cargo clippy -p adapter-x

record 3-discovery-core-test-unwrap "$corpus/discovery" \
    "allow-unwrap-in-tests only in the fixture root clippy.toml; core-a's local file holds only the deny list; core-a's test calls unwrap()" \
    cargo clippy -p core-a --all-targets

record 3c-discovery-adapter-test-unwrap "$corpus/discovery" \
    "control for 3: the same test unwrap() in adapter-x, whose nearest clippy.toml is the fixture root" \
    cargo clippy -p adapter-x --all-targets

record 3b-core-seeded-template-test-unwrap "$corpus/core-seeded" \
    "the template repeats the root test allowances; core-b (nothing on the deny list) has a test that calls unwrap()" \
    cargo clippy -p core-b --all-targets

record 4-core-seeded-every-entry "$corpus/core-seeded" \
    "one use of every deny-list entry in a crate with the template" \
    cargo clippy -p core-every

# 5: clippy.toml and .clippy.toml in one directory (plan W1 unknown). Built
# in the root target directory because it is not a committed fixture.
pair="$root/target/w1-clippy/pair"
rm -rf "$pair" && mkdir -p "$pair/src"
printf '[package]\nname = "pair"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[workspace]\n' >"$pair/Cargo.toml"
printf 'pub fn f() {}\n' >"$pair/src/lib.rs"
printf 'allow-unwrap-in-tests = true\n' >"$pair/clippy.toml"
printf 'allow-expect-in-tests = true\n' >"$pair/.clippy.toml"
record 5-config-file-pair "$pair" \
    "a crate directory holding both clippy.toml (allow-unwrap-in-tests = true) and .clippy.toml (allow-expect-in-tests = true)" \
    cargo clippy
