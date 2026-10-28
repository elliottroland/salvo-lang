// Linear values and `discard`.

//--- discard_lowers_to_drop
// The resource pattern linearity exists for: open, use, close — with
// `discard` as the deliberate drop [linear-discard]. The checker
// guarantees no path leaks the handle; the demo verifies the lowering
// (Rust: `discard` lowers to `drop`).
// [linear-discard] `discard` lowers to `drop(...)` on the moved value.
// [linear-discard] A linear value's discharge is its own `close`; what
// `discard` still lowers to is `drop`, for the non-linear values it is
// now the escape hatch for. Plain: the program's module declares one
// `close`, and std's do not take part [fn-emit-name].
linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x { discard(x) }


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
    let note = "note"
    //~ emits: drop(note)
    discard(note)
    println("done")
}
