# Front matter and tags: contract for CHG-013 (stage A)

Status: written before any fixture or code for CHG-013 (plan W9, feature 5). Plan revision 3 (M13) split the feature in two:
- **Stage A (this contract):** front matter and tags on the document.
- **Stage B (a later contract and PR):** the tag index page, which needs site assembly and both renderers.

Decisions `front-matter-first-line`, `front-matter-grammar`, `front-matter-check-surface` and `tags-stage-split` are in `.rha/tasks/CHG-008-growth.toml`.

## Purpose

A page can begin with a YAML-style front-matter block that gives it a title and tags. Today such a block renders as a horizontal rule followed by a heading made of the metadata text (probed 2026-09-25). After this stage the block is metadata:
- it produces no body nodes;
- its title replaces the derived page title;
- its tags are recorded on the document.

## 1. Recognition

A page has **front matter** when both of these hold:
- **Opening:** its first line is exactly `---`. The first line is the start of the file, with no preceding blank line and no byte-order mark. Trailing spaces or tabs on the line are allowed.
- **Closing:** a later line is exactly `---` or `...`, with the same trailing whitespace allowed. The first such line closes the block.

The lines strictly between the two delimiters are the **front-matter lines**; there may be none.

Anything else parses exactly as before this feature:
- a first line `---` with no closing line;
- a `---` block that does not start at line 1;
- a page that starts with a blank line;
- `+++` blocks.

pulldown-cmark's `ENABLE_YAML_STYLE_METADATA_BLOCKS` is **not** used. It also recognises blocks in the middle of a page (probed 2026-09-25: `Intro\n\n---\ntitle: x\n---` gives a metadata block), which would delete content from existing pages.

## 2. Grammar (a restricted subset of YAML; decision `front-matter-grammar`)

Each front-matter line is one of the following.

**Ignored lines:**
- A blank line (only spaces or tabs).
- A comment line: optional spaces, then `#`.

**Key-value line:** `key: value`.
- `key` matches `[A-Za-z_][A-Za-z0-9_-]*` at column 1.
- `value` is the rest of the line after `:` with surrounding spaces and tabs trimmed. It may be empty.

**List item:** `- item`, indented by at least one space. It belongs to the most recent key-value line whose value was empty.

**Recognised keys** (the first occurrence of each key wins; later ones are ignored):
- **`title`**
  - A value wrapped in matching double or single quotes has the quotes removed. No escape processing is done.
  - An empty title, after quote removal, is treated as absent.
- **`tags`** gives the page's tags as an ordered list of strings. Three forms are accepted:
  - a flow list `[a, b, "c d"]`: split on commas, each element trimmed, quotes removed as for `title`;
  - a block list: `tags:` with an empty value, followed by indented `- item` lines;
  - a scalar value: one tag.

  After collecting:
  - each tag is trimmed, and empty tags are dropped;
  - a tag equal to an earlier one ignoring ASCII case is dropped, and the first spelling is kept;
  - the order is the order of first appearance.

**Other keys and their values or list items are ignored.** There is no diagnostic for an unknown key.

**Invalid lines** record `Diagnostic::InvalidFrontMatter { line }` and are otherwise ignored:
- a line that is none of the above, such as a nested map or an unindented `- item`;
- a list item that does not follow an empty-valued key.

`line` is the 1-based source line of the offending front-matter line. Front matter with invalid lines is still front matter, and its valid keys still apply.

## 3. Effect on the document

- **Body:** the front-matter block, from the opening line to the closing line inclusive, produces no body nodes. Everything after the closing line is parsed as the page's markdown, and every line number reported anywhere (headings, links, diagnostics, §-references, citations, reference entries) is still the line number in the source file.
- **Title:** `Document::title` is the front-matter title when present. Otherwise it is derived as before: the first heading, else the page id.
- **Observation surface:** `Document` gains the field `front_matter`:

  ```rust
  pub struct FrontMatter { pub title: Option<String>, pub tags: Vec<String>, pub end_line: usize }
  // end_line: the 1-based line of the closing delimiter.
  ```

  It holds `Option<FrontMatter>`, which is `None` when the page has no front matter.

## 4. Everything else is unchanged

- A page without front matter, including every page listed at the end of section 1, produces exactly the document it produced before this feature: body, headings, links, title, section references, citations and diagnostics.
- A page with front matter differs only as sections 2 and 3 describe.
- No other crate changes: site assembly, both renderers and `rhawiki check` already take the title from `Document::title`, and tags are not rendered in this stage.

## 5. Check surface and non-goals

- **Check surface:** `InvalidFrontMatter` is not a `rhawiki check` witness (decision `front-matter-check-surface`). The check JSON schema stays fixed.
- **Non-goals:**
  - rendering tags, and the tag index page (stage B);
  - YAML beyond section 2 (anchors, nested maps, multi-line strings, escapes);
  - `+++`/TOML front matter;
  - other keys such as `date` and `aliases`.

## 6. Prediction

Production set: **document only**. The plan predicted document plus assembly for the whole feature; that prediction applies to stage B, the tag index.

Tests that build a `Document` literal gain `front_matter: None`, under the rule of decision `grader-compat-anchor`, extended by `front-matter-compat`.
