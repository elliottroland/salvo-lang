# Modules and files

## Overview

A Salvo program is a tree of `.sv` files, and **every file is a module**. The
module's name is the file's path under the source root, so a file never
declares its own name or package (as Java and Kotlin files do): `shop/shapes.sv`
is the module `shop.shapes`, and moving the file renames the module.

Two rules decide what one module can see of another:

* **Nothing arrives unasked.** A file names what it uses in an `import`. The
  one exception is the `std.core` modules (strings, lists, maps, the console
  and the rest of the basics), which every file sees.
* **Nothing leaves unasked.** A declaration is private to its module unless it
  is marked `export`.

```salvo
// shop/shapes.sv: the module `shop.shapes`

export struct Square { side: Int }

export fn area(s: Square) -> Int => s {
    return squared(s.side)
}

fn squared(n: Int) -> Int {       // private: only this file can call it
    return n * n
}
```

```salvo
// main.sv: the module `main`
import shop.shapes.Square
import shop.shapes.area

fn main() [use] -> None {
    use StdConsole()
    println("${area(Square { side: 3 })}")    // 9
}
```

This is close to how Rust and TypeScript work (a file is a module, items are
private until exported) and further from Java and Kotlin, where a package
spans many files and members are public by default. Privacy here is per file:
there is no "package-private" across files.

The rest of this page covers, in order: how the compiler finds the files and
which project they belong to; imports, including whole-module imports;
`export`, `opaque` and what privacy means; the naming rules the language
enforces; the requirement that every name be declared; namespaces, which group
declarations inside a file; and documentation comments, which `salvo docs`
turns into pages like the module pages these topics link to.

## Finding the sources

The file extension for Salvo source code is `.sv`. The compiler discovers
sources by walking the source directory recursively. Hidden directories and
cache directories (those carrying a `CACHEDIR.TAG` marker, such as Cargo's
`target/`) are skipped.

A `.svignore` file at the source root excludes further paths: one path per
line, relative to the root, naming a file or a whole directory subtree; blank
lines and `#` comments are ignored.

```text
# .svignore
scratch
old/experiment.sv
```

Only the modules which are used in the code are transpiled to the relevant
backend equivalent (modules in Rust, packages in Kotlin).

## Projects and the manifest

A **project** is a directory holding a `salvo.toml`: the manifest that says
what the sources are, where the program starts, what to build for, what other
projects it depends on, and what host libraries its platform code needs. Every
command (`salvo run`, `compile`, `build`, `test`, `analyze`, `platform
generate`) and the language server read it, so the command line and the editor
agree on what a project is.

A file **belongs to the nearest manifest above it**, and a nested manifest is
a boundary: a directory holding several projects (this repository, with `lib/`
and every `examples/*/`) is several programs, and the editor analyses each
document under its own.

The manifest has its own page: see [The Manifest](Manifest.md) for every
section and option: `[project]`, `[build]`, the per-backend `[rust]`/`[kotlin]`
sections, `[dependencies]`, host libraries, the `salvo.lock` file, and how the
settings interact.

## Imports

Everything must be imported explicitly unless it is defined in one of the
`std.core` modules. This includes qualifiers, types, effects, handlers, and
functions. An import names one declaration by its module path and its name.
When there is ambiguity in a reference (e.g. functions which have the same
signature, qualifiers which apply to the same type), then they can be aliased
using `as` in the import statement.

```salvo
import std.random.Random // import the Random<T> effect from std/random.sv
import list.add // import that list's add function from list.sv
import list.ext.add as ext_add // import the add function from list/ext.sv, now aliased
import std.core.Str // not strictly necessary: all of std.core is imported by default
```

An import is always fully qualified: it says where a name comes from. A
*selector* is different: `size@list`, `between@time`, `by auto` and
`@core.list` name a module at a use site, and may name it by any unambiguous
suffix of its path.

### The `std` root

**Every standard-library module is under `std`**: `std.time`, `std.fs.mem`,
`std.core.list`, `std.test`. The library's tree is laid out that way too
(`lib/std/core/list.sv` is module `std.core.list`), since a module path is
always the path under the source root. `std` is a reserved root: only the
standard library declares modules there, so a project file at `std/…` is an
error naming the reason, and nothing a project writes is ever implicitly
visible to std. An import that leaves the root off is refused with the path it
meant:

