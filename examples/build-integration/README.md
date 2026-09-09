# Build Integration Example

A minimal but complete setup for embedding reweave in a Meson build — no
wrapper scripts, reweave does everything itself.

## Layout

```text
meson.build       the gen-src target
src/main.md       main document (%include's the fragment)
src/fragment.md   fragment with a shared chunk
```

## Run

```sh
meson setup build
ninja -C build
```

This tangles `src/main.md` (which pulls in `src/fragment.md`) and writes
`src/generated/hello.txt`. Run `ninja -C build` again: nothing happens —
the depfile sees no `.md` changes, and reweave's content-aware writes would
preserve mtimes anyway. Edit `src/fragment.md` and the output is regenerated
on the next `ninja`.

How it works, with no external tooling:

- **Fragment handling**: `src/fragment.md` is never tangled standalone.
  reweave discovers the include through its own macro evaluator (macro-computed
  include arguments work too), splices the fragment into the main document,
  and tangles only that.
- **Change detection**: `write_files` compares expanded content with what is
  on disk and only writes on change, so downstream steps are not re-triggered
  by a no-op regeneration.
- **Build plumbing**: `--stamp` / `--depfile` give ninja its target file and
  dependency list (all inputs plus every resolved include).
