# Build Integration Example

A minimal but complete setup for embedding reweave in a Meson build.

## Layout

```text
meson.build              the gen-src target
scripts/reweave_gen.py   main-document discovery, tangle, change-only sync,
                         stamp + depfile emission
src/main.md              main document (%include's the fragment)
src/fragment.md          fragment with a shared chunk
```

## Run

```sh
meson setup build
ninja -C build
```

This tangles `src/main.md` (which pulls in `src/fragment.md`) and writes
`src/generated/hello.txt`. Run `ninja -C build` again: nothing happens,
because the depfile sees no `.md` changes and the sync preserves mtimes.
Edit `src/fragment.md` and the output is regenerated on the next `ninja`.

Note that only *main documents* are fed to reweave: the wrapper computes them
as every `src/**/*.md` minus the files pulled in via `%include(...)`, because
reweave tangles each input standalone and rejects duplicate `@file` chunks.
