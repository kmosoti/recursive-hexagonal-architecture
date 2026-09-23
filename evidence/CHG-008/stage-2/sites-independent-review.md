# Independent review of the original transclusion-sites draft

Scope: the original `CASES.json` and `reference.py` read before the concurrent
correction pass. The review is against `docs/architecture/transclusion-contract.md`.
No target implementation or held-out material was used.

## Required pre-registration corrections

1. **TX01 has no recognized transclusion.** The lines `before`, both WikiLink
   images, and `after` form one Markdown paragraph because no blank lines
   separate them. Both candidates therefore have meaningful siblings and must
   remain `Image`. The expected two descriptors and both expansions are
   impossible. Separate the blocks, for example:

   ```markdown
   # Host
   before

   ![[tx01/target]]

   after

     ![[tx01/target#part]]  
   ```

   The descriptor source lines then become 4 and 8.

2. **TX02 correctly has no transclusions, but its page expectations import
   content anyway.** Remove `page body` and `other body` from `text_includes`
   and add them to `text_excludes`. Insert a blank line before the four-space
   indented control; without it, that line can be a lazy continuation of the
   preceding paragraph instead of the intended indented code block. Escaped
   and spaced forms remain the two navigation links; the inline, alias, and
   two soft-break forms remain the four images.

3. **TX06's nested embed is inline, not standalone.** In `tx06/b`, `before`,
   `![[tx06/c]]`, and `after` are one paragraph. Add blank lines around the
   embed. The corrected nested descriptor is on line 5. Without this repair,
   A expands B's section but never expands C.

4. **TX07 has no recognized transclusions.** All five image forms are in one
   paragraph separated by soft breaks, so none can trigger the three expected
   witnesses. Put blank lines between them and update descriptor lines to
   2, 4, 6, 8, and 10. The registered witnesses are then necessarily exactly
   one broken target, one ambiguity with the two sorted `common` candidates,
   and one missing anchor.

5. **TX09's blocked self-edge cannot import C before stopping.** The occurrence
   `(tx09/c, id 0)` is blocked immediately. The C page therefore has heading
   IDs `['c']`, not `['c', 'tx-0-c']`; it contains only the cycle marker after
   the host heading.

6. **TX10's expected graph is absent under the authored Markdown.** A's two
   embeds share a paragraph, as do C's two embeds. Add blank lines between
   each pair. Corrected descriptor lines are A: 2 and 4; B: 3; C: 3 and 5.
   After that correction, A's second, nonblocked root occurrence also expands
   C, so `heading_ids` must additionally contain `tx-1-one`. Both blocked C
   occurrences render again under that second expansion; substring checks may
   remain non-counting, but the heading observation cannot omit the expansion.

7. **TX11's two embeds share one paragraph.** Insert a blank line between
   them and change the second descriptor line from 4 to 5. The unmatched
   `#unknown` link is rebased from source to host in the same `tx11` directory,
   so both expected hrefs are `source.md#unknown`, not
   `tx11/source.md#unknown`.

8. **TX12 rebases query-only and empty destinations incorrectly.** Host and
   origin share `tx12/é:dir/`; the relative canonical origin is therefore
   `origin.md`. Expected hrefs must use `origin.md?q=1` and `origin.md`, with
   HTML markers `origin.html?q=1` and `origin.html`. The expected document for
   `tx12/é:dir/origin` must also list its two ordinary images,
   `../img.png`/`relimg` and `./a:b.png`/`schemeimg`.

9. **TX12 does not actually cover a scheme URL, and the corrected same-directory
   paths no longer observe Unicode PageId encoding.** `./a:b.md` is explicitly
   a relative colon-like path, while `//cdn...` is protocol-relative. Add a
   true scheme link such as `https://example.com/a.md?x=1#f` and require it
   byte-for-byte unchanged. Add a host outside `é:dir` (or a separate fixed
   subcase) so canonical-origin rebasing visibly produces
   `%C3%A9%3Adir/origin.md`; retain a colon-like relative destination whose
   first computed segment requires the `./` guard. Otherwise two required
   origin-rewrite clauses have no fixture observation.

## Cases with no correction found in this pass

TX03's tight/loose/list/quote structure, TX04's root section boundary, TX05's
ordered-list and quote shells, and TX08's region cycles are consistent with the
fixed contract. The remaining TX09 cycle is global but correctly absent from
A's selected output. TX11's anchor allocation and imported navigation-slot
append order are correct after its paragraph/path repairs.

## Post-correction audit

The first correction pass resolved findings 1–8 above: paragraph boundaries,
descriptor lines, suppressed expansion observations, cycle heading IDs,
relative source hrefs, and ordinary-image document expectations now agree with
the contract.

The subsequent TX12 coverage pass added a true scheme URL and an outer host,
but its outer-host canonical-origin expectation retains one extra root segment.
For host `tx12/outer`, the base directory is `tx12/`; origin
`tx12/%C3%A9%3Adir/origin.md` must therefore be emitted relative as
`%C3%A9%3Adir/origin.md` (and the corresponding `?q=1` form), not
`tx12/%C3%A9%3Adir/origin.md`. Remove that prefix from both PageNode hrefs and
HTML marker expectations. The neighboring expected values `guide.md` and
`%C3%A9%3Adir/a:b.md` already apply the same base correctly.
