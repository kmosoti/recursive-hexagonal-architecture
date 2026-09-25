# Callouts: contract for CHG-012

Status: written before any fixture or code for CHG-012 (plan W9, feature 4). Decisions: `callouts-gfm-only`, `callouts-html-title`, `callouts-check-surface` in `.rha/tasks/CHG-008-growth.toml`.

## Purpose

A GitHub-flavoured markdown alert (`> [!NOTE]`) is a block quote that the reader should see as a note, tip, important point, warning or caution. rhawiki already recognises the five kinds and keeps the kind on the block quote, but it renders a callout as a plain block quote with a class, so a reader sees no difference. This stage renders callouts as GFM does: a visible title naming the kind, and per-kind styling.

## 1. Recognition (unchanged)

A **callout** is a block quote whose first line is exactly `[!KIND]`, where `KIND` is `NOTE`, `TIP`, `IMPORTANT`, `WARNING` or `CAUTION` in any letter case, as pulldown-cmark 0.13.4 recognises GFM alerts (`Options::ENABLE_GFM`). The document already records it as `Node::BlockQuote { kind: Some(kind), children }`, with the marker line removed. Callouts can nest, sit in list items, and have an empty body.

Everything else stays an ordinary block quote with its text unchanged: a title on the marker line (`> [!NOTE] Title`), any other kind (`> [!info]`), and fold markers (`> [!note]-`). These are Obsidian extensions, not GFM (decision `callouts-gfm-only`).

## 2. HTML rendering

For `BlockQuote { kind: Some(kind), children }` the HTML renderer writes:

```html
<blockquote class="callout KIND">
<p class="callout-title">LABEL</p>
...children...
</blockquote>
```

- `KIND` is the lowercase kind (`note`, `tip`, `important`, `warning`, `caution`), as today.
- `LABEL` is `Note`, `Tip`, `Important`, `Warning` or `Caution`.
- The title paragraph is the first child and appears even when the body is empty.
- Nested callouts each get their own title.

A block quote with `kind: None` renders exactly as before. The stylesheet `assets/style.css` gains rules for `.callout`, `.callout-title` and each of the five kind classes. The styling itself is the presentational part of the renderer; this contract only requires that each selector exists.

## 3. Everything else is unchanged

- **JSON:** the JSON renderer's output is unchanged. Its contract already says no generated callout label is added, and callouts keep `"kind"` on the block quote node. The search text is unchanged, so the label is not searchable.
- **Other stages:** the document tree, site assembly, links, headings, TOC, transclusion, §-references, citations and `rhawiki check` output are unchanged.
- **Other HTML:** a page without a callout renders byte-identical HTML. A page with callouts differs only by the inserted title paragraphs. The stylesheet differs only by added rules.

## 4. Prediction

Production set: `adapter-html` only. The plan predicted document plus adapter-html. The document part is not needed, because Phase 1 already parses the kind (`CalloutKind`, "preserved; rendering is plain in Phase 1").

## 5. Non-goals

- Custom titles, other kinds and fold markers (section 1).
- Icons.
- A callout label in JSON or in search text.
- Callouts as `rhawiki check` witnesses (decision `callouts-check-surface`).
