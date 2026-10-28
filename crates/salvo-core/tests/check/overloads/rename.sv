// ===== [fn-rename] a name of its own =====

// A rename settles an ambiguity the ranking cannot: the renamed overload
// answers *only* to the new name, so the old name is unambiguous again.
qualifier Even of Int {
    fn qualifies(n: Int) -> Bool { return true }
}

qualifier Small of Int {
    fn qualifies(n: Int) -> Bool { return true }
}

fn label(n: Even Int) [] -> Bool => n { return true }
fn label(n: Small Int) [] -> Str => n { return "small" }
rename fn label_small = label(n: Small Int)

fn settled() -> None {
    let n = 4
    if n is Even && n is Small {
        let _a: Bool = label(n)
        let _b: Str = label_small(n)
    }
}

// A rename is scoped: a statement-level one is in force to the end of its
// block, and gone after it.
fn tag(v: Int) [] -> Str { return "int" }

fn scoped() -> None {
    if true {
        rename fn tag_int = tag(v: Int)
        let _a: Str = tag_int(1)
    }
    //~ error[call-resolve:unknown-fn]
    let _b = tag_int(2)
}

// The new name must be free — a rename removes an ambiguity, so adding one
// would defeat it ("already a function in scope") — and the parameter list
// must name an overload exactly ("names no `tag` in scope"), of a name
// something declares ("nothing to rename").
//~ error[fn-rename:name-is-fn]
rename fn tag = tag(v: Int)
//~ error[fn-rename:no-overload]
rename fn tag2 = tag(v: Str)
//~ error[fn-rename:no-target]
rename fn nope2 = nope(v: Int)

// A renamed name already means one declaration, so `@module` on it is an
// error rather than a redundancy: that case stays a Rust test,
// `overload_tests.rs`, since over the real std the "`@` not needed" warning
// fires beside the error.
