// [net-wire-format] [net-noremote] What may cross a machine, as the checker
// sees it — step ② of the network sequence (user decisions 2026-09-26).
// Every type is serializable unless it is, or holds, something `noremote`;
// `encode`/`decode` are refused at the call, naming what stops them.
import std.codec

// [net-wire-format] The default: a struct of scalars, strings, lists, options
// and unions has a wire form, and so do `encode` and `decode` of it.
struct Tag { name: Str }
struct Pin { x: Int, label: Str?, tags: List<Tag>, kind: Tag | Int | None, raw: Bytes }

fn ordinary() -> None {
    let p = Pin { x: 1, label: None, tags: List<Tag>(), kind: 2, raw: encode(3) }
    let b = encode(p)
    let _back = decode<Pin>(b)
}

// [net-noremote] A `noremote` struct is refused at the `encode`, by name.
noremote struct Canvas { n: Int }

fn noremote_struct() -> None {
    //~ error[net-noremote:no-wire-form]
    let _b = encode(Canvas { n: 1 })
}

// [net-noremote] Transitive: a struct holding a `noremote` field is refused, and
// the message names the field that stops it.
struct Frame { title: Str, canvas: Canvas }

fn transitive() -> None {
    //~ error[net-noremote:no-wire-form]
    let _b = encode(Frame { title: "t", canvas: Canvas { n: 1 } })
}

// [net-noremote] std's process-local handles are `noremote` by declaration
// (`Pool` here), so a struct holding one has no wire form.
struct Placement { where: Pool }

fn pooled() [spawn] -> None {
    //~ error[net-noremote:no-wire-form]
    let _b = encode(Placement { where: pool(2) })
}

// [net-noremote] A function value has no wire form, by construction.
struct Rule { test: (Int) -> Bool }

fn fn_value() -> None {
    //~ error[net-noremote:no-wire-form]
    let _b = encode(Rule { test: n -> n > 0 })
}

// [net-wire-format] Step ②'s recorded cuts, each refused with its reason: a keyed
// container (identity capabilities not yet carried), and a generic `T`
// (the form is the instantiation's).
fn ship<T>(v: T) -> Bytes => !v {
    //~ error[net-noremote:no-wire-form]
    return encode(v)
}

fn keyed() -> None {
    //~ error[net-noremote:no-wire-form]
    let _b = decode<Set<Int>>(encode(1))
}
