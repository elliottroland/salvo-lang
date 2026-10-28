// [effect-at] [effect-member-overload] shared member names
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- effect_selectors

effect Store {
    fn open(path: Str) -> Int => path
    fn close(handle: Int) -> Str
}

effect Net {
    fn close(handle: Int) -> Str
}

handler MemStore of Store {
    fn open(path: Str) -> Int => path {
        return 7
    }
    fn close(handle: Int) -> Str {
        return "fs closed ${handle}"
    }
}

handler MemNet of Net {
    fn close(handle: Int) -> Str {
        return "net closed ${handle}"
    }
}

fn shut(h: Int) [Store] -> Str {
    return close(h)
}

fn main() [use] -> None {
    use StdConsole
    use MemStore()
    use MemNet()
    let h = open("a.txt")
    println(shut(h))
    println(close@Store(h))
    println(close@Net(9))
    println(h.close@Net())
}
//= stdout
//= fs closed 7
//= fs closed 7
//= net closed 9
//= net closed 7
