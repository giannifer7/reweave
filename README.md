# Reweave

Reweave is the forward-only extraction of the useful parts of Weaveback.

It keeps:

- the strict macro evaluator;
- Markdown source files as the authoring format;
- noweb-style chunk assembly;
- direct generation of `@file` chunks.

It removes:

- provenance databases;
- source maps and query tools;
- apply-back / reverse editing;
- overwrite protection and generated-file reconciliation;
- documentation rendering, containers, release workflows, and distribution machinery.

The intended user is a programmer who is comfortable building from source and
who wants a small macro+tangle tool, not a full literate-programming platform.

## Build

```sh
cargo build
```

Run the tests:

```sh
cargo test --workspace
```

## Basic Use

Write Markdown with fenced code blocks that contain chunk definitions:

````md
# Example

```rust
// <[@file src/main.rs]>=
fn main() {
    // <[body]>
}
// @

// <[body]>=
println!("hello");
// @
```
````

Generate files:

```sh
cargo run --bin reweave -- examples/hello.md --out /tmp/reweave-out
```

This writes `/tmp/reweave-out/src/main.rs`.

## Chunk Syntax

- `// <[@file path]>=` starts an output-file chunk.
- `// <[name]>=` starts a named chunk.
- `// @` ends the current chunk.
- `// <[name]>` references a named chunk.
- `@replace` can replace an earlier chunk definition.
- `@reversed`, `@compact`, and `@tight` can modify a chunk reference.

Comment markers are configurable and default to `//` and `#`.

## Macro Expansion

Reweave expands macros before tangling unless `--no-macro` is passed.

````md
%def(line, text, %{println!("%(text)");%})

```rust
// <[@file src/main.rs]>=
fn main() {
    %line(hello)
}
// @
```
````

Useful CLI options:

- `-D NAME=VALUE` defines a top-level macro variable.
- `-I DIR` adds a macro include directory.
- `--sigil ¤` changes the macro sigil.
- `--allow-env` enables `%env(NAME)`.
- `--no-macro` tangles Markdown without macro expansion.

## Project Layout

- `reweave-macro`: macro evaluator and its preserved test suite.
- `reweave-tangle`: forward-only chunk parser, expander, and writer.
- `reweave-cli`: the `reweave` command.
- `reweave-core`: shared constants.

The source is ordinary Rust and Markdown. Reweave does not dogfood itself.
