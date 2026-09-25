# Tag-index site corpus (CHG-013 stage B)

Independent, pre-implementation fixture/reference package for
`docs/architecture/tag-index-contract.md`. `reference.py` implements
contract sections 1-3 (which pages opt in, the site's tags, and the
generated section per tag-index page) from the contract text alone, plus
just enough of the already-implemented stage A front-matter grammar
(`docs/architecture/front-matter-contract.md`, sections 1-2) to read a
page's `title`, `tags` and the new `index` key, since the stage B contract
is defined in terms of those fields. It also re-derives
`document::slugify` (read from `crates/document/src/slug.rs`, an existing
public, documented function) and `document::parse`'s title-derivation
order (front-matter title, else the first level-1 heading, else the page
id's basename, read from `crates/document/src/parse.rs` for that API fact
only). It does not read, import, or model any part of the proposed
`site::assembly` implementation.

Run the self-test:

```sh
python3 -B reference.py
```

Regenerate `CASES.json` and `selftestreport.txt` (only needed after an
intentional corpus change):

```sh
python3 -B reference.py --write
```

## Coverage

27 site cases (`CASES.json`), each a map of relative page path (no `.md`
suffix -- the page id) to markdown source, and, for every page whose
front matter makes it a tag index, the expected generated section (a list
of `{heading_text, slug, links: [{page_id, link_text}]}`, in tag order)
and the expected TOC tail.

- **No tags anywhere, no index page** (`no-tags-no-index-page`).
- **Tags absent, an index page opts in**: an empty generated section
  (`tags-absent-index-page-present-nothing-appended`).
- **Tags exist, no index page opts in**: nothing appended anywhere
  (`tags-present-no-index-page-nothing-appended`).
- **Two index pages**, both getting the identical generated sections
  (`two-index-pages-both-get-sections`).
- **An index page that is itself tagged**, appearing in its own generated
  section (`index-page-itself-tagged`).
- **ASCII-case identity** merging tags spelled differently across pages,
  with display spelling from the first page in page-id order
  (`ascii-case-identity-merges-across-pages`,
  `display-spelling-from-first-page-in-page-id-order` -- the latter
  specifically orders the corpus's own page map so that page-id order and
  "the order pages are listed in this file" disagree, to pin that display
  spelling is chosen by page id, not corpus/file order).
- **Non-ASCII tags**: CJK (`non-ascii-tag-cjk`) and accented Latin
  (`non-ascii-tag-accented-cafe`).
- **Empty-slug tags**: a tag that slugifies to the empty string anchors at
  plain `tag` (`tag-slug-empty-from-symbols-only`); two such tags
  de-duplicate to `tag` and `tag-2`
  (`two-empty-slug-tags-dedup-tag-and-tag-2`).
- **Anchor collisions**: with the index page's own heading slug
  (`tag-slug-collides-with-page-own-heading-slug`, one collision, `-2`);
  between two tags' natural anchors
  (`two-tags-collide-with-each-other-suffix-2`); and a three-way collision
  that must skip both taken slugs and land on the first free one, not
  restart from `-2` on the next tag
  (`three-tags-collide-skip-to-first-unused-suffix-3`).
- **Subdirectory pages**, exercising page-id byte ordering across `/`
  (`pages-in-subdirectories`).
- **`index` recognition edge cases**: quoted `index: "tags"`
  (`index-quoted-value-tags`); an unrelated value `index: other`
  (`index-other-value-no-effect`); an empty value `index:`
  (`index-empty-value-absent`); first-occurrence-wins in both directions
  (`index-first-occurrence-wins-other-beats-later-tags`,
  `index-first-occurrence-wins-tags-beats-later-other`); and
  `index: [tags]`, showing that `index`'s value is a plain scalar, not
  given `tags`'s flow-list treatment
  (`index-value-flow-list-syntax-not-special-cased`).
- **Generated link text is `Document::title`**, under all three of its
  derivation rules: an explicit front-matter title
  (`link-text-uses-front-matter-title`), the first level-1 heading
  (`link-text-uses-derived-first-heading-title`), and the page id's
  basename when neither is present
  (`link-text-falls-back-to-basename-title`).
