You are the independent oracle author for feature CHG-011 (citations, `[Rn]`) of the RHA repository at /home/kmosoti/projects/rha-m2 (branch chg/008-growth, HEAD e2a2f95). You write the registered grading corpus and its graders BEFORE any implementation exists; a different model will implement later. Do not read parser or renderer internals.

## Scratch boundary (strict)
Do all scratch work, stub crates and build directories ONLY under:
/tmp/claude-1000/-home-kmosoti-projects-recursive-hexogonal-architecture/0ce0c856-7b1f-49da-afa8-4880e034d2d1/scratchpad/chg011-oracle/
Never delete, move or clean anything outside that folder (not the scratchpad's other contents, not /tmp). Never write inside /home/kmosoti/projects/rha-m2.

## Read
- The contract (your specification, authoritative): docs/architecture/citation-contract.md. Also docs/architecture/section-reference-contract.md, section 2, which the citation contract reuses for where citations are recognized.
- The convention to mirror: xtask/tests/corpus/section-refs/ (registration.toml, SHA256SUMS, README.md, CASES.json, reference.py, source-snapshots/, selftestreport.txt) and its grader crates/document/tests/section_refs.rs. Note that grader embeds its inputs with include_bytes! (core crates may not use std::fs, even in tests); do the same for any grader in a core crate.
- For API shapes only: crates/document/src/lib.rs (Document, Diagnostic, parse), crates/document/src/node.rs (Node), crates/library/src/lib.rs (Source::new, RelPath). For the end-to-end grader: how crates/app-cli/tests/transclusion.rs builds a docs tree and runs the binary (app-cli is not a core crate; it may use std::fs).
- The real spec: docs/spec/rha-spec-v0.10.md (frozen).

## Write (all under the scratch folder, mirroring repository paths)
chg011-oracle/package/xtask/tests/corpus/citations/
  reference.py        an independent Python implementation of the contract: given markdown, the expected reference_entries (label, anchor, line), citations (label, target anchor or null, line), and diagnostics (DuplicateReferenceEntry {label, first_line, second_line}, UnresolvedCitation {label, line}). Use your own markdown scanner (the section-refs reference.py is a good model of the exclusion rules).
  CASES.json, README.md, registration.toml (model = "claude-sonnet-5", effort = "medium"; source digests for the contract and this prompt, saved under source-snapshots/), SHA256SUMS, selftestreport.txt, source-snapshots/.
chg011-oracle/package/crates/document/tests/citations.rs
  The core grader, with inputs embedded by include_bytes!. For each case, assert that Document::reference_entries, Document::citations and the two diagnostics equal the expectations exactly. Also assert the node-level invariants: every resolved citation has a Node::Link with href "#"+anchor and text exactly the citation; each entry paragraph's first inline node is Node::Anchor { id: anchor }; there are no other Node::Anchor nodes; anchor ids are unique.
chg011-oracle/package/crates/app-cli/tests/citations_render.rs
  An end-to-end grader: build a small docs tree with entries, citations, a range [R1]-[R2], an unresolved citation, and a page that transcludes a section containing entries. Run `rhawiki build`, then assert: the HTML contains `<a id="ref-r1"></a>` inside the entry paragraph; citations link to `#ref-r1`; the transcluding host page contains no duplicate anchor id; `--format json` output has {"type": "anchor", "id": ...} nodes and the anchor adds no text to the search text; and `rhawiki check --format json` output on this tree has the same witness kinds as before (no citation witnesses).

## Cases: 60 to 120 synthetic cases plus the spec
Cover deliberately:
- entry forms: `[R1]`, `[R12a]`; an entry at the end of the paragraph; entries in list items and block quotes; not an entry when the label is not first, when `[R1]` is followed by a non-space, or for `[r1]` or `[R]` or `[R1A]`;
- duplicate entries (the diagnostic, and the second label counting as a citation);
- citations in paragraphs, lists, tables, quotes and emphasis;
- excluded contexts: headings, code, code blocks, HTML, link text and destination, wikilinks, transclusions and image alt;
- a markdown link reference definition `[R1]: http://x` (then `[R1]` is a link, not a citation);
- `[R1]-[R72]`; `[R1, R2]` (not a citation); adjacent punctuation;
- unresolved citations on a page with entries (diagnostic) and on a page without (none);
- citations inside an entry's own text;
- the real spec as one case, by path and sha256.

Where the contract is ambiguous, do not guess: list the case under "Open questions" in README.md and leave it out.

Report: the file list, case count, the spec's counts (entries, citations, diagnostics), every open question, and what you ran. Stub-compile your graders against a throwaway stub of the contract's API inside your scratch folder, and say so.
