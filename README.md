# Reweave

Reweave is a small Markdown-to-files tool for projects that want literate-style
source assembly without a database, reverse mapping, or generated-file
management.

You write Markdown documents containing fenced code blocks. Inside those code
blocks, noweb-style chunks describe output files and reusable snippets. Reweave
optionally expands a strict macro language first, then tangles the chunks and
writes the requested files directly.

## What It Does

- Reads one or more Markdown files.
- Expands `%` macros by default.
- Scans code fences for chunk definitions.
- Expands named chunk references.
- Writes every `@file` chunk to an output directory.

Reweave is intentionally forward-only: input Markdown goes in, generated files
come out. It does not track provenance, reconcile hand edits in generated files,
or maintain persistent state.

## Build

Build the workspace:

```sh
cargo build
```

Build an optimized release executable:

```sh
cargo build --release --bin reweave
```

The release binary is written to:

```sh
target/release/reweave
```

Run the test suite:

```sh
cargo test --workspace
```

Run clippy with warnings denied:

```sh
cargo clippy --workspace -- -D warnings
```

Check coverage:

```sh
cargo llvm-cov --workspace --summary-only
```

## Basic Use

Create a Markdown file with chunk definitions in fenced code blocks:

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

Or use a release binary:

```sh
target/release/reweave examples/hello.md --out /tmp/reweave-out
```

This writes:

```text
/tmp/reweave-out/src/main.rs
```

## CLI

```text
reweave [OPTIONS] [INPUTS]...
```

Common options:

```text
-o, --out DIR              Output directory for @file chunks
--dir DIR                  Recursively read files under DIR
--ext EXT                  Extension for --dir discovery, default: md
--no-macro                 Tangle input without macro expansion
-D, --define NAME=VALUE    Define a top-level macro variable
-I, --include DIR          Add a macro include/import search path
--sigil CHAR               Change the macro sigil, default: %
--allow-env                Enable %env(NAME)
--env-prefix PREFIX        Prefix applied to environment lookups
--open-delim TEXT          Noweb open delimiter, default: <[
--close-delim TEXT         Noweb close delimiter, default: ]>
--chunk-end TEXT           Chunk end marker, default: @
--comment-marker TEXT      Accepted chunk comment marker, repeatable
--recursion-limit N        Macro and chunk expansion recursion limit
```

Directory mode:

```sh
reweave --dir docs --ext md --out generated
```

Multiple inputs are processed in argument order:

```sh
reweave chapters/intro.md chapters/impl.md --out generated
```

## Chunk Syntax

Chunk syntax is recognized inside normal text too, but it is intended to be
used inside fenced code blocks.

### Output File Chunks

An `@file` chunk writes a generated file relative to `--out`:

```rust
// <[@file src/lib.rs]>=
pub fn answer() -> u32 {
    42
}
// @
```

Output paths must be relative and must not contain `..`.

### Named Chunks

Named chunks collect reusable code:

```rust
// <[imports]>=
use std::path::Path;
// @
```

Reference a named chunk from another chunk:

```rust
// <[@file src/main.rs]>=
// <[imports]>

fn main() {}
// @
```

Named chunks with the same name accumulate in definition order:

```rust
// <[body]>=
first();
// @

// <[body]>=
second();
// @
```

### Replacing Chunks

Use `@replace` to replace earlier definitions:

```rust
// <[@replace body]>=
replacement();
// @
```

For file chunks:

```rust
// <[@replace @file src/main.rs]>=
fn main() {}
// @
```

### Reference Modifiers

Modifiers are placed inside a chunk reference before the chunk name:

```rust
// <[@reversed rows]>
// <[@compact body]>
// <[@tight body]>
```

- `@reversed` expands multiple definitions in reverse order.
- `@compact` trims blank lines from the start and end of the expansion.
- `@tight` trims edge blank lines and removes all blank lines.

### Comment Markers

By default, chunk lines may be prefixed with `//` or `#`:

```text
# <[@file out.txt]>=
hello
# @
```

Use `--comment-marker` to configure accepted markers.

## Macro Language

Macros are expanded before tangling unless `--no-macro` is used. The default
sigil is `%`.

### Variables

Set a variable:

```text
%set(name, Ada)
```

Read a variable:

```text
Hello %(name)
```

Define variables from the CLI:

```sh
reweave input.md -D name=Ada --out generated
```

Variables are scoped. Macro parameters are available only while that macro is
expanding.

