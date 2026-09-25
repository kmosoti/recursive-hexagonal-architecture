# Citations ([Rn]): contract for CHG-011

Status: written before any fixture or code for CHG-011 (plan W9, feature 3). Decisions: `citations-anchor-node`, `citations-scope`, `citations-check-surface` in `.rha/tasks/CHG-008-growth.toml`.

## Purpose

The spec cites its references as `[R57]`, and lists them under `# References` as paragraphs that begin with their label: `[R57] … (2019). …`. rhawiki links each citation to its entry on the same page.

## 1. Reference entries

A **reference entry** is a paragraph whose text begins with a label `[R<digits>]`, optionally with one lowercase ASCII letter after the digits (`[R12a]`), followed by a space or the end of the paragraph. The label is the text between the brackets (`R57`). The paragraph can be at any depth: top level, in a list item, or in a block quote.

The entry's **anchor** is `ref-` followed by the label in lowercase: `R57` gives `ref-r57`. The first entry with a given label is that label's entry. A later paragraph with the same label records `Diagnostic::DuplicateReferenceEntry { label, first_line, second_line }` and is not an entry; its label then counts as a citation (section 2).

**Rendering:** the entry paragraph gains `Node::Anchor { id }` as its first inline node, carrying the anchor. The paragraph's text is otherwise unchanged; the leading `[R57]` stays text.

## 2. Citations

A **citation** is `[R<digits>]`, optionally with one lowercase letter, in text content, other than the leading label of a reference entry. Citations are recognized where §-references are (the section-reference contract, section 2): in paragraphs, list items, table cells and block quotes at any depth. They are never recognized in headings, inline code, code blocks, raw HTML, the text or destination of a markdown link or a wikilink, a transclusion, or an image's alt text.

A citation whose label has an entry on the same page **resolves**. It is rendered as `Node::Link { href: "#ref-<label lowercase>", children: [Text("[R57]")] }`: the link text is exactly the citation's characters. A citation without an entry stays text, unchanged. If the page has at least one reference entry, the page also records `Diagnostic::UnresolvedCitation { label, line }`. If it has none, nothing is recorded, because the references belong to another document.

A range written as two citations, like `[R1]-[R72]`, is two citations with the text `-` between them. No other list form is recognized: `[R1, R2]` is not a citation.

## 3. Observation surface

```rust
pub struct ReferenceEntry { pub label: String, pub anchor: String, pub line: usize }
pub struct Citation { pub label: String, pub target: Option<String>, pub line: usize }
// target: the entry's anchor when resolved, else None; line: the line of the citation's `[`.
```

`Document` gains `reference_entries: Vec<ReferenceEntry>` and `citations: Vec<Citation>`, both in document order.

## 4. The anchor node through the pipeline

`Node::Anchor { id }` is a new inline node with no content. Each stage handles it:

- `site` assembly carries it into the page tree as `PageNode::Anchor { id }`, except inside transcluded content. There anchors are dropped: the source page owns them, and citations in transcluded content already point at the source page (the transclusion contract's link rewriting). A host page therefore never has two elements with one id because of transclusion.
- The HTML renderer writes `<a id="<id>"></a>`, with the id escaped like every attribute.
- The JSON renderer writes `{"type": "anchor", "id": <id>}` and adds no text to the page's search text.

`docs/architecture/json-renderer-contract.md` is amended to list the new node type.

## 5. Non-goals and invariants

- **Out of scope:** cross-page citations, list forms (`[R1, R2]`), and surfacing the new diagnostics in `rhawiki check`, whose schema stays fixed (decision `citations-check-surface`).
- **Nothing else changes:** headings, slugs, the TOC, wikilinks, transclusion, §-references, other links and `rhawiki check` output. A page with no citation and no reference entry produces exactly the tree it produced before this feature.
- **Invariants:**
  - every resolved `Citation::target` is the anchor of exactly one `Node::Anchor` on the page;
  - every `Node::Anchor` belongs to a reference entry;
  - anchor ids are unique on a page.
- **Known limitation:** an entry anchor (`ref-r57`) and a heading slug may coincide only if a heading's text slugifies to `ref-r57`. The page then has two elements with one id. This is not detected in this stage.
- **The real spec** (`docs/spec/rha-spec-v0.10.md`): a pre-registration count found 168 entries, no duplicate label, and no unresolved citation label.
