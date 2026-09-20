# xtask contributor guide

`xtask` is the repository's checker and evidence producer. Keep its contracts
explicit at every boundary, from Cargo metadata and rules input to exit status
and JSON output.

## Input and identity rules

- An absent optional input may use its documented default. Reject unsupported
  schema versions and invalid policy values. For unimplemented modes, follow
  the declared refusal or explicit not-run/limitation contract; never imply
  they were evaluated.
- A Cargo transport or output failure is a `tool_error`; invalid input or rules
  configuration is a `config_error`. Preserve that distinction through the CLI
  and evidence envelope.
- Use package membership and source identity, preserving it through
  classification and every decision. Inspect all
  matching edges before denying an ordinary dependency; result order must not
  decide the outcome.
- Producer identity is the tool's own checkout, `HEAD` and working-tree
  dirtiness read at run time by the one helper `xtask ci` also uses, never
  the subject workspace. Unknown or dirty identity is reported as such.

## Tests and changes

Pair synthetic rule tests with a small real `cargo metadata` to CLI/JSON
regression and a valid control. Assert nested contract fields, including
producer and subject identities, rather than only top-level success.

Fix a fixture that misses the intended failure; do not reverse its expected
result to match current output.

Targeted checks: `cargo test -p xtask --locked`. The full lane, `cargo xtask
ci`, runs once on the settled candidate. Test changes and new dependencies
need the justification in `CONTRIBUTING.md`.
