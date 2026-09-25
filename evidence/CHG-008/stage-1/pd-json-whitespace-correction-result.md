Blocked: the required root-authored archive is absent:

`xtask/tests/corpus/write-prompts/CHG-008/pd-json-generator-whitespace-correction-prompt.md`

I did not fabricate it or apply the normalization repair. Package consistency was restored; `CASES.json` remains unchanged:

`fb7b1f6f9bf28d6e7eda3cc0d17b2613c6ba0070166468df89d05fc43059e854`

Observed checks passed:

- 79 cases; 70 project cases
- reference self-tests
- package reproduction
- SHA256SUMS verification
- first-generation identity verification

Please create the specified prompt archive, then this repair can be completed and registered.