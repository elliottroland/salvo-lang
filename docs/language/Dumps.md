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

`--phase` names the phase. So far there is one, `resolve`. The IR, printed by
`salvo ir`, has its own page: [The IR](The-IR.md).

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
  the phase. The resolve phase's are below.

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
