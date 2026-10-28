// `Mut` types: what a `canbe Mut` declaration's mutable form renders to.

//--- mut_types_map_onto_mutable_list
// [type-canbe-mut] `Mut List<T>` maps onto `MutableList<T>`;
// `Mut` on a type without `canbe Mut` is an error.
//~ emits: fun fill(target: salvo.platform.std.core.list.MutList<Int>, n: Int)
export fn fill(target: Mut List<Int>, n: Int) -> None => target: Mut {
    add(target, n)
}

export fn main() [use] -> None {
    use StdConsole
    //~ emits: val items: salvo.platform.std.core.list.MutList<Int> = mutableListOf<Int>(1)
    let items: Mut List<Int> = Mut List(1)
    fill(items, 2)
    println("size: ${items.size()}")
}
