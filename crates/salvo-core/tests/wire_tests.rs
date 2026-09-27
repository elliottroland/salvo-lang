//! [wire-format] [noremote] [protocol-hash] What may cross a machine, as the
//! checker sees it — step ② of the network sequence (user decisions
//! 2026-09-26). Every type is serializable unless it is, or holds, something
//! `noremote`; `encode`/`decode` are refused at the call, naming what stops
//! them; and an actor protocol's canonical hash is a property of its shape,
//! not its names.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// [intrinsic-std-only] A std-shaped prelude: the scalars, the containers the
/// predicate names, and `net`'s two intrinsics in a module of their own.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type Bytes canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export intrinsic type Set<T>(?hash: (T) -> Long, ?eq: (T, T) -> Bool) canbe Mut\n",
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn set_of<T>(...elems: T[]) [] -> Set<T>\n",
    "export noremote intrinsic type Pool\n",
    "export intrinsic fn pool(size: Int) [spawn] -> Pool => size\n",
    "export intrinsic type Addr<E>\n",
    "export linear intrinsic type Reply<T>\n",
    "export intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply, !value\n",
    "export struct Mailbox { capacity: Int }\n",
    "export intrinsic fn mut_list_of<T>(...elems: T[]) [] -> Mut List<T>\n",
    "export intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n",
    "export intrinsic fn copy<T>(value: T) [] -> T => value\n",
);

const STD_NET: &str = concat!(
    "export intrinsic fn encode<T>(value: T) [] -> Bytes => !value\n",
    "export intrinsic fn decode<T>(data: Bytes) [] -> T? => data\n",
    "export struct Protocol<E> { name: Str, hash: Str }\n",
    "export intrinsic fn protocol<E>() [] -> Protocol<E>\n",
    "export actor effect NodeGroup { send fn leave() }\n",
    "export actor effect ActorGroup<E> { send fn join(member: Addr<E>) => !member }\n",
    "export intrinsic fn actor_group<E>(nodes: Addr<NodeGroup>) [spawn] -> Int => !nodes\n",
    // The pick kit's shape, enough for the erased-sibling rule [route-stub].
    "export struct ActorGroupView<E> { key: Long? = None }\n",
    "export effect Pick<E> { fn choose(view: ActorGroupView<E>) -> Addr<E>? => !view }\n",
    "export handler LeastLoaded<E>(prefer_local: Bool) of Pick<E> {\n",
    "    fn choose(view: ActorGroupView<E>) -> Addr<E>? => !view { return None }\n}\n",
    "export handler Sharded<E>() of Pick<E> {\n",
    "    fn choose(view: ActorGroupView<E>) -> Addr<E>? => !view { return None }\n}\n",
);

fn program(src: &str) -> (Program, Vec<FileDiagnostic>) {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/net.sv",
        SourceSet::classify(Path::new("net.sv")).unwrap(),
        STD_NET.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let mut errors: Vec<FileDiagnostic> = resolution.errors.clone();
    errors.extend(check_program(&program, &resolution, &symbols).errors);
    (program, errors)
}

fn messages(src: &str) -> Vec<String> {
    program(src).1.iter().map(|d| d.message.clone()).collect()
}

const IMPORT: &str = "import net\n\n";

/// [wire-format] The default: a struct of scalars, strings, lists, options
/// and unions has a wire form, and so do `encode` and `decode` of it.
#[test]
fn ordinary_data_encodes() {
    let errs = messages(&format!(
        "{IMPORT}struct Tag {{ name: Str }}\n\
         struct Pin {{ x: Int, label: Str?, tags: List<Tag>, kind: Tag | Int | None, raw: Bytes }}\n\
         fn main() -> None {{\n    \
         let p = Pin {{ x: 1, label: None, tags: [], kind: 2, raw: encode(3) }}\n    \
         let b = encode(p)\n    let _back = decode<Pin>(b)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [noremote] A `noremote` struct is refused at the `encode`, by name.
#[test]
fn a_noremote_struct_is_refused_at_encode() {
    let errs = messages(&format!(
        "{IMPORT}noremote struct Canvas {{ n: Int }}\n\
         fn main() -> None {{\n    let b = encode(Canvas {{ n: 1 }})\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`encode` needs a type with a wire form")
            && e.contains("`Canvas` is declared `noremote`")
            && e.contains("[noremote]")),
        "expected the noremote refusal, got {errs:?}"
    );
}

/// [noremote] Transitive: a struct holding a `noremote` field is refused, and
/// the message names the field that stops it.
#[test]
fn noremote_is_transitive_through_fields() {
    let errs = messages(&format!(
        "{IMPORT}noremote struct Canvas {{ n: Int }}\n\
         struct Frame {{ title: Str, canvas: Canvas }}\n\
         fn main() -> None {{\n    let b = encode(Frame {{ title: \"t\", canvas: Canvas {{ n: 1 }} }})\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Frame.canvas (Canvas)` is declared `noremote`")),
        "expected the field to be named, got {errs:?}"
    );
}

/// [noremote] std's process-local handles are `noremote` by declaration
/// (`Pool` here), so a struct holding one has no wire form.
#[test]
fn a_pool_has_no_wire_form() {
    let errs = messages(&format!(
        "{IMPORT}struct Placement {{ where: Pool }}\n\
         fn main() -> None {{\n    let b = encode(Placement {{ where: pool(2) }})\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Placement.where (Pool)` is declared `noremote`")),
        "expected Pool to block, got {errs:?}"
    );
}