```salvo
import time    // error: the standard library's modules are under `std`: write `import std.time`
```

The refusal applies only when nothing else answers to the first segment, so a
project's own `time` module is still `import time`. Selectors are unaffected:
`size@list`, `between@time`, `by auto` and `@core.list` name a module by a
suffix of its path, and they still do.

### Whole-module imports

A module can also be imported *whole*, which brings every name in it into
scope with one line:

```salvo
import std.time // Instant, Tick, Duration, Clock, Ticker, Timer, millis, between, ...
```

This is how a std surface that is not implicitly visible stays cheap to use:
`std.time` is one import rather than a dozen.

**It names one module, not the tree under it.** `import std.fs` does not bring
`std.fs.mem`; that is a second line. The reading that imports less is the
default, because importing more is always one more statement while importing
less would need new syntax. A path that names no module but has modules under
it is refused, and the error lists them.

**It is the weakest way for a name to arrive.** Your own module's declarations
win over it, and so does a named import, both silently: a file that declares
its own `Duration` keeps it, and `import other.Duration` beside
`import std.time` picks the other one. Functions are not shadowed that way: an
overload from a bulk import competes with your own on its signature, at a call
and where an implicit is filled alike, and `between@time(a, b)` names the
module explicitly.

**Two modules with one type name.** The one case that cannot be resolved by
hand at the use site is two whole-module imports carrying the same *type*
name, since a type reference has no module selector. There the first module
wins and the compiler warns, naming the import that would settle it.

A module import takes no `as` (there is nothing to qualify a renamed module
with), and `import std.core` is redundant, since `std.core` is always visible.

## Exports and privacy

A declaration is only importable if its own module lets it out.
**Declarations are module-private by default, and `export` is how one becomes
visible elsewhere:**

```salvo
export fn label(n: Int) -> Str {   // callers in other modules may use this
    return "${n}"
}

fn digits(n: Int) -> Int {         // this module's business, and nobody else's
    return 1
}
```

The modifier comes first, before `intrinsic`, `linear`, `actor`, `platform`,
`provenance`, `iter` and `send`, so a declaration reads *who can see it, what
kind it is, what it is called*. It is an ordinary word rather than a reserved
one, so a variable may still be called `export`; only the start of a
declaration makes it a modifier.

It applies to the whole declaration and not its parts: an exported struct
exports its fields, an exported effect its members, an exported handler its
state. There is no field- or member-level visibility. A type you do not export
may still appear in a signature you do, and callers can then hold values of it
without being able to write its name.

**Privacy is per module, which is per file.** Splitting a file in two is
therefore a visibility decision: what the halves share has to be exported. The
standard library is written this way, which is what it is for: `earliest_due`
in `std.time` and the `mem_*` helpers in the in-memory filesystem are reachable
from their own file and nowhere else.

One module is private to the standard library as a whole: `std.runtime`, the
scheduler every program runs on, which std's own modules import and a program
cannot.

**The diagnostics send you to the right file.** Using a private name says it
is declared in module `m` and not exported, and names the fix; it is not
offered as an import, since importing it could not work. An `import` of a
private name is refused at the import line rather than silently importing
nothing. And a name that is in scope but of the wrong *kind* (a qualifier
where a type belongs) keeps its own diagnostic, because that is not a
visibility problem.

### Opaque structs

When callers should be able to name a type but not reach inside it, declare it
**`opaque`**:

```salvo
export opaque struct Bag : Eq<self> by auto {
    items: List<Int>,
    total: Int
}

export fn total(b: Bag) -> Int => b { return b.total }   // fine: Bag's own module
```

In its own module (and that module's `*.test.sv` annex, see
[Testing](Testing.md)) an opaque struct is an ordinary struct. Everywhere else
`Bag` is a type like any other (a parameter, a field, an element of a list),
but its fields are not there: no literal, no field read or write, no spread,
no destructuring. The error names the type as opaque and lists its functions.

