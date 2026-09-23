# Transclusion contract

This is the additive contract for transclusion and section selection. It is
written before implementation. It does not change the existing HTML/JSON
renderer contracts, silently regrade existing corpora, or propose a new crate.

## Document syntax and selection

`document` owns:

```text
Transclusion {
    id: usize,
    target: String,
    anchor: Option<String>,
    display: String,
    line: usize,
}
```

`Node::Transclusion(Transclusion)` is parser-owned only. IDs are contiguous
preorder IDs among recognized embeds in one source document and are stable
under document permutation. `Document.links` remains navigation-only with its
existing indexing and semantics.

A pulldown WikiLink image is a transclusion candidate only when it has no alias,
has a nonempty page target, and, when `#` is supplied, has a nonempty anchor.
The candidate must be the sole meaningful content of a paragraph or tight list
item; surrounding spaces and tabs are permitted. Recognition is recursive
through quotes and list items. Code blocks and code spans are never candidates.
Candidates with other content remain `Image`; aliases remain `Image`.

The measured exclusions are normative: escaped bangs and spaced bangs retain
their literal-bang plus navigation behavior; inline and aliased `![[...]]` are
images, not navigation links; two images separated by a soft break are not
standalone; and four-space indentation remains code. Recognition is structural,
not a blanket regular expression over raw text. Internal normalization is
permitted, but an unrecognized candidate must never leak as `Transclusion`.

The public selector is:

```text
section_nodes(&Document, Option<&str>) -> Option<Vec<document::Node>>
```

`None` selects the full body. An anchor matches a final slug exactly and
case-sensitively; an absent slug returns `None`. It selects that heading and following
siblings in that heading's immediate block sequence until the next heading of
less than or equal level. Nested headings do not terminate an outer section.
Selection recursively traverses quotes and lists, preserving ancestor shells
without unrelated siblings. A list wrapper retains only the selected item and
adjusts an ordered list's start by that item's position. Slugs remain the
original document slugs. Inputs are parsed `Document` values; no parser type is
exposed. A pure traversal helper supplies transclusion descriptors.

The traversal helper is `transclusions(&[Node]) -> Vec<&Transclusion>` in
deterministic tree order. It retains the recognized descriptors' source IDs.

## Graph

Ordinary navigation remains unchanged. `Resolved` is unchanged and no
`anchor_valid` field is added. Transclusions are resolved separately using the
existing exact-ID and unique case-insensitive-basename rules. A page that
resolves contributes a backlink even when its requested anchor is missing,
consistent with navigation. Missing or ambiguous pages do not. Expansion is
valid only when `section_nodes` succeeds.

Transclusion diagnostics are distinct from navigation diagnostics:

```text
BrokenTransclusion       { from, target, line }
AmbiguousTransclusion    { from, target, candidates, line }
MissingTransclusionAnchor{ from, target, heading, line }
TransclusionCycle        { path: Vec<TransclusionRegion> }
```

The graph defines:

```text
TransclusionRegion     { page: PageId, anchor: Option<String> }
TransclusionOccurrence { source: TransclusionRegion, transclusion_id: usize }
TransclusionCycle      { path: Vec<TransclusionRegion> }
```

The following text abbreviates these as Region and Occurrence. `SiteGraph`
provides `transclusion_target(&TransclusionOccurrence) -> Option<&TransclusionRegion>`
and `blocked_transclusion(&TransclusionOccurrence) -> Option<&TransclusionCycle>`.

Regions order by page, then `None` before `Some(anchor)`. Edge target lookup is
keyed by the full source `Occurrence`, never only by page and transclusion ID.
The graph may use a temporary global resolution map internally, while assembly
may follow only analyzed edges. Read-only target and blocked-cycle lookup is
available by `Occurrence`.

The graph builds a region for every full page and every valid anchored target.
Each region contains only recognized embeds inside its selected fragment. Roots
order by `Region`; outgoing edges order by transclusion ID, then target.
Deterministic DFS blocks every edge to an active region. A cycle witness is the
active-stack suffix plus the repeated target, canonically rotated to the least
`Region`; closed paths are deduplicated. Removing blocked occurrences leaves a
DAG. Cycles are graph-owned witnesses.

The global check covers all pages and regions. Selecting a fragment excludes
unrelated edges from that region but does not hide a separate cycle elsewhere.
Thus page-level `A -> B#keep`, with `B#other -> A`, is acyclic when those are the
only edges in the selected regions; an independent `B#other` self-cycle remains
a global witness but produces no cycle marker in `A`'s selected `keep` region.

## Assembly and page model

`Assemble` receives the current document, the complete explicit `&[Document]`,
graph, title lookup, and context. `site::build` passes its existing document
slice. Assembly never reaches build or glue.

Assembly replaces each recognized transclusion with its selected fragment and
recurses using the exact occurrence lookup. A blocked edge becomes a literal
semantic node:

