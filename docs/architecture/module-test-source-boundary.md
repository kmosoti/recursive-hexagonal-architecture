# Test source boundary for workspace module checking

This additive checker contract is registered before its implementation. The original module-check contract and all M, L-M, EM-M and X-M01 expectations remain unchanged.

The ordinary extractor entry point and standalone module-check mode continue to accept module files only within the canonical crate root. Full-workspace checking may additionally supply the canonical Cargo workspace root as an explicit permitted test-source root.

Before reading or parsing any module file, canonicalize its path. Permit it exactly when it is within the crate root, or when its inherited module context is test-only and it is within the explicit permitted test-source root. An omitted test root grants no extra access. The canonical crate root must itself be inside the configured test root. The root module remains crate-confined. Production sources outside the crate remain refused. Symlink targets outside both permitted boundaries remain refused; a textual path prefix is insufficient.

Test classification remains inherited through nested modules. Only exact declaration ancestry `#[cfg(test)]` grants the wider boundary. An external file’s inner attribute cannot grant permission to read itself; non-exact conditions such as `cfg(any(test, feature = "x"))` and `cfg_attr` remain limitations and grant no wider read root. Logical Rust module names and nearest declared component ownership remain authoritative; sharing a physical helper file does not change ownership. Continue extracting and separately reporting test edges, including M11. Do not skip test modules or turn the shared helper into an opaque include macro.

The source-digest collector uses the same explicit read boundary and binds every actual traversed file, including a workspace-contained test helper outside the crate. Full-workspace architecture reports must therefore change the helper's source digest when its bytes change. No sibling project is an authorized source root for this task.

New independent controls must distinguish default refusal, explicit test-only allowance, production refusal, nested inherited test context, canonical symlink confinement, and complete helper digest provenance. Existing registered graphs retain their grades. The real site module check must run and pass after the repair.

Boundary diagnostics distinguish the failed authority: default or production module escapes contain `outside crate root`; an inherited test-only target beyond an explicitly configured test root contains `outside permitted test source root`. A configured test root that excludes the crate itself reports the invalid crate-root relationship. These distinctions let the registered negatives reject an incorrect classification as well as a parse failure.
