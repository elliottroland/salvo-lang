// Linear values and their dischargers.

//--- a_discharger_ends_the_value
// The resource pattern linearity exists for: open, use, close. The checker
// guarantees no path leaks the handle, and the discharger's body ending is
// the value's end [linear-group] — nothing is emitted for it. Plain: the
// program's module declares one `close`, and std's do not take part
// [fn-emit-name].
linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x {}


fn open_file(path: Str) [Console] -> FileHandle => !path {
    println("open ${path}")
    return FileHandle {fd: size(path)}
}

fn close_file(h: FileHandle) [Console] -> None => !h {
    println("close fd=${h.fd}")
    //~ emits: close(h)
    close(h)
}

fn main() [use] -> None {
    use StdConsole
    let h = open_file("data.txt")
    let n = h.fd
    println("fd=${n}")
    close_file(h)
    let temp = open_file("scratch")
    //~ emits: close(temp)
    close(temp)
    println("done")
}
