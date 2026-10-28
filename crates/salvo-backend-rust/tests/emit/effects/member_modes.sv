// Effect members: overloads within one effect [effect-member-overload], and
// parameter modes from the declared clause [rs-borrows].

//--- member_overloads_get_distinct_names
// [effect-member-overload] The phase-4 shape: `Fs` declares `close` once per
// stream token (FILE_SYSTEM.md §5.10.2 sub-question A). Rust cannot overload
// a trait method at all, so the names have to be made distinct — by
// `salvo_core`'s rule, so the Kotlin backend picks the same ones.
//
// [effect-member-overload] [rs-effects] The trait declares the overloads
// under distinct names — the first keeps the plain one — the handler's impl
// uses those same names, and each call site emits the one the *checker*
// resolved.
struct InFile { id: Int }
struct OutFile { id: Int }

// The trait: each `close` suffixed by its parameter's type
// [fn-emit-name], and the un-overloaded member keeps its own name.
// [rs-borrows] The **consuming** overloads take their parameter by
// value (`=> !f`) while the keeping `describe` borrows — a member's
// written clause is its whole contract.
//~ emits: fn close__InFile(&mut self, f: crate::InFile) -> String;
//~ emits: fn close__OutFile(&mut self, f: crate::OutFile) -> String;
//~ emits: fn describe(&mut self, f: &crate::InFile) -> String;
effect Vault {
    fn close(f: InFile) -> Str => !f
    fn close(f: OutFile) -> Str => !f
    fn describe(f: InFile) -> Str => f
}

// The handler implements both under the trait's names, with the
// trait's modes.
//~ emits: impl crate::__Stateless_Vault for Files {
//~ emits: fn close__InFile(&self, f: crate::InFile) -> String {
//~ emits: fn close__OutFile(&self, f: crate::OutFile) -> String {
handler Files of Vault {
    fn close(f: InFile) -> Str => !f {
        return "closed in ${f.id}"
    }
    fn close(f: OutFile) -> Str => !f {
        return "closed out ${f.id}"
    }
    fn describe(f: InFile) -> Str => f {
        return "file ${f.id}"
    }
}

fn main() [use] {
    use StdConsole()
    use Files()
    println(describe(InFile { id: 3 }))
    // The call sites, the `OutFile` one written with the `@Vault` selector:
    // each names its own overload and passes the consumed argument owned.
    //~ emits: .close__InFile(crate::InFile {
    println(close(InFile { id: 1 }))
    //~ emits: .close__OutFile(crate::OutFile {
    println(close@Vault(OutFile { id: 2 }))
}

//--- member_modes_follow_the_declared_clause
// [rs-borrows] [deduce-syntax] An effect member's parameter modes come from
// its *written* deduction clause: `=> !t` is taken by value, `=> t: Mut` by
// `&mut`. Phase 4's `Fs` is built out of exactly these two shapes — a
// consuming `close(s: InStream)` and a mutating `read_line(s: Mut InStream)`
// — and the interesting case is a **dependent** handler forwarding both,
// since the trait method, the handler impls, the generated `__Impl_H` trait
// and the fusion's forwarding impl must all agree or rustc refuses.
//
// [rs-borrows] The consumed parameter is owned in the trait, in both
// handlers, and at the call site; the `Mut` one is `&mut` throughout.
struct Token canbe Mut { id: Int }

// The trait.
//~ emits: fn take(&mut self, t: crate::Token) -> i32;
//~ emits: fn bump(&mut self, t: &mut crate::Token);
effect Sink {
    fn take(t: Token) -> Int => !t
    fn bump(t: Mut Token) -> None => t: Mut
}

// The handlers, independent and dependent alike (stateless: `&self`).
//~ emits: fn take(&self, t: crate::Token) -> i32 {
//~ emits: fn bump(&self, t: &mut crate::Token) {
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

//--- a_linear_token_is_discharged_by_an_effect_member
// [linear-group] [effect-member-overload] [rs-borrows] Phase 4's token shape
// in miniature, and the three changes it needs at once: a **linear** token
// whose only discharger is an *effect member*, that member **overloaded** per
// token type, and the consuming overloads taking their token **by value**
// while the mutating members take `&mut`. `discard` inside the handler bodies
// is the amendment (user decision 2026-09-14): discharger status attaches to
// the member declaration, so every handler's implementation of it terminates
// the obligation.
// The phase-4 token shape in miniature: a linear token whose only discharger
// is an *effect member*, discharged inside the handler that implements it.
linear struct InTape canbe Mut { handle: Int }
linear struct OutTape canbe Mut { handle: Int }

// The consuming overloads own their token; the mutating members
// borrow it mutably.
//~ emits: fn close__InTape(&mut self, s: crate::InTape) -> String;
//~ emits: fn close__OutTape(&mut self, s: crate::OutTape) -> String;
//~ emits: fn read_line(&mut self, s: &mut crate::InTape) -> String;
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
    // A discharged token is dropped, so `discard` emits nothing that
    // could resurrect it: the value simply ends there. (The handler is
    // stateless, so its receiver is `&self` [rs-handle].)
    //~ emits: fn close__InTape(&self, s: crate::InTape) -> String {
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
