# Markdown corpus record shape amendment

This is an after-data extension of the CHG-019.1 record schema contract, owned by ledger CHG-019.1-markdown-shape-amendment and the task decision markdown-shape-amendment. The original census, snapshots, 1,497 recipes and grading remain immutable.

The current producer's report evidence/md-corpus/20260923T044415Z-ed79a7a8e0ac.json contains two fields absent from the frozen census: cases[*].well_formed is boolean, and product.binary_sha256 is a string digest. These types are mechanically observed from that report. Both properties are optional so historical reports retain their registered shape. The enclosing objects remain closed to other keys, and existing digest semantics still apply.

Before schema projection code changes, a separate generator registers the new/old snapshots, observed additions, and controls for absent fields, both boolean values, wrong types, and unknown neighboring keys. This is not a claim that the shape preceded the current report data, and it does not change markdown corpus case grading. The original contract remains the base; this document and the separate supplement describe the extension.
