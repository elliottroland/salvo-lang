// comptime: qualifiers, arms, literals, stamping and defaults
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [comptime-qualifier] Comptime qualifiers end to end: a comptime fn reads
// a struct's own (`Named`), each field's (`Short('p')`, `Positional`) and a
// field type's qualifier constants, and the struct itself compiles and runs
// with the qualifiers erased.
//--- comptime_qualifiers
comptime qualifier Short(c: Char) of Field
comptime qualifier Positional of Field
comptime qualifier Named(name: Str) of Struct

Named("srv")
struct Serve {
    Short('p') port: InRange(1, 65535) Int = 8080,
    host: Str = "127.0.0.1",
    Positional files: List<Str>
}

comptime fn help() [] -> Str {
    let out: Mut Str = Mut Str("")
    [if Serve is <n> Named(n)] {
        append(out, "command ${n}\n")
    }
    [for f in Serve.fields] {
        [if f is <c> Short(c)] {
            append(out, "-${c} ")
        }
        [if f is Positional] {
            append(out, "<${f.name}>")
        } else {
            append(out, "--${f.name}")
        }
        [if f.type is <lo, hi> InRange(lo, hi) *] {
            append(out, " (${lo}..${hi})")
        }
        append(out, "\n")
    }
    return out
}

fn main() [use] -> None {
    use StdConsole()
    print(help())
    let s = Serve { files: List("a") }
    println("${s.port} ${s.host}")
}
//= stdout
//= command srv
//= -p --port (1..65535)
//= --host
//= <files>
//= 8080 127.0.0.1

// [comptime-arm] A comptime fn builds every kind of union arm — a struct arm
// field by field, a qualified arm through its claim's constructor, `None` —
// and the values run identically on both backends.
//--- comptime_builds_each_arm
struct A { n: Int, s: Str }
struct B { m: Int }
type Pick = A | B | Ok Str | None

fn make() [] -> Int {
    return 7
}

fn make() [] -> Str {
    return "s"
}

comptime fn build(i: Int) [] -> Pick => i {
    [for arm in Pick.arms] {
        if i == arm.index {
            [if arm.kind is Struct] {
                return [arm.type] { [for f in arm.type.fields] { [f]: make() } }
            }
            [if arm.type is Ok *] {
                return [arm.type](make())
            }
            [if arm.type is None] {
                return None
            }
        }
    }
    return None
}

fn show(p: Pick) [] -> Str => p {
    when p {
        is A { return "A ${p.n} ${p.s}" }
        is B { return "B ${p.m}" }
        is Ok { return "Ok ${p}" }
        is None { return "None" }
    }
}

fn main() [use] -> None {
    use StdConsole()
    for i in List(0, 1, 2, 3) {
        println(show(build(i)))
    }
}
//= stdout
//= A 7 s
//= B 7
//= Ok s
//= None

// [comptime-literals] A literal union's members walked in comptime: listed
// with their positions, and matched to decode a string into the union.
//--- literal_members
type Level = "debug" | "info" | "warn"

struct Opts {
    output: "json" | "text" = "text",
    level: Level = "info",
    n: 1 | 2 | 3 = 1
}


comptime fn choices() [] -> Str {
    let out: Mut Str = Mut Str("")
    [for f in Opts.fields] {
        append(out, "${f.name}:")
        [for lit in f.type.literals] {
            append(out, " ${lit.value}")
            [if lit.last] {
                append(out, " (${lit.index + 1})")
            }
        }
        append(out, "\n")
    }
    [for lit in Level.literals] {
        [if lit.first] {
            append(out, "first ${lit.value}\n")
        }
    }
    return out
}


comptime fn parse_output(s: Str) [] -> ("json" | "text")? => s {
    [for f in Opts.fields] {
        [if f.name is "output"] {
            [for lit in f.type.literals] {
                if s == lit.value {
                    return lit.value
                }
            }
        }
    }
    return None
}

fn main() [use] -> None {
    use StdConsole()
    print(choices())
    let picked = parse_output("json")
    if picked !is None {
        let o = Opts { output: picked }
        println("${o.output} ${parse_output("xml") is None}")
    }
}
//= stdout
//= output: json text (2)
//= level: debug info warn (3)
//= n: 1 2 3 (3)
//= first debug
//= json true

