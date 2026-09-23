# JSON renderer contract v1

This is the normative contract for the planned `adapter-json` crate. It is
written before implementation so an independent writer can register cases and
goldens from it. No implementation result is asserted here. Required state
tests are not passed by this proposal.

The adapter implements `site::PageRenderer`, is constructed only by `app-cli`,
and follows [BDR-0006](../adr/BDR-0006-adapter-json-immutable-index.md).

## Encoding and determinism

Every JSON output is:

- UTF-8;
- deterministic for equal page-model inputs;
- pretty-printed with `\n` line endings;
- terminated by exactly one trailing newline.

Object member order is not graded; decoded member names, types and values are
exact. Repeated outputs must have identical bytes.

Arrays preserve their input order unless this contract explicitly says
otherwise. JSON objects contain exactly the listed members; no implementation
metadata or HTML markup is added.

The page renderer is clock-independent except for the `built_at` value already
present in the supplied `PageModel`. The search index never contains or
depends on `built_at`.

## Page output

`render(page)` writes `<id>.json`. Its root object is:

```json
{
  "schema_version": 1,
  "kind": "rhawiki_page",
  "id": "page-id",
  "title": "Title",
  "toc": [
    {"level": 1, "text": "Heading", "anchor": "heading"}
  ],
  "breadcrumbs": ["section"],
  "backlinks": [
    {"id": "other-page", "title": "Other page"}
  ],
  "links": [
    {"status": "resolved", "page": "target", "anchor": "section"},
    {"status": "unresolved", "page": null, "anchor": null}
  ],
  "body": [],
  "built_at": null
}
```

The `built_at` member is always present and is either a string or `null`.
`toc`, `breadcrumbs`, `backlinks`, `links`, and `body` preserve the
corresponding `PageModel` vector order. A resolved link has a required string
`page` and a nullable string `anchor`. An unresolved link has both `page` and
`anchor` set to `null`.

The `body` array is an exhaustive projection of `site::Node`:

| Node | JSON shape |
| --- | --- |
| `Heading` | `{"type":"heading","level":u8,"anchor":String,"children":[Node]}` |
| `Paragraph` | `{"type":"paragraph","children":[Node]}` |
| `Emphasis` | `{"type":"emphasis","children":[Node]}` |
| `Strong` | `{"type":"strong","children":[Node]}` |
| `Strikethrough` | `{"type":"strikethrough","children":[Node]}` |
| `Text` | `{"type":"text","text":String}` |
| `Code` | `{"type":"code","text":String}` |
| `Html` | `{"type":"html","text":String}` |
| `CodeBlock` | `{"type":"code_block","language":null\|String,"text":String}` |
| `Link` | `{"type":"link","href":String,"children":[Node]}` |
| `WikiLink` | `{"type":"wiki_link","link_index":usize,"children":[Node]}` |
| `Image` | `{"type":"image","src":String,"alt":String}` |
| `List` | `{"type":"list","start":null\|u64,"items":[[Node]]}` |
| `BlockQuote` | `{"type":"block_quote","kind":null\|CalloutKind,"children":[Node]}` |
| `Table` | `{"type":"table","align":[Alignment],"head":[[Node]],"rows":[[[Node]]]}` |
| `TaskMarker` | `{"type":"task_marker","checked":bool}` |
| `Rule` | `{"type":"rule"}` |
| `SoftBreak` | `{"type":"soft_break"}` |
| `HardBreak` | `{"type":"hard_break"}` |

`CalloutKind` is encoded as one of the lowercase strings `note`, `tip`,
`important`, `warning`, or `caution`, or `null`. Alignment is encoded as one of
`none`, `left`, `center`, or `right`. Table vector order and cell vector order
are preserved. `wiki_link.link_index` is the source node's link index; its
resolved target is represented separately in `links`.

Raw HTML remains a JSON string under `{"type":"html","text":...}`. No HTML
rendering, escaping-to-markup, link rewriting, or generated callout label is
performed in JSON. Ordinary link `href` values are copied unchanged.

## Search-index asset

`assets()` includes the deterministic asset `assets/search-index.json`. Its
root object is:

```json
{
  "schema_version": 1,
  "kind": "rhawiki_search_index",
  "entries": [
    {
      "id": "page-id",
      "path": "page-id.json",
      "title": "Title",
      "headings": [
        {"level": 1, "text": "Heading", "anchor": "heading"}
      ],
      "text": "visible body text"
    }
  ]
}
```

Entries are sorted by `id` using the source page-ID ordering. Each `path` is
the page's relative JSON output path, `<id>.json`. `headings` is the page's
TOC projection in its original order. The index excludes `built_at` and
`backlinks`; it contains no clock-derived data.

Search text is collected recursively from visible body content:

- `Text`, `Code`, and `Html` append their text;
- ordinary links and wikilinks append their child labels, not their targets;
- images append `alt`;
- emphasis, strong, and strikethrough recurse without inserting spaces;
- headings, paragraphs, code blocks, list items, quote sections, and table
  cells add whitespace boundaries around their contents;
- list, block-quote, and table nesting preserves the recursive order;
- soft and hard breaks add a whitespace boundary;
- rules add a whitespace boundary; and
- task-marker state is ignored.

Adjacent inline fragments are concatenated without artificial spaces. A
boundary is an internal whitespace separator, not text visible in the final
value. After recursive collection, the result is normalized by
`split_whitespace` followed by joining with the ASCII space character. This
trims leading and trailing whitespace and normalizes Unicode whitespace.
Consequently, adjacent inline text/code/HTML/link-label/image-alt fragments
remain adjacent, while separate block content cannot run together.

## Construction and build lifecycle

`JsonRenderer::new(&[PageModel])` rejects duplicate IDs with
`DuplicatePageId { id }`, sorts the index entries by ID, and precomputes the
asset bytes. The supplied page models are not mutated. A new renderer is
required for every corpus change; reusing one after a page is added, removed,
or changed is invalid.

`app-cli` constructs the renderer from the same corpus that it passes to
`site::build_all`. JSON mode performs the additional pure
`site::analyse`/`DefaultAssembler` pass needed to provide the constructor's
snapshot, then invokes the existing build pipeline. The existing `Inv_K`
check remains the owner of write validity. If a page output collides with
`assets/search-index.json`, the build fails before applying any writes.

## CLI compatibility

- `rhawiki build --format html` is the default and retains the existing HTML
  renderer behavior.
- `rhawiki build --format json` selects `JsonRenderer`.
- `rhawiki check --format json` retains its existing v1 witness schema and
  semantics; this renderer contract does not change it.

## Required pre-implementation cases

Before implementation, an independent writer must register exact cases and
goldens covering:

- every `Node` variant, including all callout kinds and all alignments;
- nested nodes and every table/list shape;
- preservation of page, TOC, backlink, link, table, row, and body ordering;
- input permutations and constructor duplicate-ID rejection;
- Unicode text and Unicode-whitespace normalization;
- unchanged ordinary link `href` values and raw HTML-as-text behavior;
- clock-independent search-index bytes;
- a fresh index and deleted-page behavior across successive builds;
- page/asset collision rejection with no writes;
- the `site::PageRenderer` owner-contract controls;
- exact JSON TOC golden semantics independent of the sampled metamorphic
  TOC-drop guard;
- `build` defaulting to HTML and selecting JSON explicitly; and
- unchanged `check --format json` output compatibility.

Existing older corpora remain frozen and are not regenerated or regraded.
Required state tests are a future implementation gate and have not passed in
this contract-only proposal.
