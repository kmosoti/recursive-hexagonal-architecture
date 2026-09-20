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
- Producer identity belongs to the compiled checker, never to the subject
  workspace. Keep unknown or dirty revision identity honest in reports.

## Tests and changes

Pair synthetic rule tests with a small real `cargo metadata` to CLI/JSON
regression and a valid control. Assert nested contract fields, including
producer and subject identities, rather than only top-level success.

Fix a fixture that misses the intended failure; do not reverse its expected
result to match current output.

Workers use `cargo test -p xtask --locked` for targeted checks. The coordinator
owns the full `cargo xtask ci` run and landing verification. Test changes and
new dependencies need the justification in `CONTRIBUTING.md`.
