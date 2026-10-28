// Implicit positions that take `Mut` [implicit-group].

//--- a_mut_implicit_position_is_not_borrowed_twice
// The adapter for a `Mut` implicit position passes the parameter straight
// through: it arrives as `&mut T` already.
struct Slice<T> : Next<self, proj T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> Mut Slice<T> => items {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> Emitted (proj(p) T) | Finished => p: Mut {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(e)
}

// The spread position is a `&mut`-taking, union-returning fn.
//~ emits(main.rs): next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>
fn map2<It, T, U>(it: Mut It, mapper: (T) -> U, ?Next<It, T>) -> Mut List<U> => it: Mut, mapper {
    let out = Mut List<U>()
    let going = true
    while going {
        let step = next(it)
        when step {
            is Emitted {
                add(out, mapper(step))
            }
            is Finished {
                going = false
            }
        }
    }
    return out
}

fn double(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
    //~ emits(main.rs): &mut |__a0: &mut crate::Slice<'_, i32>| (match crate::next(&mut *__a0) {
    let doubled = map2(slice(xs), double)
    for d in doubled {
        println("d ${d}")
    }
}
