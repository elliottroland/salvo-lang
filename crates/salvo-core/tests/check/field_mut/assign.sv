// [struct-mut] Only `Mut`-qualified struct values may have fields
// assigned; the checker enforces it at the assignment site.
struct Person canbe Mut {
    name: Str
}

fn bad() {
    let p = Person {name: "a"}
    //~ error[struct-mut:immutable-field-assign]
    p.name = "b"
}

fn good() -> Str {
    let p = Mut Person {name: "a"}
    p.name = "b"
    return copy(p.name)
}