What the module generates for it (`by auto` members, `copy`, the wire form) is
generated in the module and usable everywhere, and a comptime function stamped
in another module sees the type as `Opaque`, so it calls `Bag`'s own functions
rather than walking fields it may not see. Opacity is about Salvo code: the
host class still has its fields. It is what lets a module change a
representation later, which a readable struct does not.

## Naming rules

Casing is part of the language, not a convention:

* **Types start with an uppercase letter**: structs, qualifiers, type
  declarations and aliases, effects, handlers, and generic parameters.
* **Values do not**: functions, parameters, fields, variables, bindings, and
  lambda parameters.
* **Module paths are lowercase.** Since a module path is its file path, that
  applies to file and directory names too: `Utils.sv` is a compile-time error
  telling you to rename the file.
* **A file name may not contain a dot.** A module's path comes from where the
  file sits in the directory tree, and a dot in the name would read as a path
  separator that no directory backs: `string.sv` is the module `string`, while
  `string.kotlin.sv` is rejected. There are no per-backend companion files;
  both targets are served from the one source. (The one dot allowed is a test
  annex's `.test`, as in `heap.test.sv`; see [Testing](Testing.md).)

This is what lets the compiler tell a type from a value at the start of a
dotted name, which [namespaces](#namespaces-and-namespaced-names) rely on.

**A variable that is never read is a warning.** Assignment does not count as
a read: a variable only ever written to has no reader, which is the mistake
worth reporting. Prefix the name with `_` to say the omission is deliberate:

```salvo
let spare = compute()    // warning: `spare` is never used;
                         //          prefix it with `_` (`_spare`) if that is deliberate
let _ignored = compute() // no warning
```

Parameters are exempt: a signature often dictates them, and a handler member
implementing an effect cannot drop one.

## Every name is declared

Salvo takes nothing on trust. Every name a program writes, and every member it
reaches for, must be justified by a declaration the compiler can see. There is
no "pass it through and let the target language sort it out": a value's type
tells you exactly what you can do with it.

**Values.** A reference to something nothing declares is an error, not a
passthrough to the target language. Scopes are not hoisted either, so reading
a variable above its `let` is that same error. A name that *is* declared but
is not a value says which it is:

```salvo
let s = Person       // error: `Person` is a struct type, not a value:
                     //        construct one (`Name { … }`)
```

**Types.** Every name written in a type position must resolve to a declaration
in scope: base types (structs, `type` declarations and aliases, `intrinsic
type`s, effects) and qualifiers alike. A name that resolves to nothing is an
error naming the name, wherever it is written: a signature, a struct field, a
`let` annotation, a `for` type, a `canbe` or `with` clause, or an `is` / `when`
check. The diagnostic lists the modules that would bring the name into scope,
if any.

This matters most for qualifiers, because a qualifier and a base type sit next
to each other in the same syntax. `Ok Int` with no `Ok` in scope is not a type
built from an unknown claim: it is a typo or a missing import, and it would
otherwise fail much later and much less obviously, as a check that "can never
succeed" against arms tagged with a qualifier the compiler never heard of. A
name that *does* exist but in the other namespace says so, since no import can
fix it:

```salvo
qualifier Tag for Int

fn f(x: Tag) -> Int { ... }  // error: unknown type `Tag` (`Tag` is a qualifier, not a type)
```

**Members.** A call to a function nobody declared, a field on something that
is not a struct (or a field a struct does not have), `[]` on something that is
not an array, and `for` over something not iterable are all compile-time
errors. A dot call still reads like a method call (`text.shout()` is
`shout(text)` when `Str` has no member `shout`), but the function has to
exist.

**Generics.** The same applies to type parameters: a type parameter has no
bounds, so nothing is known about a `T`. Reading `value.name` inside
`fn f<T>(value: T)` is an error, not a promise about the values you will pass
in.

**Interop.** This is what makes interop itself a matter of *declaration*: a
Salvo program reaches its target language through a `platform handler`, a
host implementation of an ordinary Salvo effect whose member functions you
declare and the compiler turns into an interface for the host to implement
(see [Backends](Backends.md)). There is no way to name a Kotlin method or a
Rust function that some declaration in scope does not already stand for.

**What stays lenient.** The one thing that *is* lenient is a type the compiler
could not work out for itself: it stays unknown, compatible with everything,
so a single mistake produces a single error instead of a cascade of follow-on
complaints.

## Namespaces and namespaced names

A **`namespace X { … }`** groups declarations under a capitalized name. Any
top-level declaration can live in one (functions, types, structs, qualifiers,
effects, handlers, and further namespaces), and they nest to any depth. A
member `Thing` of `namespace X` is referred to as `X.Thing`, inside the
namespace as well as outside it:

```salvo
namespace Environment {
    struct Id { value: Str }
    struct Name { value: Str }
    qualifier Tag for Str

    fn tag(v: Str) -> +Environment.Tag Str => !v {
        return v
    }
}

let id = Environment.Id { value: "prod" }
let t = Environment.tag("x")   // a namespaced function, called by its path
```

This gives you the wrapper-type pattern (distinct types for the strings and
ids hanging off a value, so an incomplete refactor is a type error instead of
a silently mis-wired value) and keeps a family of related names out of the
module's top level.

`namespace` is a *contextual* keyword: it opens a grouping only before a
capitalized name, so you can still name a variable `namespace` or call a
function by that name everywhere else.

### A namespace beside a type

A namespace may **share its name with a type**, which is how a tagged union
keeps its arms under its own name:

```salvo
namespace AttributeValue {
    struct S { value: Str }
    struct N { value: Str }
    struct Null { value: Bool }
}

type AttributeValue = AttributeValue.S | AttributeValue.N | AttributeValue.Null
```

### The leading-dot shorthand

Inside a `when` (or any place the expected type is known) you don't have to
write the namespace: the **leading-dot shorthand** names an arm by a *suffix*
of its path, `.Thing` being the shortest:

```salvo
fn describe(v: AttributeValue) -> Str {
    when v {
        is .S { return "string ${v.value}" }
        is .N { return "number ${v.value}" }
        is .Null { return "null" }
    }
}
```

`.S` resolves because exactly one arm's type ends in `S`. A type
`A.B.C.Thing` can be written `.Thing`, `.C.Thing`, `.B.C.Thing` or in full;
you write just enough of the tail to be unambiguous. The shorthand works
wherever an expected type is in view: an `is` arm, a typed value, a call
argument, and a value returned from a function whose return type names the
namespace:

```salvo
fn s(v: Str) -> AttributeValue => !v { return .S { value: v } }
```

**Inside the alias itself**, there is no outer expected type to resolve
against (the declaration is naming itself), so the shorthand instead resolves
against the alias's *own* enclosing namespace:

```salvo
namespace Arg {
    struct Int {}
    struct Str {}
    struct Bool {}
    type Type = .Int | .Str | .Bool   // Arg.Int | Arg.Str | Arg.Bool
}
```

`.Int` is a sibling of `Type` itself. Only `Arg` is searched, not an
ancestor's ancestor, since the alias's own namespace is exact rather than a
path to walk upward from.

### Exporting, and the terminal form

`export namespace X { … }` exports every member at once; or put `export` on
individual members. A namespace is not itself a value you export: its members
are.

**The terminal form** `namespace X ...` (with a literal `...` where the `{`
would go) puts the rest of the file in `X`, the same shorthand body-less
constructs like `if` and `for` use:

```salvo
namespace Wire ...

struct Header { len: Int }   // Wire.Header
struct Body { bytes: Bytes } // Wire.Body
```

### A namespace beside a struct

A namespace may share its name with a struct too, so the wrapper types of a
value live under the value's own name. The struct may be generic; the
namespace's members are separate declarations and cannot see its type
parameters:

```salvo
struct Environment { id: Environment.Id, name: Environment.Name }

namespace Environment {
    struct Id { value: Str }
    struct Name { value: Str }
}
```

A `namespace` block is the only way to write a dotted name: `struct
Environment.Id { … }` is a syntax error. A declaration's own name is always
one segment, and the namespace around it supplies the rest.

**Nothing else visible may be called `NsName`**, the concatenation. The Rust
backend renders `Environment.Id` as `EnvironmentId`, so that spelling has to
stay free; declaring or importing an `EnvironmentId` beside `Environment.Id`
is an error.

### Importing a namespace

Import a single member, the whole namespace, or alias it:

```salvo
import env.types.Environment.Id   // just the member
import env.types.Environment      // the whole namespace: Environment.Id, .Name, .tag …,
                                  // and a struct or type named Environment with it
import env.types.Environment as Env   // aliased: Env.Id, Env.Name …
```

A whole-namespace import brings the members under the namespace's prefix (or
the alias's). It does *not* flood your scope with their bare names the way a
whole-*module* import does.

### How the backends spell them

Backends differ, deliberately. Kotlin nests *types* as `object`/`class` trees,
so `Outer.Inner.Thing` is the same path in the generated code, and flattens a
namespaced *function* to `Outer_Inner_make`. Rust flattens every dotted name
(`EnvironmentId`, `OuterInnerThing`), because Rust modules and structs share
one namespace and a `mod Environment` next to a `struct Environment` would not
compile.

## Documentation comments

Salvo has no separate doc-comment syntax. A `//` comment block sitting
directly above a declaration *is* that declaration's documentation: the
comment on the line immediately above it, plus every consecutive comment line
above that. One blank line ends the block, which is how you keep an unrelated
remark unrelated. A comment sharing its line with code documents nothing.

```salvo
// A person we know about.
//
// Only [name] is required; [surname] may be absent, and a
// [Person] is never partially built.
struct Person {
    // Their given name.
    name: Str,
    // Their family name, when we know it.
    //
    // Absent for people who go by one name.
    surname: Str? = None,
    age: Int
}

// Describes a [person] in one line.
//
// Reads [Person]'s fields directly:
//
// - `name` always
// - `surname` when present
fn describe(person: Person) -> Str {
    return person.name
}
```

Docs are **markdown**. The `//` and one following space come off; everything
after that is passed through, so emphasis, inline code, lists and fenced
blocks work as written, and a bare `//` line is a paragraph break.

`[symbol]` in a doc comment references a name: a parameter, generic or field
of the declaration being documented, or a type, qualifier, effect, handler or
function declared in the program. A reference that resolves becomes a link to
the declaration; one that does not is left exactly as written, so brackets in
prose are safe.

**Fields and members.** Structs document their fields individually: the
comment above a field belongs to that field, and tooling shows the struct's
own docs followed by a list of its fields. The same goes for anything else
declared inside a declaration: an effect's or handler's member functions, and
a handler's state.

### Tags

A line that starts with **`@name`** opens a **tag**: `@param name`,
`@returns`, `@throws`, `@see`, `@todo`, `@compilerrules`, `@compilerknown`,
and `@whynot` (for a predicate qualifier's `qualifies`, see
[Qualifiers](Qualifiers.md)). The prose comes first.

```salvo
// The words of a command line, split the way a shell splits them.
//
// @param line  the command line; quotes group words and a
//              backslash escapes the next character
// @returns     the words, quotes removed
// @see [split]
```

* A tag's text runs over the lines below it until the next tag, a bare `//`
  line or the end of the block, so it can wrap as far as it needs; pressing
  Enter in the editor lines the next line up under the text.
* Hover shows each kind as a section after the prose (**Parameters**,
  **Returns**, …).
* A tag the compiler does not know, or an `@param` naming no parameter, is a
  warning, never an error.
* `@compilerrules [rule-a, rule-b]` lists the compiler rules a declaration
  implements, optionally followed by how they apply there; it is for whoever
  reads the source, so hover leaves it out.
* `@compilerknown` marks a standard-library declaration the compiler refers to
  by name, so renaming it means changing the compiler too; hover leaves it out
  as well.

### Module documentation

**A module has documentation too**: the first comment run in the file, when it
documents no declaration. A blank line after it makes it the module's (the run
then sits above nothing), and so does the file's first `import` on the very
next line, since an import carries no docs of its own; the header-then-imports
layout needs no blank line. A run sitting directly above the first declaration
is that declaration's, and the module then has none. `[symbol]` references in
a module's docs resolve against its top-level declarations.

```salvo
// The shop's geometry: shapes and their areas.
//
// Start with [Shape].
import std.core.list.size

// A closed figure.
struct Shape { ... }
```

### Examples are checked

A fenced block in a doc comment is an **example**, and the compiler checks
it with the program, the way Rust's doctests are compiled. `salvo analyze`
and the language server report what one gets wrong, as a warning at the
comment line it is written on; hover and go-to-definition work inside it as
in any code; `salvo docs` prints the warnings of the examples it publishes.
`salvo build`, `run` and `test` never look at them.

````salvo
// The area of a square.
//
// ```
// let a = area(Square { side: 3 })    // 9
// println("${a}")
// ```
export fn area(s: Square) -> Int {
    return s.side * s.side
}
````

* **What an example is.** A block of statements is the body of a function
  that may print, `fn example() [use, Console] { … }`, with any `import`
  lines at its top kept above that function. A block with a declaration at
  its left edge (a `fn`, a `struct`, an `effect` …) is a file of its own, as
  written, and one with a `test` block is checked as a test.
* **What it sees.** Its own imports, `std.core`, and the module it
  documents. An example in the module's docs or on an exported declaration
  sees what a reader of the module can use, the exported names; one on a
  private declaration sees the module's private names too. Either way it is
  separate code: what it declares never becomes part of the module, and a
  name it declares that the module already has is a collision.
* **Opting out.** ```` ```no-check ```` is Salvo that is not checked, for an
  example that cannot stand alone (it needs two files, or a module no program
  may import). Any other language (```` ```text ````, ```` ```toml ````) is
  not Salvo and is never checked. An example is a block in a *doc comment*:
  one in a comment that documents nothing is not checked.
* **What is reported.** An example's errors, as warnings saying "in a doc
  example: …". Its own warnings are not: an example often binds a value
  only to show it.

### In the editor

The language server shows these on hover: for a declaration, for a *use* of
it, and for anything nested inside one. Hovering a field, wherever it is
written, shows that field's own documentation and says which struct declares
it. Hover also shows a variable's type as it is *known at the position you
hover*: narrowed by any `is` test or `when` arm you are inside, qualifiers
included, with the declared type named below when the two differ.

It adds a few things beyond the declaration text:

* A **`fns` group** hovers with its members, since those are the point of it.
* A **predicate qualifier** shows the condition it holds under when its
  `qualifies` is a single `return`: just the expression, so `Positive` reads
  as "Holds when `int > 0`." A longer body is hidden, and its doc comment
  explains it instead.
* A variable that **shares fate** with another says so, naming what it was
  derived from and where, down to the field (`p.name`, not all of `p`), with
  the reminder that reads are free, that moving or mutating it is rejected,
  and that `copy` makes an independent value.
* A function's hover says **where it came from**: the module of the overload
  that actually won, named the way an `@module` selector would spell it, since
  with scope-based overloading the signature alone does not tell you which
  `size` you are looking at. For a declaration from a dependency, it says which
  dependency.

Names reached through an `import` hover like local ones, on the import line
itself as well as at each use, as do the group name in a struct's obligation
clause (`: Show<self>`) and the qualifier in an `is` check (`i is Positive`).

**A module hovers wherever it is named**: the module part of an `import` line
(all of `import std.time`, the `shop.shapes` of `import shop.shapes.label`, as
one name however many segments), an `@module` selector (`size@list`,
`by @auto`), and the module a `by` site took its comptime fn from. The hover
is `module <full path>` (the full path, whatever suffix was written), then the
module's own documentation, then where it lives: the standard library, a
dependency by name, or the file. Go-to-definition on a module name opens its
file.

### Publishing them: `salvo docs`

`salvo docs` turns the doc comments into pages for a GitHub wiki: a page per
module, with its documentation, a contents table and every exported
declaration under its own heading; an **Index** page listing every documented
name A to Z; and, from the manifest's [`[docs]` section](Manifest.md#docs), a
home page and topic pages like these ones. The standard library's pages, such
as [`std.time`](../../lib/std/time.sv), are written this way.

Code in a doc comment is written in a plain fence (or one marked `salvo`) and
shows with Rust highlighting, which GitHub has and which fits Salvo closely. A
`[symbol]` becomes a link to the symbol's heading. `salvo docs --check` fails
when the pages on disk are not what it would write.
