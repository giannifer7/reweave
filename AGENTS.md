# Agent Instructions

This repository is Reweave, a simplified forward-only extraction from
Weaveback. Treat it as a normal Rust project, not as a literate source tree.

## Project Intent

Reweave keeps the valuable parts of Weaveback:

- strict macro expansion;
- Markdown as the authoring format;
- noweb-style chunk assembly;
- direct generation of `@file` chunks;
- the high-value macro and tangle tests.

Reweave intentionally excludes:

- persistent databases;
- apply-back or reverse editing;
- source-map/query tooling;
- generated-file reconciliation;
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
- Keep coverage at or above 95% line coverage.
- Avoid adding CI, containers, installers, wheels, or release automation unless
  the project scope is deliberately expanded.

## Verification

Run these before committing non-trivial changes:

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo llvm-cov --workspace --summary-only
```

The current coverage target is 95% line coverage.

## Repository Layout

- `reweave-cli`: command-line entry point.
- `reweave-macro`: strict macro evaluator and macro-language tests.
- `reweave-tangle`: noweb-style chunk parser, expander, and file writer.
- `reweave-core`: shared constants and small cross-crate definitions.
- `docs/design.md`: high-level design.
- `docs/literate-programming-lessons.md`: lessons inherited from the Weaveback
  experiment.

## Design Guardrails

- A feature that only helps reverse mapping probably belongs in Weaveback, not
  Reweave.
- A feature that simplifies forward authoring may belong here if it is testable
  without persistent state.
- Macros should make repetition explicit and deterministic, not hide large
  semantic side effects.
- Prefer local reasoning over cleverness. If a source fragment cannot be
  understood without simulating global evaluator state, the design is suspect.
