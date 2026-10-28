// [linear-union-arm] the fallible-open shape, end to end
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_linear_union_arm

linear struct InputStream {
    path: Str
}

fn close(s: InputStream) [Console] -> None => !s {
    println("closed ${s.path}")
    discard(s)
}

fn open(path: Str) -> Ok InputStream | Err Str
=> !path {
    if size(path) > 0 {
        return ok(InputStream {path: path})
    }
    return err("empty path")
}

fn attempt(path: Str) [Console] -> None
=> !path {
    let r = open(path)
    when r {
        is Ok {
            println("opened")
            close(r)
        }
        is Err {
            println("error: ${r}")
        }
    }
}

fn main() [use] {
    use StdConsole()
    attempt("a.txt")
    attempt("")
}
//= stdout
//= opened
//= closed a.txt
//= error: empty path
