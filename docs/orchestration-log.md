# Orchestration log

## Atom #4
- Tier used: session coordinator for the sole-writer branch; gpt-5.6-sol max for read-only contract and implementation verification. Decided writes stayed with the coordinator because Kennedy explicitly required one writer.
- Reasoning that did not change the outcome: repeated searches for a Git-compatible JJ workspace mode; checking the installed command's workspace limitation first would have avoided the detour.
- Errors caught: `jj git colocation enable` cannot operate on a non-main workspace; the command helper accepts UTF-8 strings rather than `OsStr`; literal-edit scripts failed before writing. Compiler and assertion exit codes identified each failure. JJ metadata must be locally excluded from the Git companion, and its HEAD/index must mirror the committed JJ candidate before evidence capture.
- Next rule: inspect the local API before calling it, use patch hunks for decided edits, and verify producer/subject identity before the expensive lane. Negative-test each new grading gate with a nearby valid control. Keep unmatched findings in the headline total as well as per-case evidence.
- Publication lesson: verify JJ author and committer against the repository no-reply identity before the first commit; a GitHub email-privacy rejection requires rewritten identities and new evidence, not an authorization retry.
