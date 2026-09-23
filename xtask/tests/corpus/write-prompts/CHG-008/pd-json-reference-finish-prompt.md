Continue the independent reference repair. This is a correction of your failed attempt, not a missing user input. You invented an extra "generator-" segment in the archive filename. The original prompt supplied the correct path, and that file exists. Do not create an alias at your invented path.

Read this EXACT file first, using its absolute path:
/home/kmosoti/projects/rha-m2/xtask/tests/corpus/write-prompts/CHG-008/pd-json-whitespace-correction-prompt.md

The task in those bytes remains authorized: replace Python split with the explicit Unicode White_Space normalization, add the hand-derived control-character checks, and preserve the exact 79-case CASES.json bytes. Copy that exact existing prompt to source-snapshots/whitespace-correction-prompt.md. Register its original correct logical path and computed digest. No adapter implementation exists or has run.

Also repair the reference README command example. ALLOWED_OUTPUT_ROOT is PACKAGE_DIR, so users must first COPY the committed package to an owned staging directory, then run the copied script. Document from the repository root:
cp -R xtask/tests/corpus/json-renderer target/m2/json-renderer-reproduction
python3 target/m2/json-renderer-reproduction/reference.py --reproduce-to target/m2/json-renderer-reproduction/reproduced
Explain that the staging destination must be fresh, and the reproduction subdirectory may be removed afterward. Self-tests from the committed script are read-only. Keep the script's explicit destination guard; no live-source or repository-root assumptions. Update the _readme template as well as generated README. Include all three prompt snapshots and say three, not both.

Your writes remain restricted to /home/kmosoti/projects/rha-m2/target/m2/json-renderer-corpus. Run your own reproduction inside that allowed package directory, then remove only the subdirectory you created. All source snapshots and prompt bytes are permitted reads; this follow-up is at /home/kmosoti/projects/rha-m2/xtask/tests/corpus/write-prompts/CHG-008/pd-json-reference-finish-prompt.md if you record it (copy it too and adjust the prompt count if so).

Never run JJ, including status or init. No Git mutations, agents, network, sibling projects, held-out material, spec edits, or implementation reads. Keep the data bytes fixed; recompute metadata from files after the code/docs repair, and run self-tests, reproduction, digest verification and the first-generation CASES identity check. Report actual results. Do not stop to ask for a file whose exact path you have not checked.
