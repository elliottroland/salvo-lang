# Dumps

The compiler works in phases, and `salvo dump` prints what one phase made of
your program. A dump is how you check that the compiler read your code the
way you meant it. It is also where to look first when a later phase
surprises you, because each phase only reads what the one before it wrote.

```bash
salvo dump --phase resolve --src ./my_project                  # every module of the project
salvo dump --phase resolve --src ./my_project --module main    # one module
salvo dump --phase resolve --src ./my_project --all            # std too, every candidate listed
```

`--phase` names the phase, in the order the compiler runs them:
`parse-ast`, `resolve`, `type-check` and `generate-ir`. The IR, which
`generate-ir` prints, has its own page: [The IR](The-IR.md); `parse-ast`
prints the syntax tree in the compiler's own (Rust) debug form, for working
on the compiler itself.

A dump prints your program even when it has errors. They go to stderr, and the
dump shows what the phase made of the code anyway.

## How a dump names things

Every dump spells names the same way, and uses Salvo's own spelling wherever
Salvo has one.

* **A declaration is `name@module`**, the selector you would write in Salvo
  yourself: `show@app.svc`. An effect's member is `member@Effect`:
  `greet@Greeter`.
* **A type of your own carries its module**: `Point@app.shapes`. Another
  module may declare a `Point` too, and this says which one is meant. The
  standard library's own names stay bare (`Int`, `List<Int>`, `println`),
  because nothing else can mean them.
* **`#{…}` marks what Salvo has no spelling for.** What goes inside depends on
  the phase. Each phase's are below.

## The resolve phase

Resolution decides what each name you wrote refers to. It doesn't decide
types, and it doesn't choose between overloads; that is the type phase's job.
The resolve dump is your file's own text, with that decision written next to
every name:

```
import shapes.Point as P
import shapes.show
import shapes.describe

fn least<T>(a: T, b: T, ?Ordered<T>) -> Bool {
    return cmp#{9:26}(a#{9:13}, b#{9:19}) < 0
}

fn main() [use] {
    use StdConsole()
    let p: Point@shapes = Point@shapes { x: 1 }
    println(show@shapes(p#{15:9}))
    println(describe#{@main, @shapes}(1))
    let q: Pont#{???} = p#{15:9}
}
```

Here is how each kind of name reads:

| As written | In the dump | Means |
|---|---|---|
| `P` (an import alias) | `Point@shapes` | the declaration it imports, under its own name |
| `show` | `show@shapes` | the one module whose `show` the call can mean |
| `describe` | `describe#{@main, @shapes}` | both modules declare one, and the type phase picks between them by the arguments; past three the list ends `… N more`, and `--all` prints it in full |
| `p` | `p#{15:9}` | the variable bound at line 15, column 9 |
| `cmp` | `cmp#{9:26}` | a member of the `?Ordered<T>` written at line 9, column 26 |
| `Pont` | `Pont#{???}` | nothing of that name is visible here |
| `.Int` | `.Int#{deferred}` | a leading-dot name, which only the type phase can resolve |
| `now` (inside a lambda) | `now#{deferred}` | nothing written answers it, but the lambda's fn type, which the type phase reads, may make an effect available that does |
| `xs` in `i: Idx(xs) Int` | `Idx(xs#{3:6}) Int` | a place in a qualifier's value block is a variable, read like any other; a constant there (`InRange(0, 9)`, `Short('v')`) is left as written |
| `cmp@Person` | `cmp@Person#{app.main}` | a selector naming a type gets the type's module |

These names are left exactly as written:
- generics (`T`);
- the compiler's own names (`None`, `Mut`, `proj`);
- declarations' own names;
- a name you selected yourself, such as `describe@main`;
- anything the compiler wrote for you, such as the body an `iter fn` expands
  into.

An effect member is found wherever the body can call it from what is
written:
- the fn's effect list, with its aliases and every effect's prerequisites;
- a fn-typed parameter's effects, when the fn writes no list;
- a handler member's dependencies;
- the faces of a handler the body `use`s by name;
- the module's own `use`s.

