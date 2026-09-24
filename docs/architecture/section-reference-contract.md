# Section references (§n.n): contract for CHG-010

Status: written before any fixture or code for CHG-010 (plan W9, feature 2). Decisions: `section-refs-scope`, `section-refs-observation`, `section-refs-check-surface` in `.rha/tasks/CHG-008-growth.toml`.

## Purpose

A page that numbers its sections, like the spec, refers to them as `§11.7.6`. rhawiki turns each such reference into a link to the heading it names, on the same page. The plan predicts that this touches the `document` crate only.

## 1. Section numbers of headings

A heading has a section number when its text (`Heading::text`) begins with one of these forms, followed by whitespace:

- `N(.N)*`, optionally followed by one `.`: `1. Problem, Scope…` has number `1`; `1.1 Target class…` has `1.1`; `11.7.6 Acceptance…` has `11.7.6`. Here `N` is one or more ASCII digits.
- `Appendix L(.N)*`, optionally followed by one `.`: `Appendix A. Mathematical…` has number `A`. Here `L` is one ASCII uppercase letter.

The heading text may also end right after the number, which then has no trailing whitespace. Numbers are compared as exact strings: `1.10` and `1.1` differ, and `01` and `1` differ. When several headings carry the same number, the first in document order is the target.

## 2. Recognizing a reference

References are recognized only in text content. That means paragraphs, list items, table cells and block quotes, at any depth. They are never recognized in headings, inline code, code blocks, raw HTML, the text or destination of a markdown link or a wikilink, a transclusion, or an image's alt text.

A reference is `§` or `§§`, then at most one space, then a number: `N(.N)*` or `L(.N)*`. An `L` counts only when the character after it is not a letter. The number is the longest match: a `.` belongs to it only when a digit follows. So in `see §7.` the number is `7`, and the final `.` is sentence punctuation.

A **range continuation** directly after a number is optional whitespace, then `–` (U+2013) or `-`, then optional whitespace, then an optional `§`, then a number. It makes that number a second reference, the range end. So `§§3–7`, `§3–§5` and `§§8.3–8.4` each contain two references. A comma list after `§§`, such as `§§8.3, 8.4`, gives only the first reference; this is a known limitation. Every other occurrence of `§` needs its own `§`, so `§6 and §12.1` gives two references.

## 3. Resolution and rendering

A reference whose number equals the number of a heading on the same page (section 1) **resolves** to that heading. It is rendered as a markdown-style link (`Node::Link`) to `#<slug>`, where the slug is that heading's unique slug (`Heading::slug`). The link's visible text is exactly the characters the reference occupies, taken from the characters that were written:

- `§4.1` becomes one link with text `§4.1`;
- `§§3–7` becomes a link with text `§§3`, then the text `–`, then a link with text `7`;
- `§3–§5` becomes a link `§3`, the text `–`, then a link `§5`.

Surrounding text is kept byte for byte.

A reference that does not resolve stays text, unchanged. If the page has at least one numbered heading, the page also records `Diagnostic::UnresolvedSectionRef { number, line }`. If the page has none, the reference is neither linked nor diagnosed, because the section numbers belong to another document. Cross-page references are out of scope (section 5).

## 4. Observation surface

`Document` gains `section_refs: Vec<SectionRef>`, one entry per recognized reference in document order:

```rust
pub struct SectionRef {
    pub text: String,           // the characters rendered as the link text (or left as text)
    pub number: String,         // the section number referred to
    pub target: Option<String>, // the target heading's slug when resolved, else None
    pub line: usize,            // 1-based source line of the reference
}
```

The registered grader compares this list, and the diagnostics, with the registered expectations.

## 5. Non-goals and invariants

- **Out of scope:** cross-page references (for example `index.md` citing a spec section), comma lists after `§§`, and surfacing `UnresolvedSectionRef` in `rhawiki check`, whose JSON schema stays fixed (decision `section-refs-check-surface`).
- **Nothing else changes:** headings, slugs, the TOC, wikilinks, transclusion, other links and `rhawiki check` output.
- **Invariant:** every resolved `SectionRef::target` is the slug of a heading on the same page, and the corresponding `Node::Link` has `href == "#" + target`.
- **The real spec** (`docs/spec/rha-spec-v0.10.md`) records no `UnresolvedSectionRef`: every reference in its text content names one of its own numbered headings. A rough pre-registration count found 163 numbered headings and no unresolved number among 366 `§` occurrences, including occurrences in code and headings, which are not references.
