# site: scoped guide

`site` is a composite (BDR-0003). Its children are `assembly` and `build`; `rha-modules.toml` owns their allowed edges.

- `build` may use `assembly`. `assembly` never uses `build`. Neither child reaches into the glue (`crate::port`, `crate::Site`).
- Effects cross the glue's ports only: `OutputSink` and `Clock`. `build::step` stays a pure `(state, observation) -> (state', commands)` transition, and `PageRenderer` is a pure port.
- `cargo xtask architecture` evaluates the declared module directions, and L0 runs that check. Review still covers declared source-extraction holes such as macro expansion and type inference. A change that needs to break one of these rules is a BDR-0003 question, not a refactor.