// [comptime-hygiene] A stamped body resolves a name the stamping site cannot
// in the template's own file, private declarations included (user decision
// 2026-10-20, ROADMAP §2d item 10 (b)): `deriv`'s private `shout` serves
// `Point`'s stamped `label`, though `main` cannot see it.
//--- a_stamped_body_falls_back_to_its_templates_names
// FILE: deriv.sv
export fns Labelled<T> {
    fn label(value: T) -> Str
}

export comptime fn label<T is Struct>(value: T) [] -> Str => value {
    return shout(T.name)
}

fn shout(s: Str) [] -> Str => s {
    return "${s}!"
}
// FILE: main.sv
import deriv

struct Point : Labelled<self> by deriv {
    x: Int
}

fn main() [use] -> None {
    use StdConsole()
    println(label(Point { x: 1 }))
}
//= stdout
//= Point!

// [comptime-hygiene] A name the site does resolve is the site's: with its
// own `shout`, `main`'s is called, and the two never meet in an ambiguity
// the site did not write.
//--- a_stamped_body_prefers_the_sites_names
// FILE: deriv.sv
export fns Labelled<T> {
    fn label(value: T) -> Str
}

export comptime fn label<T is Struct>(value: T) [] -> Str => value {
    return shout(T.name)
}

fn shout(s: Str) [] -> Str => s {
    return "${s}!"
}
// FILE: main.sv
import deriv

struct Point : Labelled<self> by deriv {
    x: Int
}

fn main() [use] -> None {
    use StdConsole()
    println(label(Point { x: 1 }))
}

fn shout(s: Str) [] -> Str => s {
    return "mine: ${s}"
}
//= stdout
//= mine: Point

// [obligation-return] A decoder stamped by an obligation clause: two structs
// each get `from_strs(args: List<Str>) -> S`, which the expected type
// chooses between (each field read with `from_str`, chosen by the field's
// type).
//--- a_stamped_decoder
// FILE: decode.sv
export fns FromStrs<T> {
    fn from_strs(args: List<Str>) -> T
}

export comptime fn from_strs<T is Struct>(args: List<Str>) [] -> T => args {
    return T { [for f in T.fields] { [f]: from_str(get(args, f.index) ?: unreachable!()) ?: unreachable!() } }
}
// FILE: main.sv
import decode

struct Serve : FromStrs<self> by decode {
    port: Int,
    host: Str
}

struct Stop : FromStrs<self> by decode {
    force: Bool
}

fn main() [use] -> None {
    use StdConsole()
    let s: Serve = from_strs(List("80", "web"))
    let t: Stop = from_strs(List("true"))
    println("${s.port} ${s.host} ${t.force}")
}
//= stdout
//= 80 web true

// [default-scope] [comptime-default] A default is evaluated where it is
// declared: a struct field's and a parameter's default that call a private
// fn of their module work when applied in another module (they were an
// "unresolved identifier" in the IR, which looked the default's facts up
// under the using file), and a comptime walk in another module reads each
// field's default with `field.default` — the declared value, tested
// against its predicate qualifier — or `None` where there is none.
//--- defaults_are_evaluated_where_they_are_declared
// FILE: lib/conf.sv
fn base_port() [] -> Int {
    return 8000
}

fn suffix() [] -> Str {
    return "!"
}

export fn wave(name: Str, punct: Str = suffix()) [] -> Str => name, punct {
    return "bye ${name}${punct}"
}

export struct Conf {
    host: Str = "127.0.0.1",
    port: InRange(1, 65535) Int = base_port() + 80,
    name: Str,
    tag: Str? = None
}
// FILE: main.sv
import lib.conf

fn show<T>(x: T?, ?to_str: (x: T) -> Str) [] -> Str => x {
    if x is None {
        return "(none)"
    }
    return to_str(x)
}

comptime fn defaults() [] -> Str {
    let out: Mut Str = Mut Str("")
    [for f in Conf.fields] {
        [if f.default is None] {
            append(out, "${f.name}: required\n")
        } else {
            let d: [f.type]? = f.default
            append(out, "${f.name}: ${show(d)}\n")
        }
    }
    return out
}

fn main() [use] -> None {
    use StdConsole()
    let c = Conf { name: "x" }
    println("${c.port} ${wave(c.host)}")
    print(defaults())
}
//= stdout
//= 8080 bye 127.0.0.1!
//= host: 127.0.0.1
//= port: 8080
//= name: required
//= tag: (none)
