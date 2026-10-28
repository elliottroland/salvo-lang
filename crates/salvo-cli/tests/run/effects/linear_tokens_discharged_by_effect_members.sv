// linear tokens discharged by effect members [linear-group]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_linear_token_closed_by_a_member

// The phase-4 token shape in miniature: a linear token whose only discharger
// is an *effect member*, discharged inside the handler that implements it.
linear struct InTape canbe Mut { handle: Int }
linear struct OutTape canbe Mut { handle: Int }

effect Tape {
    fn open_read(path: Str) -> Mut InTape => path
    fn open_write(path: Str) -> Mut OutTape => path
    fn read_line(s: Mut InTape) -> Str => s: Mut
    fn write(s: Mut OutTape, text: Str) -> Int => s: Mut, text
    fn close(s: InTape) -> Str => !s
    fn close(s: OutTape) -> Str => !s
}

handler MemTape of Tape {
    fn open_read(path: Str) -> Mut InTape => path {
        return Mut InTape { handle: size(path) }
    }
    fn open_write(path: Str) -> Mut OutTape => path {
        return Mut OutTape { handle: size(path) }
    }
    fn read_line(s: Mut InTape) -> Str => s: Mut {
        s.handle = s.handle + 1
        return "line ${s.handle}"
    }
    fn write(s: Mut OutTape, text: Str) -> Int => s: Mut, text {
        s.handle = s.handle + size(text)
        return size(text)
    }
    fn close(s: InTape) -> Str => !s {
        discard(s)
        return "closed in"
    }
    fn close(s: OutTape) -> Str => !s {
        discard(s)
        return "closed out"
    }
}

fn main() [use] {
    use StdConsole()
    use MemTape()
    let r: Mut InTape = open_read("data.txt")
    println(read_line(r))
    println(close(r))
    let w: Mut OutTape = open_write("out.txt")
    let n = write(w, "hello")
    println("wrote ${n}")
    println(close(w))
}
//= stdout
//= line 9
//= closed in
//= wrote 5
//= closed out
