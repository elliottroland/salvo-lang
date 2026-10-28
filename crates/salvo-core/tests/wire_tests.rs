//! [actor-protocol-hash] An actor protocol's canonical hash is a property of
//! its shape, not its names: a table (`Checked::protocol_hashes`), so these
//! stay Rust tests, over the real std (`salvo_testkit::front`). What may cross
//! a machine — [net-wire-format] [net-noremote], the diagnostics — is
//! `tests/check/wire/` (TESTS.md T1).

/// The protocol hash of `effect` (its key) in `files`, which must check clean.
fn hash_of(files: &[(&str, &str)], effect: &str) -> String {
    let checked = salvo_testkit::front::check(files);
    let errs = checked.errors();
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
    checked.checked.protocol_hashes.get(effect).unwrap_or_else(|| panic!("no hash for `{effect}`")).clone()
}

/// [actor-protocol-hash] [decl-id] A protocol's hash does not depend on an unrelated
/// type named like its type parameter: std's `ActorGroup<E>`'s `E` is the
/// parameter whether or not the program declares a `struct E` (found by the
/// A1 differential, which saw `ActorGroup`'s hash change with one). Over the
/// std front std is checked once, without the program, so the same shape is
/// also checked across two of the program's own modules: a generic effect in
/// `grp`, a `struct E` in `main`.
#[test]
fn a_type_named_like_a_type_parameter_leaves_the_hash_alone() {
    let std_group = |main: &str| hash_of(&[("main.sv", main)], "ActorGroup");
    assert_eq!(std_group(""), std_group("struct E { hp: Int }\n"), "the parameter, not the struct");

    let grp = "export actor effect Grp<E> {\n    send fn join(member: eff[E]) => !member\n}\n";
    let key = salvo_core::typekey::keyed("Grp", "grp");
    let own_group = |main: &str| hash_of(&[("grp.sv", grp), ("main.sv", main)], key);
    assert_eq!(
        own_group("import grp\n"),
        own_group("import grp\n\nstruct E { hp: Int }\n"),
        "the parameter, not the struct"
    );
}

/// [actor-protocol-hash] The hash is over the protocol's *shape*: a struct renamed
/// hashes the same (names are not in the canonical form), a field reordered
/// does not (the encoding is positional), and a member added does not.
#[test]
fn protocol_hash_follows_shape_not_names() {
    // [decl-id] A user declaration's key names its module.
    let ledger = |src: &str| hash_of(&[("main.sv", src)], salvo_core::typekey::keyed("Ledger", "main"));
    let base = ledger(
        "struct Entry { who: Str, amount: Long }\n\
         actor effect Ledger { send fn record(e: Entry) => !e\n send fn close() }\n",
    );
    let renamed = ledger(
        "struct Row { who: Str, amount: Long }\n\
         actor effect Ledger { send fn record(e: Row) => !e\n send fn close() }\n",
    );
    let reordered = ledger(
        "struct Entry { amount: Long, who: Str }\n\
         actor effect Ledger { send fn record(e: Entry) => !e\n send fn close() }\n",
    );
    let grown = ledger(
        "struct Entry { who: Str, amount: Long }\n\
         actor effect Ledger { send fn record(e: Entry) => !e\n send fn close()\n send fn audit() }\n",
    );
    assert_eq!(base.len(), 16, "sixteen hex digits: {base}");
    assert_eq!(base, renamed, "a renamed struct is the same protocol");
    assert_ne!(base, reordered, "a reordered field is a different protocol");
    assert_ne!(base, grown, "a new member is a different protocol");
    // And the constant is stable across runs and machines: FNV-1a 64 over a
    // fixed canonical string.
    assert_eq!(
        salvo_core::protocol_hash("record({Str,Long});close();"),
        base,
        "the canonical form is `member(types);` with structs expanded"
    );
}
