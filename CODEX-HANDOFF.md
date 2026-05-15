# Codex Handoff

This file is the starting point for a fresh Codex session in Reweave.

## Current State

Repository:

- Local path: `/home/g4/_prj/reweave`
- GitHub: `https://github.com/giannifer7/reweave`
- Branch: `main`
- Initial commit: `bf795ae Initial Reweave implementation`

Verified before this handoff:

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo llvm-cov --workspace --summary-only
```

Coverage at handoff:

```text
TOTAL lines: 3910
missed lines: 195
line coverage: 95.01%
```

## What Reweave Is

Reweave is not Weaveback renamed. It is a deliberately smaller tool built from
the useful forward path:

```text
Markdown input
  -> optional macro expansion
  -> noweb chunk scan
  -> chunk expansion
  -> direct file writes
```

It is a build-from-source tool for programmers. It is not currently a packaged
product and should not grow distribution machinery by default.

## What Was Removed

The extraction intentionally removed:

- SQLite storage;
- source-map database tables;
- query tools;
- apply-back;
- generated-output protection;
- documentation rendering;
- GitHub workflows;
- Containerfiles;
- Python wheels;
- the old multi-binary Weaveback surface.

The `%here` source-editing macro was also removed. It belonged to the
source-mutation/back-path family, not to a forward-only tangler.

## What Was Kept

The macro evaluator remains substantial because it was one of the parts that
worked:

- `%def` creates constant macro bindings.
- `%redef` creates or replaces explicitly rebindable macro bindings.
- `%[...]` and `%name[...%name]` are verbatim blocks.
- `%{...%}` is an argument block that still expands macros.
- `%match(var, default, regex0, val0, ...)` is lazy and supports captures.
- `%pydef`/Monty is retained as one escape hatch.
- Variables are strict and current-frame scoped.

The tangle engine keeps:

- `@file` chunks;
- accumulated named chunks;
- `@replace`;
- reference indentation;
- `@reversed`, `@compact`, and `@tight`.

## Recent Important Changes

- Reweave was initialized as its own git repository.
- The repository was pushed to `giannifer7/reweave`.
- Coverage was raised to 95.01%.
- The leftover standalone `reweave-macro` binary was removed from the simplified
  scope.
- Parser, evaluator, macro API, CLI, and tangle edge-case tests were added.
- `coverage_nightly` is declared in `reweave-macro/Cargo.toml` so stable builds
  do not warn when coverage-specific cfgs are present.

## Next Good Work Items

1. Read `README.md`, `docs/design.md`, and
   `docs/literate-programming-lessons.md`.
2. Run the verification commands.
3. Review whether the remaining tracing/precise-output internals still belong
   in Reweave. They are useful for tests and optional attribution, but they are
   adjacent to the removed back-path philosophy.
4. Improve user-facing Markdown examples without adding distribution or docs
   rendering machinery.
5. Keep splitting large files only when it improves normal Rust navigation.
   Do not split merely to recreate literate-programming structure.

## Non-Goals For The Next Session

- Do not add apply-back.
- Do not add a database.
- Do not make Reweave dogfood itself.
- Do not reintroduce Weaveback's generated-docs system.
- Do not add CI/release/distribution before the core tool has stabilized.

## If You Are An Agent

Work from `AGENTS.md` first. When in doubt, preserve the forward-only scope and
the 95% coverage floor.
