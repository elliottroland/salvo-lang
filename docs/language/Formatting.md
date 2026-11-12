# Formatting: `salvo fmt`

## Overview

`salvo fmt` rewrites Salvo source in one layout, so that code reads the same
whoever wrote it and a review shows changes rather than spacing. It works
like `gofmt` or `rustfmt`: it parses each file and prints the tree back.
Comments, the blank lines between statements and declarations, the author's
line breaks, grouping parentheses and the way a literal is spelled are kept.
It only parses, so it formats a file that does not type-check, and it never
changes what a program means.

This file:

```salvo
// Totals of the day's orders.

import std.random.DefaultRandom
// The queue the orders wait in.
import std.heap.Heap

fn total(prices: List<Int>, discount: Int)->Int {
  let sum = 0


  for p in prices { sum = sum + p }
  return sum - discount  // never below zero: see `check`
}

fn today() -> Int {
    return total(List(19_99, 5_49, 12_00, 3_75, 8_25, 14_50, 6_00, 2_99, 9_99, 11_49, 7_25, 4_99, 13_75, 1_25, 10_00, 15_75), 1_00)
}
```

is formatted as:

```salvo
// Totals of the day's orders.

// The queue the orders wait in.
import std.heap.Heap
import std.random.DefaultRandom

fn total(prices: List<Int>, discount: Int) -> Int {
    let sum = 0

    for p in prices {
        sum = sum + p
    }
    return sum - discount // never below zero: see `check`
}

fn today() -> Int {
    return total(
        List(19_99, 5_49, 12_00, 3_75, 8_25, 14_50, 6_00, 2_99, 9_99, 11_49, 7_25, 4_99, 13_75, 1_25, 10_00, 15_75),
        1_00
    )
}
```

The imports are sorted, the comment above one moving with it; the two
blank lines became one; the body is indented four spaces; and the call too
wide for 120 columns broke its arguments one per line. Unlike `rustfmt` or
ktlint there are no options: the layout is the language's.

## The command

```text
salvo fmt                 # every .sv file of the project, in place
salvo fmt src/app lib.sv  # the files named, and every .sv under the directories named
salvo fmt --check         # write nothing; list what would change, and fail if anything would
salvo fmt -               # standard input to standard output, for editors
```

- **Without paths**, it formats every `.sv` file under the source root of
  the project the working directory is in: the files `salvo analyze` reads
  (see [Finding the sources](Modules.md#finding-the-sources)), test files
  (`<module>.test.sv`) included. Outside a project, name the paths.
- **A directory named** is formatted throughout, except its hidden
  directories and cache directories (those with a `CACHEDIR.TAG`, such as
  Cargo's `target/`); a nested project or a `.svignore` entry under it is
  formatted too, since you named it. A file named must be a `.sv` file.
- **A file is written only when its text changes**, so a formatted file
  keeps its modification time. The command ends by saying how many files
  it reformatted, on stderr.
- **`--check`** writes nothing. It prints each file that would change on
  stdout, one per line, relative to the working directory, and exits with
  status 1 when there is one. It suits a CI step or a commit hook.
- **`-`**, as the only path, reads the source from standard input and
  writes the formatted text to standard output. With `--check` it prints
  nothing and exits with status 1 when the input is not formatted.
- **A file that does not parse is left as written.** Its parse errors are
  reported as `salvo analyze` reports them, the other files are still
  formatted, and the exit status is 1.

The standard library is formatted like any project: run `salvo fmt` in
`lib/`.

## What the formatter writes

- **Layout**: four spaces of indentation, LF line endings, one newline at
  the end of the file, and lines of at most 120 characters.
- **Line breaks**: a list (a call's arguments, a parameter list, type
  arguments, a tuple, a struct literal or pattern) or a chain (`?:`, `|>`,
  `?>`, `replyto`, a `.` call chain, a run of one binary operator) that the
  author broke anywhere prints broken, one item per line; one written on
  one line stays on one line while it fits, and breaks when it does not. A
  broken list puts its closing bracket on its own line and gets no trailing
  comma. A chain's continuation lines start with its operator (`?:`, `|>`,
  `?>`, `.`, `replyto`, `&&`, `+`, a union's `|`, …), also where the author
  wrote it at the end of the line before; a binary operator after a `}` ends
  its line instead, since a line it began there would not continue the
  expression.
- **Comments**: every comment is kept, as written, where it was written: a
  run above a declaration or a statement, a comment at the end of a line,
  one before a closing `}`. A comment inside a bracketed list breaks the
  list. Comments are never reflowed.
- **Blank lines**: one or more blank lines become one; there are none at
  the start or end of a body. Two declarations get a blank line between
  them, except two one-line declarations (imports, aliases) written on
  consecutive lines, which stay together.
- **Kept as written**: grouping parentheses, even redundant ones (`(a + b)`
  stays), and the spelling of a literal (`1_000`, `0.50`, an escape in a
  string). Parentheses are added only where precedence needs them.
- **Canonical spellings** where the program reads the same either way: a
  handler with no parameters is written without brackets (`handler H for
  E`), and a deduction entry writes the claims it establishes first
  (`list: Mut +Heap<T>(?cmp)` becomes `list: +Heap<T>(?cmp) Mut`).
- **Imports**: a file's imports are sorted by the imported path, and
  nothing else is reordered or removed. A comment written directly above
  an import moves with it, and a comment at the end of an import's line is
  part of it; the blank lines between imports go. The language server's
  *Organize imports* sorts the same way and also removes unused imports,
  which needs the program checked; the formatter removes nothing.

## Caveats

**Imports that stay where they are.** The imports are left in their
written order when a declaration sits between two of them, when a comment
with a blank line after it stands among them (a heading over a group of
imports, which belongs to no one import), or when sorting would move a
comment to the very top of a file, where it would become the module's
documentation (see [Module documentation](Modules.md#module-documentation)).

**A file the formatter refuses.** Before writing, the formatter checks that
its output parses to the same program and keeps every comment exactly
once. If it ever does not, that is a defect of the formatter: the file is
left as written, an error says so, and the exit status is 1.

## In the editor

The language server (`salvo lsp`) formats a whole document on request, the
same way `salvo fmt` does; in VS Code that is *Format Document*, and
`editor.formatOnSave` runs it on every save. A document that does not
parse is left alone, so formatting a half-written file does nothing.
There is no formatting of a selection.
