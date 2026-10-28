// ===== [comptime-kind] the kind is `.kind`, the type is `.type` =====
// Which copies a kind or type test keeps is comptime_tests.rs's.

struct P { a: Int, b: Int, c: Int, d: Str }

// [comptime-kind] `[when field.kind]` is the kind dispatch, exhaustive over
// `Kind`'s arms: the missing kinds are named.
struct Inner { n: Int }
struct Holder { i: Inner, m: Int }

comptime fn kinds_missing(h: Holder) [] -> Int => h {
    [for f in Holder.fields] {
        //~ error[when-exhaustive:kind-arms-missing]
        [when f.kind] {
            is Struct {}
        }
    }
    return 0
}

// [comptime-kind] A kind word where a type is compared names no type, and
// says so — in a test and in a `[when field.type]` alike.
comptime fn a_kind_word(p: P) [] -> Int => p {
    [for f in P.fields] {
        //~ error[comptime-kind:kind-as-type]
        [if f.type is Struct] {
            refuse!("${f.name}")
        }
    }
    return 0
}

comptime fn an_unknown_type(p: P) [] -> Int => p {
    [for f in P.fields] {
        //~ error[comptime-inline:unknown-type]
        [if f.type is Strr] {
            refuse!("${f.name}")
        }
    }
    return 0
}

// `[when x.type]` is the type dispatch now: a kind word there names no
// type, and says so.
struct Q { a: Int }

comptime fn a_kind_word_in_a_dispatch(p: Q) [] -> Int => p {
    [for x in Q.fields] {
        [when x.type] {
            //~ error[comptime-kind:kind-as-type]
            is Struct {}
            is * {}
        }
    }
    return 0
}

// ===== [comptime-generic-fn] an expanded-in-place fn may be generic =====

// [comptime-generic-fn] An expanded-in-place comptime fn may declare ordinary
// type parameters, and stays generic: it is expanded once, and the result is
// an ordinary generic fn, checked under the usual rules — a `${a.default}` of
// a `T?` field needs the `?to_str` a hand-written generic fn would (user
// decision 2026-10-20).
struct Arg<T> { name: Str, default: T? }

comptime fn show<T>(a: Arg<T>, ?to_str: (x: T) -> Str) [] -> Str => a {
    let out = ""
    [for f in Arg<T>.fields] {
        [if f.name is "default"] {
            if a.default !is None {
                out = "${out}${a.default}"
            }
        } else {
            out = "${out}${a.[f]}"
        }
    }
    return out
}

fn use_int(a: Arg<Int>) [] -> Str => a {
    return show(a)
}

fn use_bool(a: Arg<Bool>) [] -> Str => a {
    return show(a)
}

// Without the implicit, `T` has no text form — the ordinary rule.
comptime fn show_bare<T>(a: Arg<T>) [] -> Str => a {
    let out = ""
    [for f in Arg<T>.fields] {
        [if f.name is "default"] {
            if a.default !is None {
                //~ error[interp-to-str:no-text]
                out = "${out}${a.default}"
            }
        } else {
            out = "${out}${a.[f]}"
        }
    }
    return out
}

// [comptime-generic-fn] A generic struct is walked at an instance: a bare
// root has no arguments to substitute, and says what to write…
comptime fn count_bare<T>(a: Arg<T>) [] -> Int => a {
    let n = 0
    //~ error[comptime-generic-fn:uninstantiated]
    [for f in Arg.fields] {
        n = n + 1
    }
    return n
}

// …and an instance's arity is checked.
struct Box2<A, B> { a: A, b: B }

comptime fn count_short() [] -> Int {
    let n = 0
    //~ error[comptime-generic-fn:type-arg-count]
    [for f in Box2<Int>.fields] {
        n = n + 1
    }
    return n
}