- **Generated links are appended, not interleaved**: an index page with a
  pre-existing body wikilink, so the grader must confirm the generated
  links occupy new `links` entries after the existing one(s)
  (`existing-body-links-precede-generated-link-indices`).
- **A small tag/page cross product**, pinning ordinary tag ordering
  (`Blue` < `Green` < `Red` by ASCII-lowercased spelling) and each tag's
  own page-id-ordered page list
  (`multiple-tags-multiple-pages-cross-product`).
- **Tags collected regardless of source list form** (stage A's flow-list
  vs. block-list grammar), merged into one tag
  (`tags-collected-regardless-of-flow-or-block-list-form`).

Every case is a clean site: `rhawiki check --format json` is expected to
report zero witnesses on every one of them (no broken links, ambiguous
links, missing anchors, or duplicate slugs). None of this package's front
matter has an invalid line under the stage A grammar; that surface belongs
to `xtask/tests/corpus/front-matter`.

### Deliberate corpus restrictions (not ambiguities -- design choices to
keep this package a narrow, independent oracle)

- Page bodies use only plain ATX headings (`^#{1,6} text$`); no setext
  headings, no inline markup inside heading text. `document::parse`'s
  markdown-parsing surface is exercised by other corpora.
- No page in this corpus gives itself two headings with colliding base
  slugs, so "the page's own heading slugs" is always an unambiguous flat
  set, and this package never has to reproduce `document::parse`'s
  heading self-de-duplication algorithm (an existing, unrelated detail of
  an already-implemented feature).
- Front matter in this corpus is always well-formed under the stage A
  grammar; `InvalidFrontMatter` is stage A's own corpus's surface.

## Contract ambiguities (kept out of the corpus; recorded here as
questions, not answered by invention)

1. **Tag-order tie-break appears unreachable.** Section 3 sorts tags "by
   their ASCII-lowercased display spelling. Ties are broken by display
   spelling, then byte order." But section 2's identity rule already
   groups tags that are "equal ignoring ASCII case" into one entry; two
   *distinct* tag groups therefore always have distinct ASCII-lowercased
   spellings by construction, so no two entries being sorted can ever tie
   on that key. Under what scenario, if any, is the "ties are broken by
   display spelling, then byte order" clause meant to fire? This package
   does not attempt to construct an unreachable case.
2. **What exactly are "the page's own heading slugs"?** Section 3 names
   this set as one collision source for a generated anchor, but does not
   say whether it means the page's *final*, already-de-duplicated heading
   slugs (`document::Heading::slug`, i.e. `-2`/`-3` suffixes already
   applied by `document::parse`) or the pre-de-duplication base slugs
   (`document::Heading::base_slug`). This package assumes the final,
   already-assigned slugs, and avoids any page whose own headings collide
   with each other, so the two readings cannot be distinguished by this
   corpus.
3. **Collision set may be narrower than the rendered id space.** Section
   3 names only "the page's own heading slugs" and "a slug already given
   to an earlier tag" as collision sources. It does not mention other id
   sources the HTML renderer is known to emit on the same page -- for
   example a citation entry's `<a id="ref-...">` anchor
   (`docs/architecture/citation-contract.md`-adjacent behavior visible in
   `citations_render.rs`) or a transclusion-injected anchor id. Could a
   generated `<h2 id="tag-x">` collide with such an anchor and produce a
   duplicate HTML `id` that this contract's own de-duplication rule does
   not guard against? This package does not construct a page mixing
   citation/transclusion anchors with a colliding tag slug; that is a
   targeted probe better suited to review than to a baseline corpus.
4. **`index:` followed by indented list items.** Stage A's grammar lets a
   list attach to "the most recent key-value line whose value was empty."
   Nothing in the tag-index contract says whether `index:` (an empty
   value) followed by `- tags` list items has any special meaning; by the
   general grammar rule such items would simply be ignored (as they are
   for any other non-`tags` key with an empty value), leaving `index`
   absent. This package does not add a dedicated case for this, since it
   would only re-exercise stage A's already-covered "list item under a
   non-`tags` empty-valued key" surface with a different key name.

## Package layout

`SHA256SUMS` covers every payload file in sorted relative-path order,
excluding `SHA256SUMS` and `registration.toml` itself. `source-snapshots/`
holds the frozen contract text and this package's generation prompt,
verbatim.
