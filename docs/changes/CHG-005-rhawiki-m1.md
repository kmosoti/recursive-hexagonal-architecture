# CHG-005 (P-A): the rhawiki product and the M1 close

Task record: [`.rha/tasks/CHG-005-rhawiki-m1.toml`](../../.rha/tasks/CHG-005-rhawiki-m1.toml). Plan: §8.1 P-A; §8.2 W5 and W6. Base: `72c5cfd`, the merge that approved plan revision 2.

## Intent and scope

Packet P-A of plan revision 2 covers CHG-005 (W5) and CHG-006 (W6). It builds the guards revision 2 names, then the product and the M1 close, in gated stages.

## Deltas

### Stage 0, guards

- `cargo xtask scope --task <id>` (plan §2.2 M4): every changed path, committed or not, must match the task's `scope_globs`, sit in the §4 layout, and, if protected, match `protected_scope`.
- The enforcement map claims no cell from an H4 record that graded an earlier manifest.
- Mechanical pedantic warnings in `xtask/src/corpus` are fixed; length and naming warnings stay advisory under §6.4.
- `corpus held-out --kind check` moves to stage 3, where `rhawiki check` exists (task record decision `held-out-check-kind-in-stage-3`).

**The guard's first run caught this packet.** Stage 0 had added a line about the new command to CONTRIBUTING.md, which `.rha/policy.toml` has protected since CHG-000, and no approval quoted it. The line is reverted. It is proposed for Kennedy's approval with DP-5.2.

### Stage 1, boundary decisions (§7.8 step 10) and corpus registration

- **Best-of-N (§2.4 rule 3).** Three decompositions were built as stub workspaces and run through `cargo xtask architecture`; the reports are in `evidence/CHG-005/bdr-candidates/`. A is the plan's §3, B merges `library` and `document`, C splits `assembly` and `build` into crates. All pass with zero errors, so the §7.2 evidence decides. A is chosen, and each BDR records B and C with the reasons they lost.
- **BDR-0001 to BDR-0004** in `docs/adr/` each carry a proposed refutation criterion (metric, source, window, threshold, action). **Gate DP-1.3:** Kennedy accepts or amends the four criteria before any product crate exists. BDR-0003 records the real trade-off. Until P-C validates the module check, the direction between `assembly` and `build` is enforced by review only. Candidate C is the fallback its criterion names.

- **Markdown corpus registered (§2.4 rules 2 and 5).** A separate Codex session (`gpt-6-astra`, extra-high) generated 60 sites, 300 pages and 99 planted witnesses by construction, with 20 clean sites. It ran before any product crate existed. `xtask/tests/corpus/markdown/registration.toml` records the generator, the prompt digest (the prompt itself is committed beside it), and the tree digest, which `xtask/tests/corpus_markdown.rs` pins. The implementing session has not edited a site. The witness keys it uses fix part of the `check --format json` schema in advance; stage 3 conforms to them.

### Stage 2, cores

Decomposition A, as BDR-0001 to BDR-0004 accepted it. Every core has the template `clippy.toml`, the crate-root `forbid` line, and `role = "core"`. `cargo xtask architecture` classifies them by metadata and reports 0 errors and 0 warnings.

- **`library`**: `PageId` (NFC, case kept), `RelPath`, `Digest` (sha256), `Source`, and `Corpus`. `Corpus` is strict through `new`; `with_witnesses` keeps the first path per id and returns one `DuplicatePageId` per duplicated id. `load` is complete-or-fail. The `SourceRepository` contract suite catches all four seeded violators of the W5 brief, and `MemorySources` passes it.
- **`document`**: a total parser to an owned node tree. It keeps headings with base and final slugs, where a repeat gets `-2`, `-3` and a `DuplicateSlug` diagnostic. Wikilinks are split into target, anchor and alias; links in code spans and blocks are ignored. `ENABLE_HEADING_ATTRIBUTES` is off (decision `heading-attributes-off`).
- **`graph`**: exact-id, then unique case-insensitive basename resolution; broken, ambiguous and missing-anchor witnesses; symmetric backlinks. Properties: every link resolves or is witnessed, backlinks are symmetric, and results do not depend on input order.
- **`site`**, the composite: `assembly` (`Assemble`, `PageModel`, the TOC, backlinks and link targets) and `build` (`PageRenderer`, the pure `step`, `Inv_K` checked by a separate function), glued to `OutputSink` and `Clock`. `rha-modules.toml` has the plan §3.1 content, and a scoped `AGENTS.md` states the child rules. The contract suites catch a panicking and a nondeterministic renderer, and the fakes pass.
- `check` witnesses use the registered corpus's key shape (`site::CheckWitness`).

