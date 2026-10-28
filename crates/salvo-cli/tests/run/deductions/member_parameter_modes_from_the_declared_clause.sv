// member parameter modes from the declared clause [rs-borrows]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [rs-borrows] End to end: the modes have to agree across four renderers,
// and a program that moves a token into a member and mutates another
// through one is what proves they do. Byte-identical stdout on Kotlin.
//--- member_modes

struct Token canbe Mut { id: Int }

effect Sink {
    fn take(t: Token) -> Int => !t
    fn bump(t: Mut Token) -> None => t: Mut
}

handler Direct of Sink {
    fn take(t: Token) -> Int => !t {
        return t.id
    }
    fn bump(t: Mut Token) -> None => t: Mut {
        t.id = t.id + 1
    }
}

handler Doubling [Sink] of Sink {
    fn take(t: Token) -> Int => !t {
        return take(t) * 2
    }
    fn bump(t: Mut Token) -> None => t: Mut {
        bump(t)
        bump(t)
    }
}

fn main() [use] {
    use StdConsole()
    use Direct()
    use Doubling()
    let t: Mut Token = Mut Token { id: 1 }
    bump(t)
    println("bumped ${t.id}")
    println("took ${take(t)}")
}
//= stdout
//= bumped 3
//= took 6
