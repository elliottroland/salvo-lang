// effect prerequisites
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [effect-prereq] `effect Files [Streams]`, a handler of `Files` reaching
// `Streams` without writing it, a fn declaring `[Files]` calling a
// `Streams` member from a module that never imports it, and a fn value of
// type `(Str) [Files] -> Str`.
//--- effect_prerequisites
// FILE: streams.sv
export effect Streams {
    fn next_id() -> Int
}

export handler Counter() of Streams {
    state {
        n: Int = 0
    }
    fn next_id() -> Int {
        n = n + 1
        return n
    }
}
// FILE: files.sv
import streams.Streams

export effect Files [Streams] {
    fn open(path: Str) -> Str => path
}

export handler MemFiles() of Files {
    fn open(path: Str) -> Str => path {
        return "${path}#${next_id()}"
    }
}
// FILE: main.sv
import files.Files
import files.MemFiles
import streams.Counter

fn work(f: (Str) [Files] -> Str) [Files] -> Str {
    return "${f("a")} ${next_id()}"
}

fn opener(p: Str) [Files] -> Str => p {
    return open(p)
}

fn main() [use] {
    use StdConsole()
    use Counter()
    use MemFiles()
    println(opener("x"))
    println(work(opener))
}
//= stdout
//= x#1
//= a#2 3
