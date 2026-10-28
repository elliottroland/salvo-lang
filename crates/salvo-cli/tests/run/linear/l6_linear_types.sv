// L6: linear types [linear-obligation]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- linear

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
    close(h)
}

fn main() [use] -> None {
    use StdConsole
    let h = open_file("data.txt")
    let n = h.fd
    println("fd=${n}")
    close_file(h)
    let temp = open_file("scratch")
    close(temp)
    let note = "note"
    discard(note)
    println("done")
}
//= stdout
//= open data.txt
//= fd=8
//= close fd=8
//= open scratch
//= done
