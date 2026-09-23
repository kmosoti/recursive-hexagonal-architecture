# Markdown corpus schema supplement

This directory is an independent, data-only registration for the after-data
extension described by `docs/architecture/markdown-record-shape-amendment.md`.
It does not change corpus findings or grade a production validator.

The frozen old report is `old.json`; the observed new report is `new.json`.
The only registered additions are optional `boolean`
`/cases/*/well_formed` and optional `string` `/product/binary_sha256`.
Objects remain closed: an unknown neighboring case or product key is
`schema.unknown_field`. A present addition with any other JSON type is
`schema.wrong_type`. Digest-format or authenticity checks are outside this
structural supplement.

## Recipe protocol

`CASES.json` contains recipes with an `id`, a baseline filename, JSON-pointer
mutations, and the exact structural codes expected after applying them. A
`set` mutation assigns its `value`; a `remove` mutation deletes its `path`.
Array paths use zero-based literal indices. There is no wildcard mutation
operation.

`historical_remove_both_additions` is the historical-compatibility control.
Its case-field removal expands to these 60 literal paths, followed by removal
of `/product/binary_sha256`. The reports retain their own historical metadata;
this control checks structural acceptance, not byte identity between reports:

```text
/cases/0/well_formed
/cases/1/well_formed
/cases/2/well_formed
/cases/3/well_formed
/cases/4/well_formed
/cases/5/well_formed
/cases/6/well_formed
/cases/7/well_formed
/cases/8/well_formed
/cases/9/well_formed
/cases/10/well_formed
/cases/11/well_formed
/cases/12/well_formed
/cases/13/well_formed
/cases/14/well_formed
/cases/15/well_formed
/cases/16/well_formed
/cases/17/well_formed
/cases/18/well_formed
/cases/19/well_formed
/cases/20/well_formed
/cases/21/well_formed
/cases/22/well_formed
/cases/23/well_formed
/cases/24/well_formed
/cases/25/well_formed
/cases/26/well_formed
/cases/27/well_formed
/cases/28/well_formed
/cases/29/well_formed
/cases/30/well_formed
/cases/31/well_formed
/cases/32/well_formed
/cases/33/well_formed
/cases/34/well_formed
/cases/35/well_formed
/cases/36/well_formed
/cases/37/well_formed
/cases/38/well_formed
/cases/39/well_formed
/cases/40/well_formed
/cases/41/well_formed
/cases/42/well_formed
/cases/43/well_formed
/cases/44/well_formed
/cases/45/well_formed
/cases/46/well_formed
/cases/47/well_formed
/cases/48/well_formed
/cases/49/well_formed
/cases/50/well_formed
/cases/51/well_formed
/cases/52/well_formed
/cases/53/well_formed
/cases/54/well_formed
/cases/55/well_formed
/cases/56/well_formed
/cases/57/well_formed
/cases/58/well_formed
/cases/59/well_formed
```

The expected corpus grade and every historical finding remain unchanged.
