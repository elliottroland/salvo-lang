// [struct-field-canbe-mut] `name: T canbe Mut` on a struct field: the field is
// `Mut T` exactly when the struct value is `Mut`, so a plain struct is
// immutable all the way down. What the checker refuses.

struct Bag canbe Mut {
    n: Int,
    items: List<Int> canbe Mut
}

// A `Mut` struct's field is `Mut`; a plain one's is not.
fn fill(b: Mut Bag) [] -> None => b: Mut {
    add(b.items, 1)
}

fn count(b: Bag) [] -> Int => b {
    return size(b.items)
}

fn make() [] -> Mut Bag {
    return Mut Bag { n: 0, items: Mut List<Int>() }
}

fn plain() [] -> Bag {
    return Bag { n: 0, items: List(1) }
}

fn a_plain_structs_field_is_not_mut(b: Bag) [] -> None => b {
    //~ error[call-resolve:no-overload]
    add(b.items, 1)
}

// A `Mut` literal needs `Mut` field values, and may not spread a plain value
// or rely on a (plain) default for a `canbe Mut` field.
struct Def canbe Mut {
    items: List<Int> canbe Mut = List<Int>()
}

fn copy_bag(p: Bag) [] -> Bag => p {
    return Bag { n: p.n, items: List<Int>() }
}

fn a_mut_literal_takes_mut_fields_only(p: Bag) [] -> None => p {
    //~ error[struct-decl:field-mismatch]
    let _w = Mut Bag { n: 1, items: List(1) }
    //~ error[struct-field-canbe-mut:plain-spread]
    let _m = Mut Bag { n: 1, ...copy_bag(p) }
    //~ error[struct-field-canbe-mut:default-not-mut]
    let _d = Mut Def {}
}

// Where `canbe Mut` on a field is refused: in a struct that is never `Mut`,
// over a type that cannot be, and over a type that already is.
struct Plain {
    //~ error[struct-field-canbe-mut:struct-not-canbe-mut]
    items: List<Int> canbe Mut
}

struct Bad canbe Mut {
    //~ error[struct-field-canbe-mut:not-applicable]
    a: Int canbe Mut,
    //~ error[struct-field-canbe-mut:already-mut]
    b: Mut List<Int> canbe Mut
}
