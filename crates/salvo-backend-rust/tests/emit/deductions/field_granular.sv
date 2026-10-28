// A field-granular mutation entry lets a disjoint field's derivation survive
// [deduce-field].

//--- a_surviving_field_derivation_renders_as_a_virtual_place
// [deduce-field] Rung ⑤ v1 — a **field-granular** mutation entry
// (`=> e.hp: Mut`) narrows the event a call produces, so a derivation of
// a *disjoint* field survives it by the ordinary overlap rule. The
// surviving binding cannot be a live borrow (the callee still takes the
// whole struct `&mut`), so it renders as a **virtual place**:
// re-materialized per use, which agrees with Kotlin because the field was
// untouched.
struct Ring { power: Int }
struct Entity canbe Mut {
    hp: Int,
    rings: Mut List<Ring>
}

fn damage(e: Mut Entity, n: Int) -> None => e.hp: Mut, n {
    e.hp = e.hp - n
}

fn main() [use] {
    use StdConsole()
    let e = Mut Entity { hp: 20, rings: Mut List(Ring { power: 3 }) }
    // No binding at all: the place is re-read where it is used.
    //~ !emits: let mut rings
    let rings = e.rings
    damage(e, 5)
    //~ emits: size_platform::<crate::Ring>(&e.rings)
    println("${size(rings)} ${e.hp}")
}
