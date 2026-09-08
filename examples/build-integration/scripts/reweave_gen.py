#!/usr/bin/env python3
"""reweave_gen: drive reweave over the literate Markdown sources.

Usage: reweave_gen.py <project_root> <stamp> <depfile> <reweave_bin>

reweave is forward-only: it rewrites every @file on every run and has no
depfile or stamp support. This wrapper:

1. discovers main documents (all src/**/*.md minus %include'd fragments),
2. tangles them into a temp dir via reweave,
3. syncs only content-changed files into the source tree (preserving
   mtimes so ninja does not rebuild when nothing changed),
4. emits a stamp and a depfile listing every src/**/*.md (covers %include
   dependencies implicitly).
"""

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

INCLUDE_RE = re.compile(r"%include\(([^)]+\.md)\)")


def discover(root: Path) -> tuple[list[str], list[Path]]:
    """Return (main inputs relative to root, all md files under src/)."""
    all_md = sorted((root / "src").rglob("*.md"))
    included: set[str] = set()
    for md in all_md:
        included.update(INCLUDE_RE.findall(md.read_text()))
    mains = [
        str(p.relative_to(root))
        for p in all_md
        if str(p.relative_to(root)) not in included
    ]
    return mains, all_md


def tangle(reweave_bin: str, mains: list[str], root: Path, out_dir: str) -> None:
    """Run reweave over the main documents, writing @file chunks to out_dir."""
    subprocess.run(
        [reweave_bin, *mains, "-I", str(root), "-o", out_dir],
        cwd=root,
        check=True,
    )


def sync_changed(out_dir: Path, dest_root: Path) -> list[str]:
    """Copy files into dest_root only when content differs.

    Preserves mtimes of unchanged files. Returns relative paths of the
    files that were written.
    """
    changed: list[str] = []
    for gen in sorted(out_dir.rglob("*")):
        if not gen.is_file():
            continue
        rel = gen.relative_to(out_dir)
        dest = dest_root / rel
        if not dest.exists() or dest.read_bytes() != gen.read_bytes():
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(gen, dest)
            changed.append(str(rel))
    return changed


def write_stamp_and_depfile(stamp: str, depfile: str, changed: list[str], deps: list[Path]) -> None:
    Path(stamp).write_text("\n".join(changed) + "\n")
    Path(depfile).write_text(f"{stamp}: {' '.join(str(p) for p in deps)}\n")


def main() -> None:
    root_arg, stamp, depfile, reweave_bin = sys.argv[1:5]
    root = Path(root_arg).resolve()

    mains, all_md = discover(root)
    with tempfile.TemporaryDirectory() as tmp:
        tangle(reweave_bin, mains, root, tmp)
        changed = sync_changed(Path(tmp), root / "src")

    write_stamp_and_depfile(stamp, depfile, changed, all_md)


if __name__ == "__main__":
    main()