```text
Paragraph([Text("[transclusion cycle: A#one -> B#two -> A#one]")])
```

Missing, ambiguous, or invalid targets become:

```text
Paragraph([Text("[transclusion unavailable: <display>]")])
```

The same semantic marker is consumed by HTML and JSON. The host TOC remains
host-only. `PageNode` contains no transclusion variant.

All original host navigation slots and indices are preserved as assembly
currently maps `Document.links`. Imported visible wikilinks append deterministic
slots in traversal order, resolve against their origin document and origin
index, and append separately when an import repeats. The attempted hidden-alt
probe is not evidence of a bug and must not be represented as one.

Ordinary links and images in the original host body are copied unchanged.
Origin rebasing applies to imported content; a self-import still receives its
own heading namespace and the same explicit import-path rules.

Imported heading IDs use the occurrence chain:

```text
tx-<outer id>-<nested id>-<original slug>
```

Used IDs are seeded with every host heading. All mappings for one fragment are
allocated before copying it, in deterministic tree order; collisions receive
`-2`, `-3`, and so on globally. Fragment-only ordinary links whose decoded
fragment matches a selected heading are rewritten to its mapped host anchor.
Unmatched fragment-only links target the canonical origin page plus the
original fragment, not the host page.

## Origin-aware path rewriting

Assembly rebases relative ordinary-link destinations and image sources from
origin directory to host directory using lexical POSIX dot-segment
normalization. The canonical origin URL is its percent-encoded page ID plus
`.md`; the host base is the directory of its encoded page ID. This bounded
operation is not complete URI validation. It:

- preserves scheme URLs, protocol-relative URLs and absolute paths byte for
  byte; it preserves query and fragment suffixes except for the specified
  fragment-only anchor rewrite;
- treats empty and query-only destinations as references to the canonical
  origin page;
- retains `.md` for the existing HTML renderer's `rewrite_href`; core does not
  generate format-specific `.html` or `.json` suffixes;
- percent-encodes known origin and host `PageId` path segments using unreserved
  ASCII plus `/`, while preserving already encoded destination segments;
- requires valid UTF-8 when percent-decoding a fragment for matching; invalid
  fragments remain unmatched;
- percent-encodes generated anchors;
- retains `..` above the logical root instead of silently clamping it;
- preserves a directory trailing slash; and
- prefixes `./` when the computed first relative segment could be interpreted
  as a scheme, such as `a:b.md`.

Expansion and cloning allocate memory, and repeated embeds may multiply output.
No linear bound or measured performance result is claimed.

## CLI compatibility

The CLI retains envelope `schema_version: 1`, existing witness objects,
counts, ordering, and exit semantics. New witness kinds are additive:

```text
broken_transclusion
ambiguous_transclusion
missing_transclusion_anchor
transclusion_cycle
```

Exact objects are `{kind, from, target}` for `broken_transclusion`,
`{kind, from, target, candidates}` for `ambiguous_transclusion`, and
`{kind, from, target, heading}` for `missing_transclusion_anchor`. Candidates
are sorted canonical page-ID strings. Unlike the legacy ambiguous-link output,
the new ambiguity kind retains its candidate list. These objects omit `line`.
`transclusion_cycle` is `{kind, path}`, with closed path strings in `page` or
`page#anchor` form. Legacy witness objects keep every existing key and value.
Existing old markdown and JSON cases are not regraded or regenerated.

## Planned registration and checks

Registration precedes implementation. A separate session authors Markdown,
section, and path fixtures plus independent references for section selection,
region blocking, and path rebasing. Seeded normalized graph cases are
algorithm-level synthetic graphs, not claims that every graph is realizable
Markdown. Exact fixed outputs are required for every case, together with
permutation and differential property checks; every necessarily triggered scope
failure is included. Existing corpus bytes remain unchanged.

Required scenario groups are:

- whole-page expansion and exact parser exclusions;
- tight, loose, nested-list, and quote contexts;
- root-section bounds, nested-heading shells, and list ordinals;
- nested embeds;
- exact, basename, missing, ambiguous, and missing-anchor targets;
- self-cycles, two-region cycles, same-page acyclic regions, and out-of-region
  false-cycle controls;
- multi-cycle SCCs;
- repeated embeds, host-anchor collisions, imported navigation slots, and
  inside/outside fragments; and
- relative paths, queries, schemes, Unicode, colon-like names, and directory
  cases.

The predeclared production spread is `document`, `graph`, `site::assembly`,
`site::build`, the `site` facade, and `app-cli`. Renderer production remains
unchanged; renderer setup tests may migrate only for the added `Assemble`
document-slice argument. Owner-local JSON oracle tests may use `serde_json`
DEV-only metadata with an exact live `core.dev_allow` entry and task
justification; there is no normal core `serde` dependency. Frozen H4 uses its
own fixture rules, so a live dev allow does not alter frozen grades.
