# Tag index: contract for CHG-013 (stage B)

Status: written before any fixture or code for stage B of W9 feature 5. Stage A ([front-matter-contract.md](front-matter-contract.md)) records tags on the document. This stage lists them on an index page.

Decisions: `tag-index-opt-in`, `tag-index-anchors`, `tag-index-scope` and `tag-index-check-surface`, in `.rha/tasks/CHG-008-growth.toml`.

## Purpose

A reader wants to see every tag and the pages that carry it. A page opts in to being a tag index. Site assembly then appends, to that page, one section per tag, and each section lists links to the tagged pages. The listing is made of ordinary headings, lists and wikilinks, so both renderers, the JSON contract and the build invariant `Inv_K` are unchanged.

## 1. Opting in (document; decision `tag-index-opt-in`)

The front-matter grammar recognises one more key, `index`:
- Its value is a scalar, with quotes removed as for `title`.
- The first occurrence wins among valid lines. An empty value, after quote removal, counts as absent.
- `FrontMatter` gains `pub index: Option<String>`.

A page is a **tag index** when its `front_matter.index` equals `tags` exactly. Other values are recorded and have no effect in this stage. Everything else in the front-matter contract is unchanged; `index` was an unknown key and was ignored before.

## 2. Tags across the site

The **site's tags** are collected from every document's `front_matter.tags`:
- **Identity:** tags are the same when they are equal ignoring ASCII case, the stage A rule.
- **Display spelling:** the spelling on the first page, in page-id order, that carries the tag.
- **Pages of a tag:** every page whose tags include it, in page-id order. An index page that carries the tag is listed too.

## 3. The generated section (site assembly)

For each tag index page, assembly appends the following after its own body, in order:
- **Tag order:** tags are sorted by their ASCII-lowercased display spelling. Ties are broken by display spelling, then byte order.
- **A heading per tag:** `PageNode::Heading { level: 2, slug, children: [Text(display spelling)] }`.
- **A list under each heading:** `PageNode::List { start: None, items }`, with one item per page of the tag. Each item is a single `PageNode::WikiLink` whose link index points at a new entry `LinkTarget::Page { id, anchor: None }`, appended to the page's `links`. The link's children are `[Text(title of that page)]`, where the title is `Document::title`.
- **TOC:** each generated heading is appended to the page's TOC as `TocEntry { level: 2, text: display spelling, anchor: slug }`.

When the site has no tags, nothing is appended. A tag index page renders exactly as a page without `index: tags` would, apart from the generated section.

**Anchors (decision `tag-index-anchors`; handed-forward item 1).** A heading's slug is `tag-` followed by `document::slugify(display spelling)`, or `tag` when that slug is empty. Collisions are resolved in tag order:
- A slug that equals one of the page's own heading slugs, or a slug already given to an earlier tag, gets the suffix `-2`, `-3` and so on: the first unused one, checked against both sets.
- Non-ASCII spellings are handled by `slugify`, as for headings.

## 4. What does not change (M12)

- **Other pages:** every page that is not a tag index produces exactly the page model it produced before this stage. So does every page of a site where no page is a tag index.
- **Rendered output:** apart from the tag index pages themselves (HTML and JSON) and the search index's entries for them, every output file is byte-identical, once the build timestamp is removed.
- **Unchanged components:** document trees (except `front_matter.index`), graph resolution, backlinks (generated links add none), transclusion (the generated section is not part of the document, so it is not transcludable), `Inv_K`, both renderers, the JSON contract and `rhawiki check`.

## 5. Scope, check surface and non-goals

- **Scope (decision `tag-index-scope`; handed-forward item 2):** production changes are `document` (the `index` key) and `site::assembly` (the generated section). That is two crates, within M13. No new page kind, renderer method or build command is added.
- **Check surface (decision `tag-index-check-surface`):** no new `rhawiki check` witness. The check JSON schema stays fixed.
- **Non-goals:**
  - a generated page that no source file backs;
  - showing a page's tags on the page itself;
  - per-tag pages;
  - tag hierarchies;
  - other `index` values;
  - transcluding the generated section.

## 6. Prediction

Production set: **document plus site::assembly**, which is exactly the plan's original prediction for W9 feature 5 ("document + assembly"). Tests that build a `FrontMatter` literal gain `index: None`, under the rule of decision front-matter-compat.
