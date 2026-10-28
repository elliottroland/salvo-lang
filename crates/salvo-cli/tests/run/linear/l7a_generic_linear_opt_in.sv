// L7a: generic linear opt-in [linear-generics]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- linear_generics

linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x { discard(x) }


fn open_file(n: Int) [Console] -> FileHandle {
    println("open ${n}")
    return FileHandle {fd: n}
}

fn hold<T canbe linear>(value: T) -> T => !value {
    return value
}

fn main() [use] -> None {
    use StdConsole
    let h = hold(open_file(9))
    println("held fd=${h.fd}")
    close(h)
    println("done")
}
//= stdout
//= open 9
//= held fd=9
//= done
