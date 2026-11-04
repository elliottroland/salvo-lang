#!/usr/bin/env python3
"""Type-checks the code examples in std's doc comments.

Every fenced block in a doc comment (``` or ```salvo; ```text and other
languages are skipped) becomes a small program in tmp/doc-examples/, which
`salvo analyze` checks:

* an example with top-level declarations (`fn`, `struct`, `effect`, …) is
  checked as written;
* any other example is a fn body: it is wrapped in
  `fn example() [use] -> None { use StdConsole() … }`, so it may print;
* the documented module is imported unless it is in `std.core` (which every
  file sees).

A fence holding a `test "…"` (or `test actor`) block is checked as a test annex
(`main.test.sv`, beside an empty `main.sv`).

A fence whose first line is `no-check` is skipped (`salvo docs` and hover
leave that line out): for an example that
shows an error on purpose, or one that needs more than the wrapper gives.

    tools/check-doc-examples.py                 every file under lib/std
    tools/check-doc-examples.py lib/std/heap.sv  just these
    tools/check-doc-examples.py --code-unchanged [files]
                                                 check that a doc-only edit
                                                 left every non-comment line
                                                 as it is at HEAD

Exit status 1 when an example has an error. Uses target/debug/salvo as built.
"""
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SALVO = os.path.join(ROOT, "target", "debug", "salvo")
OUT = os.path.join(ROOT, "tmp", "doc-examples")
TOP = re.compile(r"^(export |fn |struct |linear |effect |actor |handler |type |qualifier |provenance |fns |iter fn |import |namespace |platform |intrinsic |test |send |comptime |threadsafe |iterable |opaque )")


def module_of(path):
    rel = os.path.relpath(path, os.path.join(ROOT, "lib"))
    return rel[: -len(".sv")].replace(os.sep, ".")


def examples(path):
    """(line, code) for every checked fence in a comment of `path`."""
    lines = open(path).read().split("\n")
    out = []
    i = 0
    while i < len(lines):
        m = re.match(r"^\s*// ?(```+)(\w*)\s*$", lines[i])
        if not m:
            i += 1
            continue
        fence, lang = m.group(1), m.group(2)
        start = i
        body = []
        i += 1
        while i < len(lines):
            stripped = re.match(r"^\s*//( ?)(.*)$", lines[i])
            if not stripped:
                break
            text = stripped.group(2) if stripped.group(1) else stripped.group(2)
            if text.strip().startswith(fence):
                break
            body.append(text)
            i += 1
        i += 1
        if lang not in ("", "salvo"):
            continue
        if body and body[0].strip() == "no-check":
            continue
        out.append((start + 1, "\n".join(body)))
    return out


def program(module, code):
    imports = "" if module.startswith("std.core.") else f"import {module}\n\n"
    if any(TOP.match(l) for l in code.split("\n")):
        return imports + code + "\n"
    body = "\n".join(("    " + l) if l.strip() else "" for l in code.split("\n"))
    return f"{imports}fn example() [use] -> None {{\n    use StdConsole()\n{body}\n}}\n"


def check(path, n, line, code):
    module = module_of(path)
    d = os.path.join(OUT, f"{module}.{n}")
    os.makedirs(d, exist_ok=True)
    for f in os.listdir(d):
        os.remove(os.path.join(d, f))
    src = program(module, code)
    if any(l.startswith("test ") for l in code.split("\n")):
        open(os.path.join(d, "main.sv"), "w").write("")
        open(os.path.join(d, "main.test.sv"), "w").write(src)
    else:
        open(os.path.join(d, "main.sv"), "w").write(src)
    r = subprocess.run([SALVO, "analyze", "--src", d], capture_output=True, text=True)
    errors = [l for l in r.stderr.split("\n") if l.startswith("error")]
    return errors, d


def code_lines(text):
    """The lines of a Salvo source that are not blank and not only a comment."""
    return [l.rstrip() for l in text.split("\n") if l.strip() and not l.strip().startswith("//")]


def code_unchanged(files):
    """Checks that a doc-only edit left the code of each file as it is at
    HEAD: the lines that are neither blank nor a comment must be identical."""
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
    unchanged = "--code-unchanged" in args
    files = [a for a in args if a != "--code-unchanged"]
    if not files:
        for dirpath, _, names in os.walk(os.path.join(ROOT, "lib", "std")):
            for name in sorted(names):
                if name.endswith(".sv") and not name.endswith(".test.sv"):
                    files.append(os.path.join(dirpath, name))
    if unchanged:
        sys.exit(1 if code_unchanged(sorted(files)) else 0)
    failed = 0
    jobs = []
    for path in sorted(files):
        for n, (line, code) in enumerate(examples(path)):
            jobs.append((path, n, line, code))
    from concurrent.futures import ThreadPoolExecutor
    with ThreadPoolExecutor(max_workers=os.cpu_count() or 4) as pool:
        results = list(pool.map(lambda j: check(*j), jobs))
    total = len(jobs)
    for (path, n, line, code), (errors, d) in zip(jobs, results):
        if errors:
            failed += 1
            rel = os.path.relpath(path, ROOT)
            print(f"{rel}:{line}: example fails ({os.path.relpath(d, ROOT)}/main.sv)")
            for e in errors[:4]:
                print(f"    {e}")
    print(f"{total} examples, {failed} failing", file=sys.stderr)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