/// [noremote] A function value has no wire form, by construction.
#[test]
fn a_function_value_has_no_wire_form() {
    let errs = messages(&format!(
        "{IMPORT}struct Rule {{ test: (Int) -> Bool }}\n\
         fn main() -> None {{\n    let b = encode(Rule {{ test: n -> n > 0 }})\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("holds a function value")),
        "expected the fn-value block, got {errs:?}"
    );
}

/// [wire-format] Step ②'s recorded cuts, each refused with its reason: a keyed
/// container (identity capabilities not yet carried), and a generic `T`
/// (the form is the instantiation's).
#[test]
fn keyed_containers_and_generic_parameters_are_refused_for_now() {
    let errs = messages(&format!(
        "{IMPORT}fn ship<T>(v: T) -> Bytes => !v {{\n    return encode(v)\n}}\n\
         fn main() -> None {{\n    let b = decode<Set<Int>>(encode(1))\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Set` is a keyed container")),
        "expected the keyed-container cut, got {errs:?}"
    );
    assert!(
        errs.iter().any(|e| e.contains("`T` is a generic parameter")),
        "expected the generic cut, got {errs:?}"
    );
}

/// [protocol-hash] The hash is over the protocol's *shape*: a struct renamed
/// hashes the same (names are not in the canonical form), a field reordered
/// does not (the encoding is positional), and a member added does not.
#[test]
fn protocol_hash_follows_shape_not_names() {
    fn hash_of(src: &str) -> String {
        let (prog, errs) = program(src);
        assert!(errs.is_empty(), "expected no errors, got {errs:?}");
        let symbols = Symbols::collect(&prog);
        let resolution = resolve(&prog);
        let checked = check_program(&prog, &resolution, &symbols);
        checked.protocol_hashes["Ledger"].clone()
    }
    let base = hash_of(
        "struct Entry { who: Str, amount: Long }\n\
         actor effect Ledger { send fn record(e: Entry) => !e\n send fn close() }\n",
    );
    let renamed = hash_of(
        "struct Row { who: Str, amount: Long }\n\
         actor effect Ledger { send fn record(e: Row) => !e\n send fn close() }\n",
    );
    let reordered = hash_of(
        "struct Entry { amount: Long, who: Str }\n\
         actor effect Ledger { send fn record(e: Entry) => !e\n send fn close() }\n",
    );
    let grown = hash_of(
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

/// [actor-group] [noremote] The **crossing site**: naming a protocol with a
/// `noremote` payload for a group is refused at `protocol<E>()`, by member and
/// type — the protocol itself stays a legal local one (user decision
/// 2026-09-26: the check lands at the binding, not the declaration).
#[test]
fn a_protocol_with_a_noremote_payload_cannot_be_named_for_a_group() {
    let errs = messages(&format!(
        "{IMPORT}noremote struct Canvas {{ n: Int }}\n\
         actor effect Painter {{ send fn paint(c: Canvas) => !c }}\n\
         actor effect Counter {{ send fn bump(n: Int) => !n }}\n\
         fn main() -> None {{\n    let _ok = protocol<Counter>()\n    let _bad = protocol<Painter>()\n}}\n"
    ));
    assert_eq!(errs.len(), 1, "one refusal, for the noremote protocol only: {errs:?}");
    assert!(
        errs[0].contains("a group of `Painter` cannot span nodes")
            && errs[0].contains("`Painter.paint` takes `Canvas`")
            && errs[0].contains("[noremote]"),
        "expected the crossing-site refusal, got {errs:?}"
    );
    // [actor-group] The same site one level up: `actor_group<E>(…)`, where a
    // program actually writes the protocol.
    let errs = messages(&format!(
        "{IMPORT}noremote struct Canvas {{ n: Int }}\n\
         actor effect Painter {{ send fn paint(c: Canvas) => !c }}\n\
         fn main(nodes: Addr<NodeGroup>) [spawn] -> None {{\n    let _bad = actor_group<Painter>(nodes)\n}}\n"
    ));
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("a group of `Painter` cannot span nodes") && errs[0].contains("`actor_group<Painter>()`"),
        "{errs:?}"
    );
}

/// [effect-generic-decl] An effect generic over an *effect*: `Addr<E>` is the
/// one position `E` may take, a generic instance is itself an effect inside
/// `Addr` (`Addr<Reg<Ping>>`), and a phantom struct parameter (`Protocol<E>`)
/// types a value without occurring in it.
#[test]
fn effect_typed_generics_check() {
    let errs = messages(&format!(
        "{IMPORT}actor effect Ping {{ send fn ping(n: Int) => !n }}\n\
         actor effect Reg<E> {{\n    send fn join(member: Addr<E>) => !member\n    \
         send fn peers(out: Reply<List<Addr<E>>>) => !out\n    \
         send fn twin(other: Addr<Reg<E>>) => !other\n}}\n\
         handler Registering<E>() of Reg<E> {{\n    mailbox {{ capacity: 4 }}\n    \
         all: Mut List<Addr<E>> = mut_list_of()\n    \
         send fn join(member: Addr<E>) => !member {{ add(all, member) }}\n    \
         send fn peers(out: Reply<List<Addr<E>>>) => !out {{ out.send(copy(all)) }}\n    \
         send fn twin(other: Addr<Reg<E>>) => !other {{ }}\n}}\n\
         fn main() [spawn] -> None {{\n    let _r = spawn Registering<Ping>() on pool(1)\n    \
         let _p = protocol<Ping>()\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [effect-generic-decl] Two instances of an effect whose arguments are
/// effects erase to one type, so one scope holds one: the second `use` is
/// refused, naming the nested-block remedy; a nested scope is fine.
#[test]
fn two_erased_instances_cannot_share_a_scope() {
    let src = "\
import net

actor effect Ping {
    send fn ping(out: Reply<Str>) => !out
}

actor effect Pong {
    send fn pong(out: Reply<Str>) => !out
}

fn inner() [use] {
    use LeastLoaded<Pong>(true)
}

fn main() [use] {
    use LeastLoaded<Ping>(true)
    inner()
    use Sharded<Pong>()
}
";
    let errs: Vec<String> = program(src)
        .1
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("`Pick<Pong>` cannot be bound while `Pick<Ping>` is")
            && errs[0].contains("nested block"),
        "{errs:?}"
    );
}

/// [actor-group] The `in GROUP` clause of a spawn: the group must be of one
/// of the spawn's faces, and must be a group handle at all.
#[test]
fn spawn_in_group_needs_a_group_of_one_of_its_faces() {
    let src = "\
import net

actor effect Ping { send fn ping() }
actor effect Pong { send fn pong() }
handler Pinging() of Ping { mailbox { capacity: 1 } send fn ping() {} }
handler Both() of Ping, Pong { mailbox { capacity: 1 } send fn ping() {} send fn pong() {} }

fn main(pings: Addr<ActorGroup<Ping>>, pongs: Addr<ActorGroup<Pong>>, n: Int) [spawn] -> None
    => !pings, !pongs, !n {
    let _ok = spawn Pinging() on pool(1) in pings
    let (_p, _q) = spawn Both() on pool(1) in pongs
    let _wrong = spawn Pinging() on pool(1) in pongs
    let _not = spawn Pinging() on pool(1) in n
}
";
    let errs: Vec<String> = program(src)
        .1
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(errs.len(), 2, "{errs:?}");
    assert!(errs[0].contains("answers no `Addr<Pong>`") && errs[0].contains("serves `Ping`"), "{errs:?}");
    assert!(errs[1].contains("`in` takes a group handle") && errs[1].contains("this is `Int`"), "{errs:?}");
}
