# BDR-0007: `site::assembly` owns the renderable page tree

> **Status: decided** by `agent:executor` under `human:kennedy`'s delegation
> on 2026-09-23. This accepts the refutation criterion; it is not a PR or
> artifact acceptance. No new crate is proposed.

```text
Boundary: site::assembly  (PageNode and transclusion expansion)
Level:    module facade within site

Decision:
  site::assembly owns PageNode, using the existing renderable variants and
  fields of document::Node. site exports PageNode as site::Node. PageModel.body
  is Vec<PageNode>. document owns parser syntax, including Transclusion, but
  no parser-only variant reaches a renderer.
```

The current public model has `document::Document` with `body: Vec<Node>`,
navigation-only `links`, headings, and diagnostics. `site::assembly` currently
projects that document into `PageModel`, whose body is consumed by the
`PageRenderer` port. `graph` owns `Resolved`, navigation witnesses, backlinks,
and resolution lookup. This decision keeps the renderer-facing source shape
stable while moving the final semantic boundary to `site::assembly`.

## Delegation

The task instructions are quoted in full:

> “Determine a large slice to hand off to gpt-6-astra ultra. Make sure it has
> a goal to implement its work and fix any and all issue in accordance with
> rha design. (Let it flag and improve the architecture and tooling if it can
> justify it no need for my approval). I approve all PRs as long as both an
> opus 5.5 and gpt-6 agent has reviewed and approved the code.”

> “Use recommended. Shift using codex to validate that the deterministic schema
> is used according to rha guidelines rather than regenerating code as thats
> the contract that should determine the qiality of the code. Reviee the PR
> codex reviews. Accept the recommend and most correct implementations going
> forward instead of escalating to me.”

Only the refutation criterion below is accepted under that delegation. This
document does not accept a PR, claim implementation evidence, or change the
method/spec authority.

## Owned page vocabulary

`PageNode` is defined at the `site::assembly` facade and retains the existing
renderable variants and fields:

```text
Heading { level, slug, children }
Paragraph(children)
Text(text)
Code(text)
CodeBlock { lang, text }
Emphasis(children)
Strong(children)
Strikethrough(children)
Link { href, children }
WikiLink { index, children }
Image { src, alt }
List { start, items }
BlockQuote { kind, children }
Table { align, head, rows }
Rule
SoftBreak
HardBreak
Html(text)
TaskMarker(checked)
```

`Align` and `CalloutKind` may remain shared semantic enums. The crate root
exports `PageNode` as `site::Node`, preserving the existing renderer import
shape. `PageModel.body` is `Vec<PageNode>`. `PageNode` has no `Transclusion`
variant.

`document::Node` remains the parser-owned tree and gains the owned
`Transclusion { id, target, anchor, display, line }` representation described
by the transclusion contract. Assembly alone converts parser syntax and
removes transclusions. There is no blanket `From<document::Node>`:
conversion depends on origin and host pages, graph results, occurrence identity,
heading allocation, and expansion context.

The existing HTML and JSON production renderers remain unchanged. Their setup
tests may need the explicit document-slice argument added to `Assemble`; those
call-site migrations are recorded separately and are not hidden as renderer
changes. `site::build` passes its existing complete document slice downward;
assembly never reaches build or glue.

## Alternatives

Scores are out of five for ownership, syntax cannot leak, migration
compatibility, W9 extensibility, and diagnosis respectively.

| Alternative | Ownership | Syntax cannot leak | Migration | W9 extensibility | Diagnosis | Total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Shared document AST | 2 | 1 | 5 | 2 | 3 | 13/25 |
| **Owned `PageNode`** | **5** | **5** | **4** | **5** | **5** | **24/25** |
| Flat render-event stream | 4 | 5 | 1 | 4 | 2 | 16/25 |

Sharing the document AST is cheap to migrate and avoids duplicate vocabulary,
but it makes parser semantics part of the renderer contract: a transclusion,
section-reference, or later parser-only node must be understood by every
consumer. It also leaves origin-aware conversion and diagnostics without a
clear owner.

An owned page tree duplicates the vocabulary at the semantic boundary and
requires explicit conversion. That cost is genuine: conversion must preserve
renderer fields while resolving links, importing headings, and rebasing paths.
It buys a stable renderer model, keeps parser syntax private, gives assembly a
place to diagnose expansion failures, and leaves later W9 features additive.

A flat render-event stream isolates syntax well, but its event balancing and
rewrites make existing renderer migration substantially larger. Nested imports,
heading allocation, link rewriting, and partial section selection would need
event buffering or repair passes, weakening diagnosis and source ownership.

## Refutation criterion

The accepted criterion is:

```text
metric:
  During the transclusion and section-reference stages, renderer production
  files require zero edits solely because of new parser-only semantic variants.

source:
  Exact source-path diffs for those stages, with every renderer-file change
  classified as solely parser-variant-driven or as a justified exception.

action:
  Reopen the site::assembly ownership boundary if the zero-edit condition is
  violated without a justified exception.
```

This does not prohibit later presentation capabilities such as citation
anchors. No implementation result, hash, count, or PR acceptance is asserted.

