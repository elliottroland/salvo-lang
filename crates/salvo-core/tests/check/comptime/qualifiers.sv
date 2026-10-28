// ===== [comptime-qualifier] qualifiers on declarations, read by comptime =====
// What a test reads off them is comptime_tests.rs's; these are the refusals.

comptime qualifier Short(c: Char) of Field
comptime qualifier Positional of Field
comptime qualifier Named(name: Str) of Struct
qualifier Small(hi: Int) of Int {
    fn qualifies(n: Int, hi: Int) -> Bool {
        return n < hi
    }
}

// Serves.
Named("srv")
struct Serve {
    Short('p') port: Small(100) Int = 80,
    host: Str,
    Positional Short('f') files: List<Str>
}

// [comptime-qualifier] A constant binder is lowercase (an uppercase one would
// bind a type)…
comptime fn upper_binder() [] -> Int {
    [for f in Serve.fields] {
        //~ error[comptime-qualifier:type-binder]
        [if f is <C> Short] { }
    }
    return 0
}

// …binds nothing under `!`…
comptime fn binder_under_not() [] -> Int {
    [for f in Serve.fields] {
        //~ error[comptime-pattern:binder-under-or-not]
        [if !(f is <c> Short(c))] { }
    }
    return 0
}

// …and an arm carries no comptime qualifiers yet.
struct A { n: Int }
struct B { n: Int }
type U = A | B

comptime fn on_an_arm() [] -> Int {
    [for arm in U.arms] {
        //~ error[comptime-qualifier:on-arm]
        [if arm is Short] { }
    }
    return 0
}

// ===== [comptime-arm] building a union arm =====

// [comptime-arm] Each arm of a union is built in its copy: a struct arm by
// `[arm.type] { [for f in arm.type.fields] { [f]: … } }`, a qualified arm by
// `[arm.type](value)` — its claim's constructor, told the subject when the
// claim is generic — and `None` by `None`; calling a `None` arm is refused.
struct Two { n: Int, m: Int }
provenance qualifier Tag<T> of T
fn Tag<T>(value: T) [] -> +Tag T => !value {
    return value
}
type Pick = Two | Tag Int | None

comptime fn build(i: Int) [] -> Pick => i {
    [for arm in Pick.arms] {
        if i == arm.index {
            [if arm.kind is Struct] {
                return [arm.type] { [for f in arm.type.fields] { [f]: arm.index } }
            }
            [if arm.type is Tag *] {
                return [arm.type](i)
            }
            [if arm.type is None] {
                return None
            }
        }
    }
    return None
}

comptime fn build_none_by_call(i: Int) [] -> Pick => i {
    [for arm in Pick.arms] {
        if i == arm.index {
            [if arm.kind is Struct] {
                return [arm.type] { [for f in arm.type.fields] { [f]: arm.index } }
            }
            [if arm.type is Tag *] {
                return [arm.type](i)
            }
            [if arm.type is None] {
                //~ error[comptime-arm:none-called]
                return [arm.type](i)
            }
        }
    }
    return None
}

// ===== [comptime-literals] a literal union's members =====

// [comptime-literals] `[for lit in X.literals]` walks a literal union's
// members in declaration order — a field's type, a named alias, `None`
// skipped — with `lit.value` the member as a literal of the union (so it
// can be returned where the union is expected).
type Level = "debug" | "info"
struct Opts {
    output: ("json" | "text")?,
    level: Level,
    n: Int
}

comptime fn names() [] -> Int {
    [for lit in Opts.fields] { }
    [for lit in Level.literals] {
        let _k = 0
        [if lit.name is "info"] {
            let _j = 0
        }
    }
    return 0
}

comptime fn parse(s: Str) [] -> ("json" | "text")? => s {
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

// A type with no literals, and `.value` on another binder, are errors.
struct P { n: Int }

comptime fn no_literals() [] -> Int {
    [for f in P.fields] {
        //~ error[comptime-literals:not-literal-union]
        [for lit in f.type.literals] { }
        //~ error[comptime-literals:value-outside-literals]
        let v = f.value
    }
    return 0
}

//--- declarations and uses are checked
// [comptime-qualifier] A comptime qualifier's declaration and its uses are
// checked: its subject, its body, its constants, and where it is written —
// one error per misplaced qualifier.
comptime qualifier Short(c: Char) of Field
comptime qualifier Named(name: Str) of Struct
//~ error[comptime-qualifier:subject]
comptime qualifier Bad of Int
//~ error[comptime-qualifier:slot-type]
comptime qualifier Wide(x: List<Int>) of Field
qualifier Plain of Int

struct S {
    //~ error[qual-const:slot-mismatch]
    Short("p") a: Int,
    //~ error[comptime-qualifier:wrong-subject]
    Named("x") c: Int,
    //~ error[comptime-qualifier:not-comptime-annotation]
    Plain d: Int,
    //~ error[comptime-qualifier:in-type]
    e: Short('p') Int
}