**Differential oracles (§2.4 rule 4).** A separate Codex session (`gpt-6-astra`, extra-high) wrote independent reference implementations of `slugify` and of resolution. It worked from the contract alone and was told not to read `crates/*/src`. The results are `crates/document/tests/oracle_slug.rs` and `crates/graph/tests/oracle_resolve.rs`, compared under proptest.

- **The resolver oracle agreed from the first run.**
- **The slug oracle found a real defect.** I lowercased character by character, so a word-final `Σ` became `σ`. The oracle lowercased the whole string, which applies Unicode's context-sensitive final sigma and gives `ς`, as GitHub's slugger does. The contract's "Unicode-lowercase" is best read as the full string mapping, so the product changed. The minimal inputs were `HΣ२Z` and `e\u{301}Σ`. No registered site contains `Σ`, so the corpus result is unaffected.
- **One edit to the oracle.** It had been given the W5 brief's NFC step, and its NFC line was removed to match the registered contract (decision `slug-without-nfc`). The file marks that edit.

### Stage 3, adapters and CLI

- **`adapter-fs`**: `FsSources` and `FsSink`. It walks directories itself, never follows symlinks, and writes through a temporary file and a rename. Both owners' contract suites pass.
- **`adapter-html`**: one escape function, with a property test that decodes its output. It renders the TOC, backlinks, relative wikilink hrefs, broken links as marked spans, `pre.mermaid` blocks, and the footer. The renderer contract passes. It depends on `site` only, through re-exports, so the checker reports **0 errors and 0 warnings** over the whole product.
- **`adapter-sys`**: `SystemClock` with an RFC 3339 formatter (DP-1.2 recorded as its default, yes).
- **`app-cli`** is the `rhawiki` binary with `build` and `check --format json`. The check schema is `{schema_version, pages, witnesses[{kind, …}], counts}`, with witness keys exactly as the registered corpus uses them. **Gate DP-1.4:** Kennedy's private markdown fixtures can now be written against this schema. `cargo xtask corpus held-out --kind check --archive <tar>` runs them, one site per subdirectory, printing only opaque ids, exit statuses and counts.
- **On this repository:** `rhawiki check --root docs` finds 26 pages and 0 witnesses (DP-1.5's expectation). `rhawiki build` renders the spec with exactly **182** headings and **2** mermaid blocks, as W5 requires. `xtask/tests/` in app-cli pins both.

### Stage 4, markdown corpus run

The first run graded **59 of 60** sites as registered. MD051 failed: the product applied NFC to slugs, which the W5 brief asks for. The registered contract derives slugs without NFC, so a combining mark is dropped. The registration was fixed first and grades the product, so the product now follows it (decision `slug-without-nfc`). The disclosed defect is the Executor's: the brief's NFC was dropped when the registration contract was written. After the change, **60 of 60** sites pass. The first run was a check, not a recorded harness run, so no failed record exists for it; this paragraph is its record. `cargo xtask corpus run --level markdown` writes the evidence below, and an app-cli test pins it under `L0.nextest`.

**Evidence:** `evidence/md-corpus/20260922T210717Z-58e45f444c9f.json` at `58e45f4`, clean tree: 60 of 60 sites graded as registered, with the registration's tree digest verified before the run.

### Stage 5, M1 close

- `.rha/assumptions.toml`: nine entries from plan §3.3, each naming the test that discharges it. Two are undischarged at the composition root.
- `docs/conformance/self-assessment.md`: every §15 item answered, with the gaps named.
- `docs/maturity.md` proposes: the crate-graph checker at V (crate level, from CHG-004.6); the deny list at V (crate level); the fast lane at V; held-out checks at I.
- **L0 at the settled head:** `evidence/CHG-005/20260922T211312Z-e213c0b52b98.json` at `e213c0b`, clean tree, all eight `passed`, 188 tests. Tag `v0.10-m1` follows Kennedy's merge.

### Review round 1 (Codex, `gpt-6-astra`, extra-high, read-only) on `3c4a41f`, determinations

REQUEST_CHANGES: 11 findings, 2 P1 and 9 P2. Each names a concrete failing input or scenario on the reviewed revision, and all are confirmed and repaired in the commit after this one.

1. **P1: writes escape through directory symlinks.** `FsSink::write` used `create_dir_all` and `write`, which follow a symlinked directory under `--out`. Repair: every existing component of the target path is checked with `symlink_metadata`, a link is refused, and directories are created one level at a time. `delete` gets the same check.
2. **P1: predictable temporary names.** A pre-planted `.rhawiki-tmp-…` symlink was truncated through, and a source page named like a temporary file had its output overwritten and renamed. Repair: temporary files are `.rhawiki-tmp-<pid>-<counter>.tmp`, opened with `create_new`, so an existing file or link fails. `list` hides only that exact pattern, which no renderer output can take, because pages become `.html`.
3. **P2: generated slug suffixes collided with natural slugs.** `# A`, `# A`, `# A-2` gave `a`, `a-2`, `a-2`. Repair: the allocator tracks every final slug, and both a generated suffix and a colliding natural slug advance to the next unused one: `a`, `a-2`, `a-2-2`. `DuplicateSlug` still fires only for a repeated base, as the contract says.
4. **P2: wikilink hrefs were not URL-encoded.** `Budget?2026` became a query. Repair: path segments and fragments are percent-encoded outside RFC 3986's unreserved set, for wikilinks and backlinks alike.
5. **P2: ordinary relative `.md` links pointed at files that are never produced.** Repair: a relative destination with no scheme and a `.md` path becomes `.html`, keeping any fragment. Absolute and scheme URLs are unchanged.
6. **P2: the markdown held-out mode verified against the crate-level commitment.** Repair: `--kind check` reads DP-1.4's commitment, and it is `not_run` while that row carries none.
7. **P2: the markdown harness could run a stale binary.** It ran `target/debug/rhawiki` whatever `CARGO_TARGET_DIR` said. Repair: the executable path comes from Cargo's own build output (`--message-format=json`).
8. **P2: invalid checker output passed clean sites.** Empty stdout, `{}`, or a non-array `witnesses` became an empty multiset. Repair: a case is graded only when the output is JSON with `schema_version == 1` and a `witnesses` array; anything else fails the case.
9. **P2: renames hid protected sources from the scope guard.** Git's rename detection listed only the destination. Repair: `--no-renames`, so both endpoints are checked.
10. **P2: the source contract could not detect consistent truncation.** The seeded `TruncatingSource` alternated, so only instability was tested, and the change record's claim about the W5 violators was unsupported. Repair: the suite takes the fixture's expected contents and compares every read with them. The seeded violator now truncates every read, as the W5 brief describes. The unstable-read case is kept as its own violator.
11. **P2: the oracles lacked generation provenance (§2.4 rule 1).** Repair: the oracle prompt is committed beside the markdown corpus's prompt. The task record gains a `[[provenance.generations]]` entry for each generated artifact (the corpus and both oracles), with model, effort, prompt path and digest.

**Repairs committed in `25b035e`.** L0 at that revision: `evidence/CHG-005/20260922T213429Z-25b035eac999.json`, clean tree, all eight `passed`, 192 tests. The earlier stage 5 record describes the reviewed revision and is kept.

### Review round 2 on `52ceb20`, determinations

Ten of 11 were confirmed resolved. On finding 5, and one new P2 in the same function: `rewrite_href` took the query as part of the path (`x.md?print=1` kept `.md`, and `download?file=manual.md` was rewritten), and it took any colon as a scheme (`./a:b.md`). **Confirmed.** Repair: the reference is split as RFC 3986 parses it, fragment then query. A scheme is recognized only by §3.1's syntax before any `/`, `?` or `#`. Only the path's `.md` suffix is rewritten, and the query and fragment are kept byte for byte. All four inputs are regression tests.

**A void review run.** The first attempt at round 3 reviewed `52ceb20` unchanged, because the Executor's script failed before committing the repair: a rustfmt-reformatted anchor did not match. Its verdict describes the old code and is not counted as a round under M6. Round 3 below reviews the committed repair.

### Automated review threads on PR 16 (`chatgpt-codex-connector`, on `8b81dd5`), determinations

Two threads were open after round 3. Each was checked against the code before any repair.

1. **A symlinked output root (P1). Confirmed.** `prepare` and `delete` checked every segment *below* the root and `list` walked the root with `read_dir`, which follows a link. So `--out` pointing at a symbolic link to a populated directory would inventory that directory, and the build would delete what it did not produce there. Repair: `FsSink` refuses a root that is itself a symbolic link, before `list`, `write` and `delete`. Discriminating check: a new contract test, `a_symlinked_output_root_is_refused_before_listing_writing_or_deleting`, fails on `8b81dd5` (the target is inventoried) and passes on the repair; the seeded file is untouched. A symbolic link *source* root (`--root`) stays allowed: reading through it deletes nothing.
2. **A panic on a non-ASCII glob segment (P2). Refuted.** `glob`'s `segment` matches `&[u8]`, and a byte slice has no character boundary to split, so `&text[i..]` cannot panic. Discriminating check: `glob("docs/*.md", "docs/café.md")`, `glob("docs/c*é.md", "docs/café.md")` and the negative `docs/*.rs` ran against `8b81dd5` and gave the expected answers without a panic. Byte-wise matching agrees with character-wise matching here because the only wildcard is `*` and literals compare whole UTF-8 sequences. The three assertions are kept as a regression test; no code changed.

### Repair attempts (§11.7.10)

2. **`L0.typos` failed in CI run 35784806281, and the lane had not been run locally.** Hypothesis: the spell checker splits Unicode escapes such as `\u{e9}` and reads the letters before them as a word; one variable name also read as a misspelling. (This note does not quote it, which is CHG-002's lesson.) Discriminating check: `typos --format brief` reproduced all nine findings locally. Change: the characters are written literally and the variable is renamed. No dictionary exception was added, following CHG-003's precedent. Result: `typos` is clean. A method lesson too: M5's full lane belongs before the first push, not only before the record.

1. **The scope guard reported 52 false findings on the corpus commit.** The hypothesis was git's default path quoting. The discriminating check: every flagged path was a non-ASCII name, such as `café.md`, printed as a quoted string of octal escapes. The change turns off `core.quotePath` in every git call the guard makes. Result: 0 findings. The adversarial corpus found a defect in the guard before any product code existed.

## Acceptance concerns

1. **An earlier protected edit without a quoted approval, found by the new guard.** CHG-004.6's commit `ad13efa` added the held-out paragraph to CONTRIBUTING.md; its approval was recorded only as owned scope. Resolved: approved retroactively under Kennedy's delegation of 2026-09-22 (task record decision `contributing-scope-line`; ledger DP-5.2).
2. **The CONTRIBUTING line for `cargo xtask scope`.** Resolved: restored, with DP-5.2 decided as a pre-PR command.
3. **DP-1.4, the private markdown held-out fixtures, is not_run.** Authoring them is Kennedy's alone (§9.14), so the delegation cannot close it. The row stays open; a run after the merge is dated as a post-acceptance observation.