### Macro Definitions

Define a macro:

```text
%def(greet, name, Hello %(name)!)
%greet(Ada)
```

Output:

```text
Hello Ada!
```

Macro bodies can be blocks:

```text
%def(fn_line, name, %{
fn %(name)() {}
%})
```

Call arguments are evaluated before the macro body runs. Missing parameters,
too many positional arguments, unknown named arguments, or positional arguments
after named arguments are errors.

Named arguments are supported:

```text
%def(tag, name, value, <%(name)>%(value)</%(name)>)
%tag(value=hello, name=span)
```

### Redefinition

`%def` creates a constant binding. Use `%redef` when a macro is intentionally
replaceable:

```text
%redef(render, old)
%redef(render, new)
%render()
```

### Blocks

Blocks delay parsing of commas and parentheses inside the block:

```text
%def(wrap, body, %{
before
%(body)
after
%})
```

Tagged blocks are useful when the content itself contains `%}`:

```text
%def(raw, %tag{
literal content
%tag})
```

Verbatim blocks use square brackets and are not macro-expanded while lexed:

```text
%[literal %(text)%]
```

Tagged verbatim blocks are also supported:

```text
%raw[literal %(text)%raw]
```

### Conditionals

`%if(condition, then[, else])` treats a non-empty condition as true:

```text
%if(%(name), Hello %(name), Missing name)
```

With no `then` branch, true expands to an empty string.

### Pattern Matching

`%match(value, default, regex1, result1, regex2, result2, ...)` evaluates the
first matching branch:

```text
%match(error-404, unknown,
       ^warn-\d+$, warning,
       ^error-\d+$, error)
```

Regex captures are exposed inside the selected result as `%(match_1)`,
`%(match_2)`, and named captures by name:

```text
%match(error-404, no,
       %[^(?P<kind>[a-z]+)-(\d+)$%],
       %{kind=%(kind), code=%(match_2)%})
```

### Includes And Imports

Include another file and expand its output in place:

```text
%include(header.md)
```

Import another file for definitions only:

```text
%import(macros.md)
```

Search paths are controlled with `-I` / `--include`.

### Aliases

`%alias(new_name, source_name[, key=value, ...])` creates a replaceable copy of
an existing macro. Named overrides freeze free variables for the alias:

```text
%def(render, msg, [%(level)] %(msg))
%alias(warn, render, level=WARNING)
%warn(check this)
```

### Dynamic Calls

`%eval(name[, args...])` calls a macro whose name is computed:

```text
%set(which, greet)
%eval(%(which), Ada)
```

### Export

`%export(name)` copies a variable or macro from an inner macro scope to the
enclosing scope. Calling `%export` at global scope is allowed but produces a
warning.

### Environment Variables

Environment access is disabled by default. Enable it with `--allow-env`:

```text
%env(HOME)
```

With a prefix:

```sh
reweave input.md --allow-env --env-prefix REWEAVE_
```

Then `%env(CONFIG)` reads `REWEAVE_CONFIG`.

### String Helpers

```text
%capitalize(text)
%decapitalize(text)
%convert_case(text, snake)
%to_snake_case(text)
%to_camel_case(text)
%to_pascal_case(text)
%to_screaming_case(text)
```

Supported case names include:

```text
snake
camel
pascal
kebab
screaming
screaming_kebab
ada
lower
upper
```

### Predicates

```text
%eq(a, b)
%neq(a, b)
%not(value)
```

These return non-empty strings for true and empty strings for false, so they
compose with `%if`.

### Python-Style Script Macros

Reweave also supports `%pydef`, backed by the embedded Monty evaluator:

```text
%pydef(double, x, %{str(int(x) * 2)%})
%double(21)
```

Store helpers are available to script macros:

```text
%pyset(counter, 1)
%pyget(counter)
```

Script parameters shadow store keys with the same name.

## Strictness

Reweave deliberately fails on ambiguous or unsafe input:

- Undefined macros and variables are errors.
- Undefined chunks are errors by default.
- Recursive chunk references are detected.
- Macro recursion is bounded by `--recursion-limit`.
- Output paths must be relative and safe.
- `%set` is not allowed in macro argument position.

## Project Layout

```text
reweave-cli      Command-line entry point
reweave-macro    Strict macro evaluator
reweave-tangle   Chunk parser, expander, and file writer
reweave-core     Shared constants
examples         Small input examples
docs             Design notes
```