So `[Time]`, an alias of `Clock` from another module, makes `now()` read
`now@Clock` even though `Clock` itself was never imported. The type phase
decides only where a type says what is available: a `use` of a handle
rather than a named handler, or a lambda's body. A name nothing else answers
there reads `#{deferred}`.

## The type-check phase

Type checking decides everything resolution left open: which overload each
call means, the type arguments it was called at, what fills each implicit
parameter, what each operator and interpolated value calls, and the type of
everything you left unannotated. The type-check dump is your file's own text
with each of those decisions written out as Salvo, so it reads as the program
you would have written had you spelled everything yourself:

```bash
salvo dump --phase type-check --src ./my_project                 # the decisions
salvo dump --phase type-check --src ./my_project --expr-types    # and every expression's type
salvo dump --phase type-check --src ./my_project --rows          # the checker's facts, node by node
```

For this program:

```
struct Box<T> {
    v: T
}

fn least<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Bool {
    return cmp(a, b) < 0
}

fn f(x: Int?, n: Long) -> Int {
    let b = Box { v: 1 }
    let s = (2 + 3) * b.v
    if x is Int {
        return s + x
    }
    for y in range(0, 2) {
        let z = "${y + 1}"
    }
    let w = n + 1
    return s
}

fn g() -> Bool {
    return least(1, 2)
}
```

the dump reads:

```
struct Box<T> {
    v: T
}

fn least<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Bool {
    return cmp@std.core.basic(cmp(a, b), 0) < 0
}

fn f(x: Int?, n: Long) -> Int {
    let b: Box@main<Int> = Box<Int> { v: 1 }
    let s: Int = times@std.core.basic((plus@std.core.basic(2, 3)), b.v)
    if x is Int {
        return plus@std.core.basic(s, x#{narrowed from Int?})
    }
    for y in range@std.core.range(0, 2) #{next@std.core.range} {
        let z: Str = "${to_str@std.core.basic(plus@std.core.basic(y, 1))}"
    }
    let w: Long = plus@std.core.basic(n, 1#{as Long})
    return s
}

fn g() -> Bool {
    return least@main<Int>(1, 2, cmp = cmp@std.core.basic)
}
```

What it writes out:

| Decision | In the dump |
|---|---|
| the overload a call means | its module after the name: `least@main(…)`, a member `now@Clock()` |
| the type arguments a call inferred | `least@main<Int>(…)` |
| what filled an implicit parameter | the argument by name: `cmp = cmp@std.core.basic` |
| an operator | the call it names: `plus@…(a, b)` for `a + b`, `eq@…(a, b)` for `==`, `!eq@…(a, b)` for `!=`, `cmp@…(a, b) < 0` for `<` (your parentheses stay) |
| an interpolated value | its `to_str` call, inside the `${}` |
| the type of an unannotated `let` | `let s: Int = …` |
| a lambda's parameter and return types | `(a: Int) -> Int { … }` |
| a generic struct literal's type arguments | `Box<Int> { v: 1 }` |

std's callees are written with their module too (`plus@std.core.basic`), since
the point is to see which one was meant. Types are spelled as everywhere else
in the dumps: a type of your own with its module.

Some decisions have no Salvo spelling, and are marked after the expression
they are about:

| Mark | Means |
|---|---|
| `#{as T}` | the value is converted to `T` here: wrapped into a union, widened to a wider number, its `Mut` dropped |
| `#{narrowed from T}` | the read sees a narrower type than its variable is stored at (`T`), because a test said so |
| `#{claimed from T}` | a field read at the type a qualifier's claim refines it to |
| `#{next@…}` | the calls a `for` makes (`#{iter@…, next@…}` for a source) |
| `#{: T}` | with `--expr-types`: the expression's type |

The compiler checks that this dump says what it means: for the examples, it
writes each program out this way (marks left out), checks the result again,
and requires the same decisions and the same types. A rule that decided
differently on the spelled-out form than on what you wrote would show up
there.

`--rows` prints the same facts as a table instead of your text: per
declaration, one line per expression, with its type, what a call resolved
to, and marks for moves, narrowings, effects, tests, handles,
constructions and borrows.
