// [name-resolve] Written names in type positions resolve to declarations.

// [name-resolve] An undeclared qualifier in an `is` check is reported as the
// unresolved name it is — not as the arm mismatch it causes. Before this
// rule the same program said "this check can never succeed" (the name was
// silently treated as a base type, which no arm matched) and then cascaded a
// bogus consumed-value error on the subject. The return type mentions
// `Broken` too, and that site reports it as an unknown qualifier. (std
// declares `Ok` and `Err`, so the case's own qualifier is `Fine` and the
// unknown one `Broken`.)
qualifier Fine<T> of T

fn fine<T>(value: T) [] -> +Fine T => !value {
    return value
}

//~ error[name-resolve:unknown-type]
fn produce() -> Fine Int | Broken Str {
    return fine(1)
}

fn checks_an_unknown_name() -> Int {
    let v = produce()
    //~ error[name-resolve:unknown-type-or-qualifier]
    if v is Broken {
    }
    return 1
}

// [name-resolve] A `when` branch naming something unresolved reports the
// name; the arms it fails to match must not also surface as a non-exhaustive
// `when`.
qualifier Sour<T> of T

fn produce_known() -> Fine Int | Sour Str {
    return fine(1)
}

fn branches_on_an_unknown_name() -> Int {
    let v = produce_known()
    when v {
        is Fine {
            1
        }
        //~ error[name-resolve:unknown-type-or-qualifier]
        is Nope {
            2
        }
    }
    return 1
}

// [name-resolve] A name that exists in the *other* namespace is a position
// mistake: say so rather than suggesting an import that cannot help.
qualifier Tag of Int

//~ error[name-resolve:unknown-type]
fn tag_as_type(x: Tag) -> Int {
    return 1
}

struct S {
    v: Int
}

//~ error[name-resolve:unknown-type]
fn struct_as_qualifier(x: S Int) -> Int {
    return 1
}

// [name-resolve] The compiler's intrinsic qualifiers have no declaration to
// find, so they must not be reported as unknown; their own rules still
// apply.
fn intrinsic_qualifier(s: Mut List<Int>) -> Int {
    return 1
}

// [qual-canbe-optin] `canbe` grants one of the compiler's own qualifiers; a
// user qualifier there is a confusion with `with` [qual-with].
//~ error[qual-canbe-optin:unknown]
struct Tagged canbe Tag {
    v: Int
}

//--- a_callee_type_means_its_writers
// FILE: svc.sv
// [name-table] [type-identity] A written type name means what it means where
// it is written: `show`'s `Item` is this module's struct wherever `show` is
// called. A caller with a `fns` group of that name lowered it as the group
// (to `Unknown`), so `show(5)` passed and rustc refused the output (found
// 2026-10-29, PIPELINE.md C1.2).
export struct Item {
    n: Int
}

export fn make() -> Item {
    return Item { n: 1 }
}

export fn show(i: Item) -> Str {
    return "${i.n}"
}
// FILE: main.sv
import svc.make
import svc.show

fns Item<T> {
    fn size(x: T) -> Int
}

fn main() [use] {
    use StdConsole()
    println(show(make()))
    //~ error[call-resolve:no-overload]
    println(show(5))
}
