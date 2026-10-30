// [comptime-hygiene] A template's names mean what they mean where it is
// written (user decision 2026-10-30). A fn name its file can see is refused
// bare — the template says `name@module` for that one, or `name@[site]` for
// the stamping site's — and one it cannot see is the site's. Types, effects
// and qualifiers are the template's, and a site's same-named one does not
// capture them.
// FILE: deriv.sv
export fns Tagged<T> {
    fn tag(value: T) -> Int
}

struct Tag {
    n: Int
}

fn weight(t: Tag) [] -> Int => t {
    return t.n
}

export comptime fn tag<T is Struct>(value: T) [] -> Int => value {
    let t = Tag { n: 1 }
    let total = weight@deriv(t) + size@list(List(1, 2))
    [for field in T.fields] {
        let v: [field.type] = value.[field]
        total = total + rank@[site](v) + bonus(v)
    }
    //~ error[comptime-hygiene:ambiguous-name]
    return total + weight(t)
}
// FILE: main.sv
import deriv

// The site's own `Tag` and `weight` mean nothing to the template.
struct Tag {
    s: Str
}

fn weight(t: Tag) [] -> Int => t {
    return 0
}

struct Inner {
    k: Int
}

fn rank(i: Inner) [] -> Int => i {
    return i.k
}

fn bonus(i: Inner) [] -> Int => i {
    return 1
}

struct Point : Tagged<self> by deriv {
    inner: Inner
}

//--- site_outside_a_template
// [comptime-hygiene] An expanded-in-place comptime fn is stamped nowhere, so
// it has no `@[site]`.
struct Pt {
    x: Int
}

comptime fn total(p: Pt) [] -> Int => p {
    let n = 0
    [for field in Pt.fields] {
        //~ error[comptime-hygiene:site-in-place]
        n = n + cmp@[site](p.[field], 0)
    }
    return n
}

//--- site_outside_a_comptime_fn
// [comptime-hygiene] `@[site]` is comptime syntax.
fn f(x: Int) [] -> Int {
    //~ error[comptime-inline:outside-compfn]
    return cmp@[site](x, 0)
}

//--- a_place_that_is_not_site
// [comptime-hygiene] The one comptime place is `site`.
comptime fn g<T is Struct>(v: T) [] -> Int => v {
    //~ error[comptime-hygiene:unknown-place]
    return cmp@[here](1, 0)
}

//--- a_pattern_names_a_declaration
// [comptime-pattern] [comptime-hygiene] A type pattern names a declaration as
// the template's file sees it, and a field's type is its declaration's: the
// site's `Item` is not `deriv`'s, so the branch that passes the field to
// `deriv`'s `weight` is not kept (until 2026-10-30 both were "`Item`", the
// branch was kept, and the call did not check).
// FILE: deriv.sv
export fns Weighed<T> {
    fn weigh(value: T) -> Int
}

struct Item {
    n: Int
}

fn weight(i: Item) [] -> Int => i {
    return i.n
}

export comptime fn weigh<T is Struct>(value: T) [] -> Int => value {
    let total = 0
    [for field in T.fields] {
        [if field.type is Item] {
            total = total + weight@deriv(value.[field])
        }
    }
    return total
}
// FILE: main.sv
import deriv

struct Item {
    s: Str
}

struct Box : Weighed<self> by deriv {
    item: Item
}

//--- a_spliced_type_means_its_declarations_name
// [comptime-hygiene] [type-identity] A type the expansion splices in reads
// where it was written: `Pt`'s field type `Inner` is `geo`'s, though the
// stamping file never imports it (until 2026-10-30 it resolved at the site,
// where `Inner` is unknown).
// FILE: geo.sv
struct Inner {
    k: Int
}

export struct Pt {
    inner: Inner
}
// FILE: deriv.sv
export comptime fn fields<T is Struct>(value: T) [] -> Int => value {
    let total = 0
    [for field in T.fields] {
        let _v: [field.type] = value.[field]
        total = total + 1
    }
    return total
}
// FILE: main.sv
import geo.Pt
import deriv

fn fields(p: Pt) -> Int by deriv
