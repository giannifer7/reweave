# Agent Instructions

This repository is Reweave, a forward-only literate-source tool.
Treat it as a normal Rust project, not as a literate source tree.

## Project Intent

Reweave deliberately keeps only the forward path of literate programming:

- strict macro expansion;
- Markdown as the authoring format;
- noweb-style chunk assembly;
- direct, content-aware generation of `@file` chunks;
- exact `file:line:col` positions in every error message;
- high-value macro and tangle tests.

Reweave intentionally excludes:

- persistent databases;
- reverse editing or provenance mapping;
- generated-file reconciliation or edit protection;
- documentation rendering pipelines;
- release/distribution machinery.

Do not reintroduce those removed systems casually. If a feature requires a
database, reverse provenance, or generated-file protection, first question
whether it belongs in Reweave at all.

## Working Rules

- Prefer ordinary Rust and Markdown.
- Do not dogfood Reweave to generate its own source.
- Keep the forward path simple: macro expansion, chunk scan, chunk expansion,
  direct writes.
- Preserve strict macro semantics unless a change is explicitly requested.
- Add tests for every macro-language or tangle behavior change.
- Keep coverage at or above 99.5% line coverage.
- Avoid adding CI, containers, installers, wheels, or release automation unless
  the project scope is deliberately expanded.

## Verification

Run these before committing non-trivial changes:

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
scripts/check-coverage.sh
```

The current coverage target is 99.5% line coverage. The coverage check is
deliberately fail-fast through `cargo llvm-cov --fail-under-lines`.

## Repository Layout

- `reweave-cli`: command-line entry point.
- `reweave-macro`: strict macro evaluator and macro-language tests.
- `reweave-tangle`: noweb-style chunk parser, expander, and file writer.
- `reweave-core`: shared constants and small cross-crate definitions.
- `docs/design.md`: high-level design.
- `docs/literate-programming-lessons.md`: lessons on forward-only literate
  programming.

## Working on the Code

### Evaluation is single-path

There is exactly one evaluation engine (`Evaluator::evaluate` /
`evaluate_macro_call`). There used to be a parallel tracing engine
(`evaluate_to` / `*_to` pairs); it was deleted. Never add a second evaluation
path — if a feature needs output metadata, attach it to the plain path.

### Error locations

Every `EvalError` carries a `SourceLocation` (structured slot or, for
string-payload variants, via `ensure_location` at the macro-call frame). When
you add a new error variant, give it an `Option<SourceLocation>` slot and a
location-first `#[error]` template, and attach positions at raise sites via
`self.node_location(node)`.

### Adding a builtin

Builtins live in `reweave-macro/src/evaluator/builtins/` grouped by topic
(`control.rs`, `definition.rs`, `scope.rs`, `strings.rs`, …) and are registered
in `builtins.rs::default_builtins()`. A builtin is
`fn(&mut Evaluator, &ASTNode) -> EvalResult<String>`: evaluate arguments with
`eval.evaluate(part)`, and raise `InvalidUsage` on arity/type errors (the
call-site location is attached automatically at the macro-call frame). Add
tests under `reweave-macro/src/evaluator/tests/` in the matching `test_*.rs`
file (or a new one, registered in `tests/mod.rs`).

### Dependencies

- `monty` (the `%pydef` evaluator) is a git dependency pinned by tag. After any
  `cargo update`, re-run
  `cargo update -p get-size2 --precise 0.10.0` — newer `get-size2` breaks
  monty's `ruff_python_ast` (see the note in the workspace `Cargo.toml`).
- Do not add dependencies for conveniences the stdlib covers. The current set
  is deliberately small.

## Design Guardrails

- A feature that only helps reverse mapping does not belong here.
- A feature that simplifies forward authoring may belong here if it is testable
  without persistent state.
- Macros should make repetition explicit and deterministic, not hide large
  semantic side effects.
- Prefer local reasoning over cleverness. If a source fragment cannot be
  understood without simulating global evaluator state, the design is suspect.
