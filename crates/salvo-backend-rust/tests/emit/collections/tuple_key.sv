// A tuple key resolves the structural identity std declares for it
// [cmp-carry] [col-hashed-ordered].

//--- a_tuple_key_resolves_the_structural_identity
// [cmp-carry] The limitation the capability makes visible: a **tuple** key,
// refused with a Salvo diagnostic rather than a target-compiler one
// [backend-never-wrong]. Accepted deliberately (user, 2026-09-26): the lift
// is ROADMAP's "Recursive implicit resolution".
//
// [col-hashed-ordered] **Interim** (2026-09-28): a tuple or a list key
// resolves the structural identity std declares for it,
// so this container builds — the same on both backends for scalar
// elements. What is still not consulted is an element struct's *own*
// `cmp` inside a tuple; std owning these waits on ROADMAP §6.
//~ emits(main.rs): pub fn main()
fn main() [use] -> None {
    use StdConsole()
    let ns: SortedSet<(Int, Int)> = SortedSet((1, 2), (1, 2), (0, 5))
    println("${size(ns)}")
}
