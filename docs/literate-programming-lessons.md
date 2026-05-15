# Literate Programming Lessons From Weaveback

Reweave exists because the Weaveback experiment produced useful pieces, but the
full system became too complex for its payoff.

This document preserves the hard-earned design knowledge so future work can
reuse the good parts without reconstructing the failed complexity.

## Core Lesson

Forward literate extraction is useful. Bidirectional literate programming is
fragile.

The valuable path is:

```text
human-authored source document
  -> macro expansion
  -> chunk expansion
  -> generated files
```

The costly path was:

```text
generated file edit
  -> provenance lookup
  -> source patch synthesis
  -> literate document rewrite
```

The second path required databases, source maps, lock handling, generated-file
protection, apply-back heuristics, and complex failure modes. It also created a
workflow that was unfamiliar to both humans and coding agents.

Reweave keeps the first path and rejects the second.

## What Worked

### Strict Macros

The macro language became better when it favored explicitness:

- `%def` means stable binding.
- `%redef` means intentional rebinding.
- builtins cannot be overwritten.
- missing variables and parameters are errors.
- variables are current-frame scoped instead of leaking from outer frames.

These rules improve local reasoning. A human or agent can inspect a macro body
without assuming every name might silently come from dynamic outer state.

### Verbatim Blocks

Verbatim blocks solve a real problem at the right layer:

```text
%[literal content%]
%tag[literal content%tag]
```

They avoid special raw variants of individual builtins. This keeps scripting
escape hatches simpler and makes literal opacity lexical rather than
builtin-specific.

### Accumulating Chunks

Accumulating named chunks are useful when they are explicit and predictable:

```text
// <[imports]>=
use std::path::PathBuf;
// @

// <[imports]>=
use anyhow::Result;
// @
```

They support extension points without requiring a full programming language in
the document format.

### Reference-Site Indentation

Indentation belongs at the reference site, not in every chunk body. This makes
chunks reusable in different contexts.

```text
fn main() {
    // <[body]>
}
```

The expansion should inherit the indentation of the reference.

## What Failed

### Dogfooding Everything

Making the implementation itself literate introduced constant maintenance
pressure:

- every source edit became a question of which layer to edit;
- generated files drifted from sources;
- agents had to learn the project-specific workflow before fixing ordinary
  bugs;
- large literate files became harder to navigate than normal Rust modules.

Reweave therefore uses ordinary Rust source.

### Apply-Back

Apply-back looked aligned with the philosophy but violated the Zen of Python
rule:

```text
If the implementation is hard to explain, it's a bad idea.
```

It required reverse mapping from generated files into macro-expanded,
chunk-expanded source. Macro calls inside macro bodies, included files, and
synthetic text made the semantics hard to explain and hard to trust.

Reweave should not implement apply-back.

### Persistent Provenance Databases

The database was useful for ambitious querying but made simple generation much
harder:

- lock contention appeared during parallel tangling;
- schema choices leaked into unrelated code;
- path normalization became a cross-cutting concern;
- generated artifacts depended on persistent state.

Reweave recomputes everything per run.

### Source Maps As A Default Concern

Source maps are useful for diagnostics and debugging, but making them central
to the system pushed the project toward bidirectional editing.

For Reweave, attribution should remain optional and local. If a feature needs a
persistent source-map store, it probably belongs outside Reweave.

### Mixed Markup Ambition

AsciiDoc was technically powerful but had dependency and ecosystem costs.
Markdown is less expressive but more universally understood by tools, humans,
and agents.

Reweave standardizes on Markdown inputs. If richer structures are needed, add
small explicit macros or chunk conventions rather than a second markup engine.

## Design Heuristics

Use these when deciding whether a feature belongs in Reweave.

- If it makes forward generation simpler, consider it.
- If it exists mainly to recover from generated-file edits, reject it.
- If it needs persistent state, reject it by default.
- If it hides global effects in innocent-looking syntax, redesign it.
- If it improves local reasoning, it is probably valuable.
- If it makes the workflow unfamiliar to agents, the cost is real.
- If the implementation is hard to explain, treat that as evidence against the
  idea.

## Agent-Oriented Rules

Coding agents work best with ordinary files, normal tests, and explicit
commands. Reweave should preserve that.

Preferred:

- edit Rust directly;
- write or update tests directly;
- run `cargo test`, `cargo clippy`, and `cargo llvm-cov`;
- use Markdown examples as input fixtures.

Avoid:

- generated Rust checked in as the main source of truth;
- hidden preprocessing steps;
- special local databases;
- source mutations triggered by macro evaluation;
- repository-specific workflows that require long explanations.

## What Reweave Should Become

Reweave should be a small, understandable tool:

```text
read Markdown
expand macros if enabled
collect chunks
expand chunks
write files
```

The implementation should remain easier to explain than the Weaveback system it
replaced.
