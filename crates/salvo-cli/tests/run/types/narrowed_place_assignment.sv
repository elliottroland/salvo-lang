// Assigning to a narrowed place [flow-place-invalidate].

//--- a_narrowed_field_is_assigned_at_its_declared_type
// Under `if p.surname is Str`, `p.surname = None` writes the field's declared
// `Str?`: the assignment ends the narrowing, as it does for a local. Found
// 2026-10-28: the assignment was refused as `None` into `Str`.
struct P canbe Mut {
    name: Str,
    surname: Str? = None
}

fn forget(p: Mut P) -> Str => p: Mut {
    if p.surname is Str {
        let was = "had ${p.surname}"
        p.surname = None
        return was
    }
    return "had none"
}

fn main() [use] -> None {
    use StdConsole()
    let p = Mut P { name: "Ada", surname: "Lovelace" }
    println(forget(p))
    println(forget(p))
    println("${p.surname ?: "gone"}")
}
//= stdout
//= had Lovelace
//= had none
//= gone
