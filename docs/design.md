# Design

Reweave is intentionally small.

The core pipeline is:

```text
Markdown input
  -> optional macro expansion
  -> chunk scan
  -> chunk expansion
  -> direct file writes
```

There is no persistent state. Every run recomputes from the input files.

## Non-goals

- No reverse mapping from generated files to Markdown.
- No database.
- No apply-back (editing sources through generated files).
- No edit protection for generated outputs.
- No generated documentation pipeline.
- No distribution automation.

## Kept

The strict macro evaluator: strict bindings, explicit rebinding, verbatim
blocks, a Python/Monty escape hatch, and thorough tests.

The noweb chunk expander, with the practical behaviors:

- accumulated named chunks;
- `@file` output chunks;
- `@replace`;
- `@reversed`;
- `@compact`;
- `@tight`;
- indentation at the reference site;
- content-aware writes (unchanged outputs keep their mtime).

## Removed

The DB and source-map layer: it served tracing and apply-back, made the system
much harder to understand and maintain, and falls outside the forward-only
scope. Tracing outputs and the second ("`*_to`") evaluation engine that fed
them went with it.

The `%here` source-editing builtin was removed for the same reason. Reweave
does not modify its own input files.
