#!/usr/bin/env python3
"""Checks that a doc-only edit of std left its code alone.

The examples in std's doc comments are checked by the compiler itself
([doc-example]): `salvo analyze` in `lib/` reports what one finds, and a
docs test asserts it finds nothing. What is left here is the git side:

    tools/check-doc-examples.py --code-unchanged [files]

compares every line of each `.sv` file that is neither blank nor only a
comment with the same file at HEAD (default: every non-test file under
lib/std), so a rewrite of comments that moved code is caught.

Exit status 1 when a file's code changed.
"""
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def code_lines(text):
    """The lines of a Salvo source that are not blank and not only a comment."""
    return [l.rstrip() for l in text.split("\n") if l.strip() and not l.strip().startswith("//")]


def code_unchanged(files):
    changed = 0
    for path in files:
        rel = os.path.relpath(path, ROOT)
        r = subprocess.run(["git", "show", f"HEAD:{rel}"], cwd=ROOT, capture_output=True, text=True)
        if r.returncode != 0:
            continue
        before, after = code_lines(r.stdout), code_lines(open(path).read())
        if before != after:
            changed += 1
            for n, (a, b) in enumerate(zip(before, after)):
                if a != b:
                    print(f"{rel}: code changed at code line {n + 1}:\n    was: {a}\n    now: {b}")
                    break
            else:
                print(f"{rel}: code changed (now {len(after)} code lines, was {len(before)})")
    print(f"{len(files)} files, {changed} with code changes", file=sys.stderr)
    return changed


def main():
    args = sys.argv[1:]
    if "--code-unchanged" not in args:
        print(__doc__.strip(), file=sys.stderr)
        sys.exit(2)
    files = [a for a in args if a != "--code-unchanged"]
    if not files:
        for dirpath, _, names in os.walk(os.path.join(ROOT, "lib", "std")):
            for name in sorted(names):
                if name.endswith(".sv") and not name.endswith(".test.sv"):
                    files.append(os.path.join(dirpath, name))
    sys.exit(1 if code_unchanged(sorted(files)) else 0)


if __name__ == "__main__":
    main()
