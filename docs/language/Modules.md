# Modules and files

The file extension for Salvo source code is `.sv`. Modules correspond to files, so that there is no need to specify the module name or path at the top of the file (as in Java and Kotlin).

The compiler discovers sources by walking the source directory recursively. Hidden directories and cache directories (those carrying a `CACHEDIR.TAG` marker, such as Cargo's `target/`) are skipped. A `.svignore` file at the source root excludes further paths: one path per line, relative to the root, naming a file or a whole directory subtree; blank lines and `#` comments are ignored.

Everything must be imported explicitly unless it is defined in the `core` module. This includes qualifiers, types, effects, handlers, and functions. When there is ambiguity in a reference (e.g. functions which have the same signature, qualifiers which apply to the same type), then they can be aliased using `as` in the import statement.

```
import random.Random // import Random<T> effect from random.sv
import list.add // Import that list's add function from list.sv
import list.ext.add as ext_add // Import the list extensions add function from list/ext.sv, which is now aliased
import core.Str // Not strictly necessary -- all of core is imported by default
```

A declaration is only importable if its own module lets it out. **Declarations are module-private by default, and `export` is how one becomes visible elsewhere:**

```
export fn label(n: Int) -> Str {   // callers in other modules may use this
    return "${n}"
}

fn digits(n: Int) -> Int {         // this module's business, and nobody else's
    return 1
}
```

The modifier comes first, before `intrinsic`, `linear`, `actor`, `platform`, `provenance`, `iter` and `send`, so a declaration reads *who can see it, what kind it is, what it is called*. It is an ordinary word rather than a reserved one, so a variable may still be called `export`; only the start of a declaration makes it a modifier.

It applies to the whole declaration and not its parts: an exported struct exports its fields, an exported effect its members, an exported handler its state. There is no field- or member-level visibility. What that leaves is one useful consequence worth knowing deliberately — a type you do not export may still appear in a signature you do, and callers can then hold values of it without being able to write its name. That is an opaque type, and it is the only way Salvo has to say it.

Privacy is per *module*, which is per file. Splitting a file in two is therefore a visibility decision: what the halves share has to be exported. The standard library is written this way, which is what it is for — `fire_after` and `earliest_due` in `time`, the `mem_*` helpers in the in-memory filesystem, are reachable from their own file and nowhere else.

The diagnostics are meant to send you to the right file. Using a private name says it is declared in module `m` and not exported, and names the fix; it is not offered as an import, since importing it could not work. An `import` of a private name is refused at the import line rather than silently importing nothing. And a name that is in scope but of the wrong *kind* — a qualifier where a type belongs — keeps its own diagnostic, because that is not a visibility problem.

A module can also be imported *whole*, which brings every name in it — and in every module beneath it — into scope with one line:

```
import time // Instant, Tick, Duration, Clock, Ticker, Timer, millis, between, ...
```

This is how a std surface that is not implicitly visible stays cheap to use: `time` is one import rather than a dozen. A whole-module import is deliberately the weakest way for a name to arrive. Your own module's declarations win over it, and so does a named import, both silently — so a file that declares its own `Duration` keeps it, and `import other.Duration` beside `import time` picks the other one. Functions do not compete at all: overloads from a bulk import simply sit lower on the scope ladder, and `between@time(a, b)` names the module explicitly. The one case that cannot be resolved by hand at the use site is two whole-module imports carrying the same *type* name, since a type reference has no module selector; there the first module wins and the compiler warns, naming the import that would settle it. A module import takes no `as` — there is nothing to qualify a renamed module with — and `import core` is redundant, since core is always visible.

Only the modules which are used in the code are transpiled to the relevant backend equivalent (modules in Rust, packages in Kotlin).

Every name written in a type position must resolve to a declaration in scope — base types (structs, `type` declarations and aliases, `intrinsic type`s, effects) and qualifiers alike. A name that resolves to nothing is an error naming the name, wherever it is written: a signature, a struct field, a `let` annotation, an `of` type, a `canbe` or `with` clause, or an `is` / `when` check. The diagnostic lists the modules that would bring the name into scope, if any.

This matters most for qualifiers, because a qualifier and a base type sit next to each other in the same syntax. `Ok Int` with no `Ok` in scope is not a type built from an unknown claim — it is a typo or a missing import, and it would otherwise fail much later and much less obviously, as a check that "can never succeed" against arms tagged with a qualifier the compiler never heard of. A name that *does* exist but in the other namespace says so, since no import can fix it:

```
qualifier Tag of Int

fn f(x: Tag) -> Int { ... }  // error: unknown type `Tag` (`Tag` is a qualifier, not a type)
```

Nothing is taken on trust. Besides written names, every *member* you reach
for must be justified by a declaration the compiler can see: a call to a
function nobody declared, a field on something that is not a struct (or a
field a struct does not have), `[]` on something that is not an array, and
`for` over something not iterable are all compile-time errors. There is no
"pass it through and let the target language sort it out" — a value's type
tells you exactly what you can do with it.

This is what makes interop itself a matter of *declaration*: a Salvo
program reaches its target language through a `platform effect` — or a
`platform handler`, a host implementation of an ordinary Salvo effect —
whose member functions you declare and the compiler turns into an interface
for the host to implement (see [Backends](Backends.md)) — there is no way to
name a Kotlin method or a Rust function that some declaration in scope does
not already stand for. Dot-notation still reads like a method call
(`text.shout()` is `shout(text)`), but the function has to exist. The same
applies to generics: a type parameter has no bounds, so nothing is known
about a `T` — reading `value.name` inside `fn f<T>(value: T)` is an error,
not a promise about the values you will pass in.

The one thing that *is* lenient is a type the compiler could not work out
for itself: it stays unknown, compatible with everything, so a single
mistake produces a single error instead of a cascade of follow-on
complaints.

## Naming rules

Casing is part of the language, not a convention:

* **Types start with an uppercase letter** — structs, qualifiers, type declarations and aliases, effects, handlers, and generic parameters.
* **Values do not** — functions, parameters, fields, variables, bindings, and lambda parameters.
* **Module paths are lowercase.** Since a module path is its file path, that applies to file and directory names too: `Utils.sv` is a compile-time error telling you to rename the file.
* **A file name may not contain a dot.** A module's path comes from where the file sits in the directory tree, and a dot in the name would read as a path separator that no directory backs — so `string.sv` is the module `string`, while `string.kotlin.sv` is rejected. There are no per-backend companion files; both targets are served from the one source.

This is what lets the compiler tell a type from a value at the start of a dotted name, which the next section relies on.

**Every name must be declared.** A reference to something nothing declares is an error, not a passthrough to the target language — the same rule as calls, field reads and subscripts. Scopes are not hoisted either, so reading a variable above its `let` is that same error. A name that *is* declared but is not a value says which it is:

```
let s = Person       // error: `Person` is a struct type, not a value:
                     //        construct one (`Name { … }`)
```

**A variable that is never read is a warning.** Assignment does not count as a read — a variable only ever written to has no reader, which is the mistake worth reporting. Prefix the name with `_` to say the omission is deliberate:

```
let spare = compute()    // warning: `spare` is never used;
                         //          prefix it with `_` (`_spare`) if that is deliberate
let _ignored = compute() // no warning
```

Parameters are exempt: a signature often dictates them, and a handler member implementing an effect cannot drop one.

## Namespaced names

Structs and qualifiers can be declared with a *dot-name* `Ns.Name`, where `Ns` is a struct in the same file. This gives you the wrapper-type pattern — distinct types for the strings and ids hanging off a struct, so that an incomplete refactor is a type error instead of a silently mis-wired value — without nesting declarations:

```
struct Environment {
    id: Environment.Id,
    name: Environment.Name
}

struct Environment.Id {
    value: Str
}

struct Environment.Name {
    value: Str
}

// Qualifiers can be namespaced too
qualifier Environment.Tag of Str
```

Dot-names work in every type position (annotations, `of` types, `is` checks, `as` constructors, deduction clauses) and in struct literals:

```
let env = Environment {
    id: Environment.Id {value: "prod"},
    name: Environment.Name {value: "Production"}
}
```

Three rules apply:

* `Ns` must be a struct **in the same file**, and it must not be generic.
* A dot-name has exactly two segments: `A.B.C` is an error, and a dot-named struct cannot itself be a namespace.
* Nothing else visible in the file may be called `NsName` — the concatenation. The Rust backend renders `Environment.Id` as `EnvironmentId`, so that spelling has to stay free.

Importing works either way: import the member directly, or import the namespace struct and get its members with it.

```
import env.types.Environment.Id     // just the member
import env.types.Environment        // the struct *and* Environment.Id
```

Backends differ, deliberately. Kotlin emits a nested class, so `Environment.Id` is the same name in the generated code. Rust concatenates, because Rust modules and structs share one namespace and a `mod Environment` next to a `struct Environment` would not compile.

## Documentation comments

Salvo has no separate doc-comment syntax. A `//` comment block sitting directly above a declaration *is* that declaration's documentation: the comment on the line immediately above it, plus every consecutive comment line above that. One blank line ends the block, which is how you keep an unrelated remark unrelated. A comment sharing its line with code documents nothing.

Docs are **markdown**. The `//` and one following space come off; everything after that is passed through, so emphasis, inline code, lists and fenced blocks work as written, and a bare `//` line is a paragraph break.

`[symbol]` in a doc comment references a name: a parameter, generic or field of the declaration being documented, or a type, qualifier, effect, handler or function declared in the program. A reference that resolves becomes a link to the declaration; one that does not is left exactly as written, so brackets in prose are safe.

Structs document their fields individually — the comment above a field belongs to that field, and tooling shows the struct's own docs followed by a list of its fields. The same goes for anything else declared inside a declaration: an effect's or handler's member functions, and a handler's state.

```
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

The language server shows these on hover — for a declaration, for a *use* of it, and for anything nested inside one: hovering a field, wherever it is written, shows that field's own documentation and says which struct declares it. It also shows a variable's type as it is *known at the position you hover* — narrowed by any `is` test or `when` arm you are inside, qualifiers included, with the declared type named below when the two differ.

Three things it adds beyond the declaration text. A **`params` group** hovers with its members, since those are the point of it. A **predicate qualifier** shows the condition it holds under when its `qualifies` is a single `return` — just the expression, so `Positive` reads as "Holds when `int > 0`." A longer body is hidden, and its doc comment explains it instead. And a variable that **shares fate** with another says so, naming what it was derived from and where, down to the field (`p.name`, not all of `p`) — with the reminder that reads are free, that moving or mutating it is rejected, and that `copy` makes an independent value. Names reached through an `import` hover like local ones, on the import line itself as well as at each use — as do the group name in a struct's obligation clause (`: Show<self>`) and the qualifier in an `is` check (`i is Positive`). A function's hover also says **where it came from** — the module of the overload that actually won, named the way an `@module` selector would spell it, since with scope-based overloading the signature alone does not tell you which `size` you are looking at.
