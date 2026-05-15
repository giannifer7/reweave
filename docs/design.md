# Design

Reweave is intentionally smaller than Weaveback.

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
- No apply-back.
- No edit protection for generated outputs.
- No generated documentation pipeline.
- No distribution automation.

## Kept From Weaveback

The macro evaluator is kept because it was one of the valuable parts of the
experiment: strict bindings, explicit rebinding, verbatim blocks, Python/Monty
escape hatch, and tests.

The chunk expander keeps the practical noweb behavior:

- accumulated named chunks;
- `@file` output chunks;
- `@replace`;
- `@reversed`;
- `@compact`;
- `@tight`;
- indentation at the reference site.

## Removed From Weaveback

The DB and source-map layer were removed because they served tracing and
apply-back. Those features made the system much harder to understand and
maintain, and they are outside Reweave's forward-only scope.

The `%here` source-editing builtin was removed for the same reason. Reweave
does not modify its own input files.
