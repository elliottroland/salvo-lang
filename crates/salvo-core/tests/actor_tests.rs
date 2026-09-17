//! [actor-spawn-expr] [actor-replyto] [actor-waitfor] [actor-use-addr] The
//! asynchronous surface's *checker* rules: what a `spawn` needs to be legal,
//! what a `replyto` may target, where a `waitfor` may stand, and how an `Addr`
//! is called. The forms' syntax is tested in `salvo-syntax`; the types they
//! produce and consume are `core.actor`'s ([actor-types]).
//!
//! The shape every test here rests on: a process is a handler bound
//! asynchronously, so almost every rule is one `use` already had, moved to
//! the spawn site.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// A stand-in for `core.actor` and the little of std these tests need.
/// `intrinsic` is std-only [intrinsic-std-only], so it is loaded as a std
/// file rather than pasted into the source under test.
const STD_PRELUDE: &str = "\
intrinsic type Int
intrinsic type Str
intrinsic type Bool
intrinsic type List<T> canbe Mut
intrinsic fn discard<T canbe linear>(value: T) [] -> None => !value
intrinsic type Addr<E>
struct Mailbox { capacity: Int }
linear intrinsic type Reply<T>
intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply, !value
intrinsic fn array_of<T>(...elems: T[]) [] -> T[]
intrinsic type Pool
intrinsic fn pool(size: Int) [spawn] -> Pool => size
provenance qualifier Dedicated of Pool
intrinsic fn thread() [spawn] -> Dedicated Pool
struct Exit { reason: Str }
intrinsic fn watch<E>(target: Addr<E>, on_exit: Reply<Exit>) [spawn] -> None => target, !on_exit
";

/// The effects and handlers the cases share: a `Counter` protocol with a
/// request/response pair, a `Log` dependency, and handlers for both.
const PRELUDE: &str = r#"
actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

actor effect Log {
    send fn note(what: Str) => !what
}

handler Printing() of Log {
    mailbox { capacity: 4 }

    send fn note(what: Str) {}
}

handler Counting() [Log] of Counter {
    mailbox { capacity: 16 }

    sum: Int = 0

    send fn bump(n: Int) {
        sum = sum + n
        note("bumped")
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}
"#;

fn errors(src: &str) -> Vec<String> {
    diagnostics(src)
        .into_iter()
        .filter(|(error, _)| *error)
        .map(|(_, message)| message)
        .collect()
}

/// [actor-deadlock-cycle] The other severity: a warning reports something the
/// program should look at without refusing it, and the back-pressure cycle is
/// the one on this surface.
fn warnings(src: &str) -> Vec<String> {
    diagnostics(src)
        .into_iter()
        .filter(|(error, _)| !*error)
        .map(|(_, message)| message)
        .collect()
}

/// Every diagnostic the program produces, as `(is_error, message)`.
fn diagnostics(src: &str) -> Vec<(bool, String)> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        format!("{PRELUDE}\n{src}"),
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
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| (d.is_error(), d.message.clone()))
        .collect()
}

// ===== The shape that must check clean =====

/// The whole first-pass surface in one `main`: two spawns (one supplying a
/// dependency with the *other's* addr), a send through an addr, `waitfor` with
/// the token consumed inside its block, and `use addr` followed by an
/// unqualified call. If this ever stops checking, the feature is broken
/// regardless of what the negative tests say.
#[test]
fn the_first_pass_surface_checks_clean() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let logger = spawn Printing() on pool(1)
    let counter = spawn Counting() use logger on pool(2)
    counter.bump(2)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    use counter
    bump(sum)
}
",
    );
    assert!(errs.is_empty(), "the first-pass surface must check: {errs:?}");
}

// ===== [actor-spawn-expr] The spawn =====

/// [actor-spawn-effect] The capability gate: creating a process is something
/// a function must declare, exactly as registering a handler is.
#[test]
fn a_spawn_requires_the_spawn_capability() {
    let errs = errors(
        "\
fn start() [use] -> Int {
    let c = spawn Printing() on pool(1)
    return 1
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("`spawn` requires the `spawn` capability")),
        "expected the capability gate: {errs:?}"
    );
}

/// A handler's dependencies come from the spawn's own `use` clause, never
/// from the spawning scope — the rule that makes "handlers never cross into a
/// process, construction does" checkable. The diagnostic says where they do
/// come from.
#[test]
fn a_spawned_handler_cannot_inherit_the_spawning_scope() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    use Printing()
    let counter = spawn Counting() on pool(1)
}
",
    );
    let diag = errs
        .iter()
        .find(|m| m.contains("depends on effect `Log`"))
        .unwrap_or_else(|| panic!("expected the missing-dependency error: {errs:?}"));
    assert!(
        diag.contains("`use` clause") && diag.contains("never from the spawning scope"),
        "the diagnostic must name the remedy: {diag}"
    );
}

/// Supplying something the handler does not depend on is a mistake, not
/// generosity: the reader believes it is being used.
#[test]
fn a_spawn_cannot_supply_an_unused_dependency() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let counter = spawn Printing() use Printing() on pool(1)
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("does not depend on effect `Log`")),
        "expected the unused-dependency error: {errs:?}"
    );
}

/// A dependency *constructed* in the clause may not itself have
/// dependencies: there is no scope on the child to resolve them from. The
/// remedy is an addr of a process that already serves the effect.
#[test]
fn a_clause_construction_cannot_have_dependencies_of_its_own() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let c = spawn Counting() use Counting() on pool(1)
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("cannot be constructed in a spawn's `use` clause")),
        "expected the nested-dependency refusal: {errs:?}"
    );
}

/// [actor-mailbox] The mailbox is typed where it is declared — its `capacity`
/// is an `Int`, checked as any struct field's value is — and the spawn's one
/// remaining clause is typed too: `on` takes a `Pool`, which `pool(n)` is the
/// way to get.
#[test]
fn the_mailbox_and_the_pool_clause_are_typed() {
    let errs = errors(
        "\
handler Loud() of Log {
    mailbox { capacity: \"lots\" }
    send fn note(what: Str) {}
}

fn main() [use, spawn, waitfor] {
    let b = spawn Printing() on 7
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("field `capacity` expects `Int`, found `Str`")),
        "expected the capacity type error: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("`Pool`") && m.contains("on pool(2)")),
        "expected the pool type error: {errs:?}"
    );
}

/// A spawn's value is the child's `Addr<E>` for the effect its handler
/// implements — which is what makes it usable as another spawn's dependency
/// and as a receiver. Checked through a *wrong* annotation, so the message
/// states the type.
#[test]
fn a_spawn_answers_a_addr_of_the_handlers_effect() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let c: Int = spawn Counting() use Printing() on pool(1)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("Addr<Counter>")),
        "the spawn's type must be `Addr<Counter>`: {errs:?}"
    );
}

// ===== [actor-use-addr] Sends through an addr =====

/// A dot-call through an addr resolves against the effect the process serves,
/// with the receiver naming *where* the message goes rather than being the
/// first argument.
#[test]
fn a_addr_call_resolves_against_the_served_effect() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let counter = spawn Counting() use Printing() on pool(1)
    counter.wind_down()
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("serves effect `Counter`") && m.contains("no member named")),
        "expected the unknown-member error naming the effect: {errs:?}"
    );
}

#[test]
fn a_addr_call_checks_its_arguments() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let counter = spawn Counting() use Printing() on pool(1)
    counter.bump(\"two\")
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("expected `Int`, found `Str`")),
        "expected the argument type error: {errs:?}"
    );
}

/// `use addr` binds the effect in scope, so its members are callable
/// unqualified — and an addr is *not* consumed by binding it, because an addr is
/// freely copyable.
#[test]
fn use_addr_binds_the_effect_and_keeps_the_addr() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let counter = spawn Counting() use Printing() on pool(1)
    use counter
    bump(1)
    counter.bump(2)
}
",
    );
    assert!(errs.is_empty(), "got {errs:?}");
}

/// `use` of something that is neither a handler nor an addr says so, naming
/// both forms.
#[test]
fn use_of_a_plain_value_is_refused() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let n = 1
    use n
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("registers a handler or binds an `Addr`")),
        "expected the use-a-value refusal: {errs:?}"
    );
}

// ===== [actor-replyto] The mint =====

/// `replyto` targets a member of the handler it is written in, so it is
/// illegal outside one — and the diagnostic names `main`'s alternative.
#[test]
fn replyto_is_illegal_outside_a_handler() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let r = replyto somewhere()
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("legal only inside a handler member") && m.contains("waitfor")),
        "expected the placement error: {errs:?}"
    );
}

/// The target must exist, and must be a `send fn`: an answer arrives as a
/// message.
#[test]
fn replyto_needs_a_send_member_of_its_own_handler() {
    let unknown = errors(
        "\
handler Asking() [Counter] of Log {
    mailbox { capacity: 1 }

    send fn note(what: Str) {
        total(replyto arrived())
    }
}
",
    );
    assert!(
        unknown
            .iter()
            .any(|m| m.contains("has no member `arrived`")),
        "expected the unknown-member error: {unknown:?}"
    );

    let not_send = errors(
        "\
actor effect Ask {
    send fn go(out: Reply<Int>) => !out
    fn ready() -> Bool
}

handler Asker() [Counter] of Ask {
    mailbox { capacity: 1 }

    send fn go(out: Reply<Int>) {
        out.send(1)
    }

    fn ready() -> Bool {
        total(replyto ready())
        return true
    }
}
",
    );
    assert!(
        not_send
            .iter()
            .any(|m| m.contains("is not a `send fn`") && m.contains("answer arrives as a message")),
        "expected the non-send target refusal: {not_send:?}"
    );
}

/// The target member's parameters are the captures *plus one*: the trailing
/// parameter is the answer the token carries, which is what types the
/// `Reply<T>`. Writing the wrong number of captures says how many are wanted.
#[test]
fn replyto_captures_are_the_members_leading_parameters() {
    let errs = errors(
        "\
actor effect Ask {
    send fn go() 
    send fn arrived(id: Int, sum: Int) => !id, !sum
}

handler Asker() [Counter] of Ask {
    mailbox { capacity: 1 }

    send fn go() {
        total(replyto arrived())
    }

    send fn arrived(id: Int, sum: Int) {}
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("wants 1 capture(s), found 0")),
        "expected the capture-count error: {errs:?}"
    );
}

/// And with the captures written, the token's type comes from that trailing
/// parameter: `arrived(id: Int, sum: Int)` minted with one capture is a
/// `Reply<Int>`, which is what `total` takes.
#[test]
fn a_replyto_types_its_token_from_the_answer_parameter() {
    let errs = errors(
        "\
actor effect Ask {
    send fn go()
    send fn arrived(id: Int, sum: Int) => !id, !sum
    send fn wrong(id: Int, text: Str) => !id, !text
}

handler Asker() [Counter] of Ask {
    mailbox { capacity: 1 }

    send fn go() {
        total(replyto arrived(7))
    }

    send fn arrived(id: Int, sum: Int) {}

    send fn wrong(id: Int, text: Str) {}
}
",
    );
    assert!(errs.is_empty(), "the matching mint must check: {errs:?}");

    let mismatch = errors(
        "\
actor effect Ask {
    send fn go()
    send fn wrong(id: Int, text: Str) => !id, !text
}

handler Asker() [Counter] of Ask {
    send fn go() {
        total(replyto wrong(7))
    }

    send fn wrong(id: Int, text: Str) {}
}
",
    );
    assert!(
        mismatch
            .iter()
            .any(|m| m.contains("`Reply<Int>`") && m.contains("found `Reply<Str>`")),
        "the token's payload must come from the answer parameter: {mismatch:?}"
    );
}

// ===== [actor-waitfor] The bridge =====

/// [waitfor-effect] `waitfor` may occupy the thread it runs on, so the
/// function must hold the capability — and since 2026-09-17 that is all it
/// takes: `main` was never special except by owning its thread, and any
/// function may say so (user decision, T-5(c)).
#[test]
fn waitfor_needs_the_waitfor_capability() {
    let errs = errors(
        "\
fn helper() [use, spawn] -> Int {
    let counter = spawn Counting() use Printing() on pool(1)
    return waitfor out: Reply<Int> {
        counter.total(out)
    }
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("must declare `[waitfor]`")),
        "expected the capability rule: {errs:?}"
    );

    let declared = errors(
        "\
fn helper() [use, spawn, waitfor] -> Int {
    let counter = spawn Counting() use Printing() on pool(1)
    return waitfor out: Reply<Int> {
        counter.total(out)
    }
}
",
    );
    assert!(
        declared.is_empty(),
        "a fn that declares `[waitfor]` may wait, wherever it is: {declared:?}"
    );
}

/// [waitfor-effect] And it **propagates through calls** like any effect: a
/// caller that reaches a wait through a helper is a frame that may be
/// occupied, which is what the placement check downstream reads. (The
/// `[spawn]` capability was decorative for exactly this reason once.)
#[test]
fn the_waitfor_capability_propagates_through_calls() {
    let errs = errors(
        "\
fn waits() [use, spawn, waitfor] -> Int {
    let counter = spawn Counting() use Printing() on pool(1)
    return waitfor out: Reply<Int> {
        counter.total(out)
    }
}

fn caller() [use, spawn] -> Int {
    return waits()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("may occupy the thread it runs on")
            && m.contains("[waitfor]")),
        "expected the propagation error: {errs:?}"
    );
}

/// [waitfor-dedicated] The grant check, and the whole point of typing a
/// placement: a handler that declares `[waitfor]` may occupy its thread, so it
/// must own one. `thread()` answers a `Dedicated Pool`; `pool(n)` does not, and
/// neither does the pool current at the spawn (`main`'s included).
#[test]
fn a_waiting_handler_must_be_spawned_on_a_dedicated_thread() {
    const WAITING: &str = "\
handler Slow() [Counter, waitfor] of Log {
    mailbox { capacity: 2 }

    send fn note(what: Str) {
        let sum = waitfor out: Reply<Int> {
            total(out)
        }
        discard(what)
        discard(sum)
    }
}
";
    let shared = errors(&format!(
        "{WAITING}
fn main() [use, spawn] {{
    let counter = spawn Counting() use Printing() on pool(1)
    let slow = spawn Slow() use counter on pool(2)
}}
"
    ));
    assert!(
        shared.iter().any(|m| m.contains("declares `[waitfor]`")
            && m.contains("on thread()")),
        "expected the dedicated-placement rule: {shared:?}"
    );

    let inherited = errors(&format!(
        "{WAITING}
fn main() [use, spawn] {{
    let counter = spawn Counting() use Printing() on pool(1)
    let slow = spawn Slow() use counter
}}
"
    ));
    assert!(
        inherited.iter().any(|m| m.contains("declares `[waitfor]`")),
        "an omitted `on` inherits a shared pool, so it is refused too: {inherited:?}"
    );

    let dedicated = errors(&format!(
        "{WAITING}
fn main() [use, spawn] {{
    let counter = spawn Counting() use Printing() on pool(1)
    let slow = spawn Slow() use counter on thread()
}}
"
    ));
    assert!(
        dedicated.is_empty(),
        "`on thread()` is the placement it needs: {dedicated:?}"
    );
}

/// [waitfor-dedicated] A dedicated thread has exactly one occupant, and the
/// `on` clause is what spends it: reusing the value is the ordinary
/// use-after-move diagnostic rather than a rule of its own (user refinement
/// 2026-09-17).
#[test]
fn an_on_clause_consumes_a_dedicated_pool() {
    let errs = errors(
        "\
fn main() [use, spawn] {
    let own = thread()
    let a = spawn Printing() on own
    let b = spawn Printing() on own
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("`own`") && m.contains("consumed")),
        "expected the move diagnostic on the second spawn: {errs:?}"
    );

    let shared = errors(
        "\
fn main() [use, spawn] {
    let many = pool(2)
    let a = spawn Printing() on many
    let b = spawn Printing() on many
}
",
    );
    assert!(
        shared.is_empty(),
        "a plain `Pool` is shared by many spawns: {shared:?}"
    );
}

/// [waitfor-effect] Binding a `[waitfor]`-carrying handler with `use` makes
/// *this* frame one that may be occupied: the capability flows outward from
/// the binding, so the binder declares it too. Effect callers learn nothing —
/// a handler's dependencies are invisible to them by the ordinary rule.
#[test]
fn using_a_waiting_handler_needs_the_capability() {
    const WAITING: &str = "\
handler Slow() [Counter, waitfor] of Log {
    mailbox { capacity: 2 }

    send fn note(what: Str) {
        let sum = waitfor out: Reply<Int> {
            total(out)
        }
        discard(what)
        discard(sum)
    }
}
";
    let errs = errors(&format!(
        "{WAITING}
fn main() [use, spawn] {{
    let counter = spawn Counting() use Printing() on pool(1)
    use counter
    use Slow()
}}
"
    ));
    assert!(
        errs.iter().any(|m| m.contains("declares `[waitfor]`")
            && m.contains("must declare")),
        "expected the use-site grant check: {errs:?}"
    );

    let declared = errors(&format!(
        "{WAITING}
fn main() [use, spawn, waitfor] {{
    let counter = spawn Counting() use Printing() on pool(1)
    use counter
    use Slow()
}}
"
    ));
    assert!(
        declared.is_empty(),
        "declaring it is all it takes: {declared:?}"
    );
}

/// [waitfor-effect] An **effect declaration's member** may carry `[waitfor]`
/// (user decision 2026-09-17, the T-5 refinement): "this member may occupy
/// your thread" is a fact about the protocol, so it belongs where the protocol
/// is written — and a caller must then hold the capability. It is the only
/// effect ref a member may declare [effect-member-no-effects].
#[test]
fn an_effect_member_may_declare_waitfor() {
    const CLOCK: &str = "\
effect Clock {
    fn now() [waitfor] -> Int
}

handler Ticking() of Clock {
    fn now() [waitfor] -> Int {
        return 7
    }
}
";
    let ok = errors(&format!(
        "{CLOCK}
fn reader() [Clock, waitfor] -> Int {{
    return now()
}}
"
    ));
    assert!(
        ok.is_empty(),
        "`[waitfor]` is legal on an effect member and its handler: {ok:?}"
    );

    let missing = errors(&format!(
        "{CLOCK}
fn reader() [Clock] -> Int {{
    return now()
}}
"
    ));
    assert!(
        missing
            .iter()
            .any(|m| m.contains("may occupy the thread it runs on")),
        "a caller of a waiting member must hold the capability: {missing:?}"
    );

    let other = errors(
        "\
effect Clock {
    fn now() [Log] -> Int
}
",
    );
    assert!(
        other
            .iter()
            .any(|m| m.contains("cannot declare effect dependencies")),
        "every other effect ref is still refused on a member: {other:?}"
    );
}

/// [main-pool] An omitted `on` clause means the pool current at the spawn —
/// `main`'s own pool in `main` (FC-4(a)), the actor's own in a member. It is
/// how a spawn site names the main pool without new vocabulary.
#[test]
fn a_spawn_may_omit_its_pool() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let counter = spawn Counting() use Printing()
    counter.bump(2)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
}
",
    );
    assert!(errs.is_empty(), "`on` is optional: {errs:?}");
}

/// Its binding is a reply token; anything else is a mistake about what the
/// form does.
#[test]
fn waitfor_binds_a_reply_token() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let n = waitfor out: Int {
        let x = 1
    }
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("its binding is a `Reply<T>`")),
        "expected the token type error: {errs:?}"
    );
}

/// The token is linear, so "the block must consume it" needs no rule of its
/// own — and the leak names the discharger, `send`.
#[test]
fn a_waitfor_that_never_sends_its_token_leaks() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let sum = waitfor out: Reply<Int> {
        let x = 1
    }
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("still owns a linear value") && m.contains("`send`")),
        "expected the token leak: {errs:?}"
    );
}

/// And its value is the token's payload, so `waitfor out: Reply<Int>` is an
/// `Int` — checked through a wrong annotation.
#[test]
fn a_waitfor_answers_its_tokens_payload() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let counter = spawn Counting() use Printing() on pool(1)
    let sum: Str = waitfor out: Reply<Int> {
        counter.total(out)
    }
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("expected `Str`, found `Int`")),
        "the bridge's value is the payload: {errs:?}"
    );
}

// ===== The leftovers this slice closed =====

/// [actor-use-addr] A receiver is any **place** whose type is an addr, not just
/// a variable: a field chain (a registry of children, a supervisor's state)
/// and an array element both work, which is what a program holding several
/// processes writes.
#[test]
fn a_addr_place_of_any_shape_is_a_receiver() {
    let errs = errors(
        "\
struct Registry {
    counter: Addr<Counter>
}

fn main() [use, spawn, waitfor] {
    let counter = spawn Counting() use Printing() on pool(1)
    let r = Registry { counter: counter }
    r.counter.bump(1)
    let many: Addr<Counter>[] = array_of(r.counter)
    many[0].bump(2)
}
",
    );
    assert!(errs.is_empty(), "an addr place must be a receiver: {errs:?}");
}

/// [actor-spawn-expr] [actor-waitfor] [actor-replyto] The asynchronous forms
/// stop at a **closure**: a function value's body runs wherever it is called,
/// a fn type cannot declare `spawn`, `waitfor` needs `main`'s own thread, and
/// a continuation belongs to the handler that minted it. Each diagnostic says
/// so in the closure's terms rather than repeating the general rule.
#[test]
fn the_asynchronous_forms_stop_at_a_lambda() {
    let spawned = errors(
        "\
fn main() [use, spawn, waitfor] {
    let f = () -> spawn Printing() on pool(1)
}
",
    );
    assert!(
        spawned
            .iter()
            .any(|m| m.contains("cannot be written inside a lambda")
                && m.contains("pass the `Addr`")),
        "expected the lambda-spawn refusal: {spawned:?}"
    );

    let waited = errors(
        "\
fn main() [use, spawn, waitfor] {
    let counter = spawn Counting() use Printing() on pool(1)
    let f = () -> waitfor out: Reply<Int> {
        counter.total(out)
    }
}
",
    );
    assert!(
        waited
            .iter()
            .any(|m| m.contains("cannot be written inside a lambda")
                && m.contains("a fn type cannot declare `waitfor`")),
        "expected the lambda-waitfor refusal: {waited:?}"
    );

    let minted = errors(
        "\
actor effect Ask {
    send fn go()
    send fn arrived(sum: Int) => !sum
}

handler Asker() [Counter] of Ask {
    send fn go() {
        let f = () -> total(replyto arrived())
    }

    send fn arrived(sum: Int) {}
}
",
    );
    assert!(
        minted
            .iter()
            .any(|m| m.contains("a lambda is not one")),
        "expected the lambda-replyto refusal: {minted:?}"
    );
}

/// [effect-member-overload] An overloaded send member reached through an addr
/// is picked by **argument count**: typing the arguments to choose and then
/// again against the winner would report every mistake in them twice. A tie
/// is refused rather than guessed.
#[test]
fn an_overloaded_send_member_is_picked_by_arity() {
    let ok = errors(
        "\
actor effect Sink {
    send fn put(a: Int) => !a
    send fn put(a: Int, b: Int) => !a, !b
}

handler Dropping() of Sink {
    mailbox { capacity: 1 }

    send fn put(a: Int) {}
    send fn put(a: Int, b: Int) {}
}

fn main() [use, spawn, waitfor] {
    let s = spawn Dropping() on pool(1)
    s.put(1)
    s.put(1, 2)
}
",
    );
    assert!(ok.is_empty(), "arity must settle the overload: {ok:?}");

    let tie = errors(
        "\
actor effect Sink {
    send fn put(a: Int) => !a
    send fn put(a: Str) => !a
}

handler Dropping() of Sink {
    send fn put(a: Int) {}
    send fn put(a: Str) {}
}

fn main() [use, spawn, waitfor] {
    let s = spawn Dropping() on pool(1)
    s.put(1)
}
",
    );
    assert!(
        tie.iter()
            .any(|m| m.contains("picks by argument count")),
        "a same-arity overload must be refused, not guessed: {tie:?}"
    );
}

// ===== [actor-self-send] `k@self(args)` =====

/// The form's point is *ordering*: an unqualified call would run `k` inside
/// this activation, and a self-send runs it as its own later one — which is
/// how "finish this, then continue with `k`" is written. Legal in a member
/// body, with the arguments checked like any send's.
#[test]
fn a_member_can_send_to_its_own_process() {
    let errs = errors(
        "\
actor effect Work {
    send fn start(n: Int) => !n
    send fn step(n: Int) => !n
}

handler Working() of Work {
    mailbox { capacity: 1 }

    done: Int = 0

    send fn start(n: Int) {
        step@self(n)
    }

    send fn step(n: Int) {
        done = done + n
    }
}
",
    );
    assert!(errs.is_empty(), "a self-send must check: {errs:?}");
}

/// The target must exist and be a `send fn`: a member that answers would have
/// to wait for itself.
#[test]
fn a_self_send_needs_a_send_member_of_this_handler() {
    let unknown = errors(
        "\
handler Working() of Log {
    mailbox { capacity: 1 }

    send fn note(what: Str) {
        later@self(what)
    }
}
",
    );
    assert!(
        unknown
            .iter()
            .any(|m| m.contains("has no member `later` to send to")),
        "expected the unknown-member error: {unknown:?}"
    );

    let not_send = errors(
        "\
actor effect Work {
    send fn start(n: Int) => !n
    fn ready() -> Bool
}

handler Working() of Work {
    mailbox { capacity: 1 }

    send fn start(n: Int) {
        ready@self()
    }

    fn ready() -> Bool {
        return true
    }
}
",
    );
    assert!(
        not_send
            .iter()
            .any(|m| m.contains("is not a `send fn`") && m.contains("wait for itself")),
        "expected the non-send refusal: {not_send:?}"
    );
}

/// `self` names the handler a member belongs to, so the form needs one: in a
/// plain function it is an error saying what `self` means.
#[test]
fn a_self_send_is_illegal_outside_a_handler_member() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    bump@self(1)
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("legal only inside a handler member")),
        "expected the placement error: {errs:?}"
    );
}

/// Its arguments are typed against the member's parameters, and a payload is
/// *consumed*: the message outlives this activation even though it never
/// leaves the process.
#[test]
fn a_self_send_types_and_consumes_its_arguments() {
    let errs = errors(
        "\
actor effect Work {
    send fn start(n: Int) => !n
    send fn step(n: Int) => !n
}

handler Working() of Work {
    mailbox { capacity: 1 }

    send fn start(n: Int) {
        step@self(\"one\")
    }

    send fn step(n: Int) {}
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("expected `Int`, found `Str`")),
        "expected the argument type error: {errs:?}"
    );
}

/// [actor-self-send] `self` is **contextual**, not reserved: it means the
/// enclosing handler only immediately after `@`, so it stays an ordinary name
/// elsewhere — and a local of that name can never shadow the form, which is
/// what the selector spelling buys over the `self.k(…)` receiver it replaced.
#[test]
fn self_stays_an_ordinary_name_away_from_the_selector() {
    let errs = errors(
        "\
handler Working() of Log {
    mailbox { capacity: 1 }

    send fn note(what: Str) {
        let self = 1
        note@self(what)
    }
}

fn plain() -> Int {
    let self = 2
    return self
}
",
    );
    assert!(errs.is_empty(), "`self` must stay a legal name: {errs:?}");
}

/// And the *old* spelling is a plain parse error naming the new one — no
/// transitional accept (the `canbe` precedent: nothing outside this repository
/// writes Salvo).
#[test]
fn the_receiver_spelling_of_a_self_send_is_a_parse_error() {
    let source = "\
handler Working() of Log {
    send fn note(what: Str) {
        self.note(what)
    }
}
";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics.iter().any(|d| d.is_error()
            && d.message.contains("is not the self-send form")
            && d.message.contains("`k@self(…)`")),
        "expected the old form to be refused: {diagnostics:?}"
    );
}

/// The self-dispatch diagnostic now names the remedy that exists: calling
/// one's own effect member unqualified is still refused, but for a `send fn`
/// the message points at `k@self(…)`.
#[test]
fn the_self_dispatch_error_names_the_self_send() {
    let errs = errors(
        "\
handler Counting2() of Counter {
    sum: Int = 0

    send fn bump(n: Int) {
        bump(n)
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("send it to this actor instead: `bump@self(…)`")),
        "expected the diagnostic to name the self-send: {errs:?}"
    );
}

// ===== [actor-effect-kind] [actor-sendable] The effect kind =====

/// The kind is declared, not diagnosed (user decision 2026-09-15, EU-5): an
/// author choosing between `effect` and `actor effect` is deciding whether the
/// protocol crosses threads, so `send fn` needs the actor kind and the
/// diagnostic names the marker.
#[test]
fn a_send_member_needs_an_actor_effect() {
    let errs = errors(
        "\
effect Plain {
    send fn nope(n: Int) => !n
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("needs an `actor effect`") && m.contains("actor effect Plain")),
        "expected the kind requirement: {errs:?}"
    );
}

/// And inside an `actor effect`, everything a seam cannot carry is refused
/// **at the declaration**, where the choice is being made: a kept parameter, a
/// `Mut` parameter, and a non-sendable payload — each naming the law rather
/// than the symptom.
#[test]
fn an_actor_effect_refuses_what_cannot_cross_a_seam() {
    let errs = errors(
        "\
struct Job {
    name: Str,
    run: () -> Int
}

actor effect Bad {
    send fn keeps(s: Str) => s
    send fn mutates(xs: Mut List<Int>) => !xs
    send fn unsendable(j: Job) => !j
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("cannot keep `s`") && m.contains("always consumed")),
        "expected the kept-parameter refusal: {errs:?}"
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("cannot take a `Mut` parameter")),
        "expected the Mut-parameter refusal: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("cannot carry `Job`")
            && m.contains("holds a function value")),
        "expected the sendability refusal: {errs:?}"
    );
}

/// [actor-sendable] The other half of C-4(a): a `proj` view borrows the
/// sender's value, so it cannot cross either.
#[test]
fn a_view_is_not_sendable() {
    let errs = errors(
        "\
struct Window {
    over: proj List<Int>
}

actor effect Peek {
    send fn look(w: Window) => !w
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("cannot carry `Window`")
            && m.contains("holds a `proj` view")),
        "expected the view refusal: {errs:?}"
    );
}

/// [actor-effect-kind] The binding gate, which is what closes the design's
/// carried named question: a plain effect is **never** process-backed, so
/// `spawn`, `use addr` and even naming `Addr<E>` require the actor kind. The
/// last is reported where the type is written, before any spawn exists.
#[test]
fn only_an_actor_effect_can_be_bound_to_an_actor() {
    let errs = errors(
        "\
effect Plain {
    fn ping() -> Int
}

handler Pinging() of Plain {
    mailbox { capacity: 1 }

    fn ping() -> Int {
        return 1
    }
}

fn hold(a: Addr<Plain>) -> Int => a {
    return 1
}

fn main() [use, spawn, waitfor] {
    let p = spawn Pinging() on pool(1)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("`Addr<Plain>` needs an `actor effect`")),
        "expected the addr-type gate: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("`spawn` cannot bind `Plain` to an actor")
            && m.contains("no mailbox")),
        "expected the spawn gate: {errs:?}"
    );
}

/// A **plain** handler binding of an actor effect stays legal — that is
/// Example 6's binding swap, and the reason the kind sits on the effect rather
/// than on the handler.
#[test]
fn an_actor_effect_can_still_be_used_synchronously() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    use Printing()
    note(\"inline\")
}
",
    );
    assert!(errs.is_empty(), "the binding swap must stay legal: {errs:?}");
}

/// [actor-replyto] A handler that **parks** may only be spawned (user decision
/// 2026-09-15, D5-c). Bound with `use`, its member bodies run inline on the
/// caller's thread: the mint would target a member of a *local* instance, which
/// has no mailbox for the answer to arrive on and no dispatcher to run it, so
/// the continuation would silently never run. Parking is the one thing only a
/// process can do, so the restriction costs nothing real.
///
/// The gate is syntactic per *handler*, not per member, because effects
/// propagate: a fn declaring `[Counter]` may call any member, so a `use` site
/// cannot know which ones a scope will reach.
#[test]
fn a_parking_handler_may_not_be_used_synchronously() {
    let errs = errors(
        "\
actor effect Waiting {
    send fn ask() 
    send fn got(n: Int) => !n
}

handler Asking() [Counter] of Waiting {
    mailbox { capacity: 1 }

    send fn ask() {
        total(replyto got())
    }
    send fn got(n: Int) {}
}

fn main() [use, spawn, waitfor] {
    let c = spawn Counting() use Printing() on pool(1)
    use c
    use Asking()
    ask()
}
",
    );
    assert!(
        errs.iter().any(|m| m
            .contains("mints a continuation with `replyto`, so it can only be `spawn`ed")),
        "expected the parking-handler `use` refusal: {errs:?}"
    );
}

/// …and spawning it is fine, which is what makes the refusal a *binding* rule
/// rather than a restriction on the form.
#[test]
fn a_parking_handler_may_be_spawned() {
    let errs = errors(
        "\
actor effect Waiting {
    send fn ask() 
    send fn got(n: Int) => !n
}

handler Asking() [Counter] of Waiting {
    mailbox { capacity: 1 }

    send fn ask() {
        total(replyto got())
    }
    send fn got(n: Int) {}
}

fn main() [use, spawn, waitfor] {
    let c = spawn Counting() use Printing() on pool(1)
    let a = spawn Asking() use c on pool(1)
    a.ask()
}
",
    );
    assert!(errs.is_empty(), "spawning a parking handler must be legal: {errs:?}");
}

/// [actor-replyto] A **remote** mint — `k` naming a member of another `actor
/// effect` in scope rather than of the enclosing handler — is the generalized
/// form (the generalized mint, decided — ROADMAP.md's sugar pass) and a later
/// slice: it makes the
/// mint itself send-like, since capacity must be reserved in the *target's*
/// queue. Named, with the workaround that needs nothing new — a token is an
/// ordinary linear value, so the handler that owns `k` mints it and passes it.
#[test]
fn a_remote_mint_target_is_refused_by_name() {
    let errs = errors(
        "\
actor effect Waiting {
    send fn ask() 
}

handler Asking() [Counter] of Waiting {
    mailbox { capacity: 1 }

    send fn ask() {
        total(replyto bump())
    }
}

fn main() [use, spawn, waitfor] {
    let c = spawn Counting() use Printing() on pool(1)
    let a = spawn Asking() use c on pool(1)
    a.ask()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("`bump` is a member of `Counter` instead")
            && m.contains("minting toward another actor's member is not supported yet")),
        "expected the remote-mint refusal naming the effect: {errs:?}"
    );
}

// ===== [actor-watch] the monitor surface =====

/// [actor-watch] The whole of it: a token minted like any other, handed to
/// `watch`, and answered by the scheduler when the process dies. `main`'s
/// token comes from `waitfor`, so a program can wait for a child's death
/// without a handler of its own.
#[test]
fn watch_takes_an_addr_and_a_reply_token() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let c = spawn Counting() use Printing() on pool(1)
    let e = waitfor out: Reply<Exit> {
        watch(c, out)
    }
    discard(e.reason)
}
",
    );
    assert!(errs.is_empty(), "watching a process must be legal: {errs:?}");
}

/// [actor-watch] [linear-obligation] The registration *is* the obligation:
/// the token is consumed by `watch`, so a `waitfor` block that mints one and
/// registers nothing is the ordinary leak — which is how "you cannot silently
/// forget you were watching" is enforced, with no rule of its own.
#[test]
fn a_watch_token_that_is_never_registered_leaks() {
    let errs = errors(
        "\
fn main() [use, spawn, waitfor] {
    let c = spawn Counting() use Printing() on pool(1)
    let e = waitfor out: Reply<Exit> {
        discard(c)
    }
    discard(e.reason)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("out")),
        "an unregistered token must be reported as a leak: {errs:?}"
    );
}

/// [actor-watch] [actor-spawn-effect] Watching is a `spawn`-capability
/// operation: a monitor is part of running processes, so a function that has
/// not been given the capability cannot register one.
#[test]
fn watch_needs_the_spawn_capability() {
    let errs = errors(
        "\
fn observe(c: Addr<Counter>, out: Reply<Exit>) [] -> None => c, !out {
    watch(c, out)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("spawn")),
        "watch without `[spawn]` must be refused: {errs:?}"
    );
}

/// [actor-spawn-effect] And the capability **propagates**, like any effect: a
/// helper that creates a pool needs `[spawn]`, and so does everyone who calls
/// it. Until 2026-09-16 the gate sat on the `spawn` *expression* only, which
/// made `[spawn]` on `pool` and `watch` decorative — a caller reached them
/// through an undeclared helper.
#[test]
fn the_spawn_capability_propagates_through_calls() {
    let errs = errors(
        "\
fn make() -> Pool {
    return pool(1)
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("`pool` requires the `spawn` capability")),
        "calling a `[spawn]` fn without the capability must be refused: {errs:?}"
    );
}

// ===== [actor-deadlock-cycle] the static deadlock baseline =====
/// [actor-deadlock-cycle] Example 4, the committed baseline: two processes
/// that each park a **gated** continuation on the other's answer. Neither
/// handler is wrong on its own — the bug is a property of the pair, and it is
/// interleaving-dependent, so it is the possibility that is reported.
#[test]
fn two_processes_that_gate_on_each_other_are_refused() {
    let errs = errors(
        "\
actor effect OrderApi {
    send fn place(id: Int, out: Reply<Str>) => !out
    send fn open_orders(id: Int, out: Reply<Int>) => !out
}

actor effect CreditApi {
    send fn credit(id: Int, out: Reply<Bool>) => !out
}

handler Orders(credit: Addr<CreditApi>) of OrderApi {
    mailbox { capacity: 1 }

    send fn place(id: Int, out: Reply<Str>) {
        credit.credit(id, replyto! placed(out))
    }
    send fn placed(out: Reply<Str>, ok: Bool) {
        out.send(\"placed\")
    }
    send fn open_orders(id: Int, out: Reply<Int>) {
        out.send(0)
    }
}

handler Credit(orders: Addr<OrderApi>) of CreditApi {
    mailbox { capacity: 1 }

    send fn credit(id: Int, out: Reply<Bool>) {
        orders.open_orders(id, replyto! counted(out))
    }
    send fn counted(out: Reply<Bool>, n: Int) {
        out.send(true)
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("wait for each other")
            && m.contains("OrderApi")
            && m.contains("CreditApi")),
        "the gate cycle must be an error naming both protocols: {errs:?}"
    );
}

/// [waitfor-effect] [actor-deadlock-cycle] The third edge kind, and it comes
/// from a **signature**: a handler that declares `[waitfor]` serves no message
/// while it waits, so a cycle through it deadlocks exactly as a gate cycle
/// does — and a dedicated thread does not save it, because the fulfilment has
/// to route back through the stalled mailbox. This is the hazard T-5 records
/// as the reason placement alone is not enough.
#[test]
fn a_declared_waitfor_closes_a_wait_cycle() {
    let errs = errors(
        "\
actor effect TimerApi {
    send fn after(ms: Int, out: Reply<Int>) => !out
}

actor effect ClockApi {
    send fn now(out: Reply<Int>) => !out
}

handler Timing(clock: Addr<ClockApi>) of TimerApi {
    mailbox { capacity: 1 }

    send fn after(ms: Int, out: Reply<Int>) {
        clock.now(replyto! fired(out))
    }
    send fn fired(out: Reply<Int>, at: Int) {
        out.send(at)
    }
}

handler Clocking() [TimerApi, waitfor] of ClockApi {
    mailbox { capacity: 1 }

    send fn now(out: Reply<Int>) {
        let at = waitfor fired: Reply<Int> {
            after(0, fired)
        }
        out.send(at)
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("wait for each other")
            && m.contains("declares `[waitfor]`")
            && m.contains("TimerApi")
            && m.contains("ClockApi")),
        "the block cycle must be an error naming the declaration: {errs:?}"
    );
}

/// [actor-deadlock-cycle] The rule the refusal keys on: a **bare** `replyto`
/// leaves the mailbox open, so a cycle where one side keeps serving is not a
/// deadlock. It is still the back-pressure class, so what is left is the
/// warning — the program compiles.
#[test]
fn one_ungated_side_downgrades_the_cycle_to_a_warning() {
    let src = "\
actor effect OrderApi {
    send fn place(id: Int, out: Reply<Str>) => !out
    send fn open_orders(id: Int, out: Reply<Int>) => !out
}

actor effect CreditApi {
    send fn credit(id: Int, out: Reply<Bool>) => !out
}

handler Orders(credit: Addr<CreditApi>) of OrderApi {
    mailbox { capacity: 1 }

    send fn place(id: Int, out: Reply<Str>) {
        credit.credit(id, replyto! placed(out))
    }
    send fn placed(out: Reply<Str>, ok: Bool) {
        out.send(\"placed\")
    }
    send fn open_orders(id: Int, out: Reply<Int>) {
        out.send(0)
    }
}

handler Credit(orders: Addr<OrderApi>) of CreditApi {
    mailbox { capacity: 1 }

    send fn credit(id: Int, out: Reply<Bool>) {
        orders.open_orders(id, replyto counted(out))
    }
    send fn counted(out: Reply<Bool>, n: Int) {
        out.send(true)
    }
}
";
    assert!(
        errors(src).is_empty(),
        "an ungated side is not a deadlock: {:?}",
        errors(src)
    );
    assert!(
        warnings(src)
            .iter()
            .any(|m| m.contains("send to each other in a cycle")),
        "the load-conditioned cycle must still be reported: {:?}",
        warnings(src)
    );
}

/// [actor-deadlock-cycle] **Interception is exempt**, and it has to be: a
/// handler of `E` declaring `[E]` is the phase's showcase pattern — a policy
/// wrapper around the process already serving `E` — and it is an `E → E` edge
/// by construction. The dependency binds strictly *outward*, so the chain ends
/// at the innermost instance and no instance ever waits for itself.
#[test]
fn an_intercepting_handler_that_gates_is_not_a_cycle() {
    let src = "\
handler Caching() [Counter] of Counter {
    mailbox { capacity: 1 }

    send fn bump(n: Int) {
        bump(n)
    }
    send fn total(out: Reply<Int>) {
        total(replyto! answered(out))
    }
    send fn answered(out: Reply<Int>, n: Int) {
        out.send(n)
    }
}
";
    assert!(
        errors(src).is_empty(),
        "an own-effect dependency must not close a cycle: {:?}",
        errors(src)
    );
    assert!(
        warnings(src).is_empty(),
        "and it must not warn either: {:?}",
        warnings(src)
    );
}

/// [actor-deadlock-cycle] A one-directional pair is the common, correct
/// topology and must stay silent: the requester gates, the answerer only ever
/// fulfils tokens, and a fulfil contributes no edge at all.
#[test]
fn a_one_way_request_response_pair_is_silent() {
    let src = "\
actor effect Waiting {
    send fn ask()
    send fn got(n: Int) => !n
}

handler Asking(c: Addr<Counter>) of Waiting {
    mailbox { capacity: 1 }

    send fn ask() {
        c.total(replyto! got())
    }
    send fn got(n: Int) {}
}
";
    assert!(errors(src).is_empty(), "{:?}", errors(src));
    assert!(warnings(src).is_empty(), "{:?}", warnings(src));
}

// ===== [actor-mailbox] the actor settings slot =====

/// [actor-mailbox] The slot in its two shapes (user decision 2026-09-16): a
/// literal bound, and one taken as a constructor parameter — which is what
/// "optionally exposed as a constructor argument" means, and needs no new
/// grammar because the block's expressions are checked in the constructor's
/// scope.
#[test]
fn a_mailbox_may_be_literal_or_a_constructor_parameter() {
    let errs = errors(
        "\
handler Fixed() of Log {
    mailbox { capacity: 16 }
    send fn note(what: Str) {}
}

handler Sized(room: Int) of Log {
    mailbox { capacity: room }
    send fn note(what: Str) {}
}

fn main() [use, spawn] {
    let a = spawn Fixed() on pool(1)
    let b = spawn Sized(4) on pool(1)
}
",
    );
    assert!(errs.is_empty(), "both mailbox shapes must check clean: {errs:?}");
}

/// [actor-mailbox] It is **required** on a handler of an actor effect: the
/// queue bound had no default at the spawn site and has none here either — a
/// bound the compiler picked would be a performance cliff nobody wrote. What
/// changed is only *where* it is said, and by whom.
#[test]
fn an_actor_handler_must_declare_its_mailbox() {
    let errs = errors(
        "\
handler Quiet() of Log {
    send fn note(what: Str) {}
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("has a mailbox and has to say how deep it is")
            && m.contains("mailbox { capacity: 16 }")),
        "expected the missing-mailbox error: {errs:?}"
    );
}

/// [actor-mailbox] …and refused on a handler of a **plain** effect, where
/// there is no queue: its members run on the caller's thread. This is the
/// half of the old objection that survives — a mailbox means nothing to a
/// synchronously bound handler — and it is now a diagnostic instead of an
/// inert setting.
#[test]
fn a_plain_handler_has_no_mailbox() {
    let errs = errors(
        "\
effect Plain {
    fn ask() -> Int
}

handler Answering() of Plain {
    mailbox { capacity: 4 }
    fn ask() -> Int {
        return 1
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("only a handler of an `actor effect` has a mailbox")),
        "expected the plain-effect refusal: {errs:?}"
    );
}

/// [actor-mailbox] The scope is **constructor parameters only**: the bound is
/// wanted before the actor exists — the scheduler needs it at spawn time,
/// ahead of every state initialiser — so a state field is not in scope.
#[test]
fn a_mailbox_sees_constructor_parameters_only() {
    let errs = errors(
        "\
handler Loud() of Log {
    room: Int = 8
    mailbox { capacity: room }
    send fn note(what: Str) {}
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("no variable or function named `room`")),
        "a state field must not be in scope: {errs:?}"
    );
}

/// [actor-mailbox] The slot only *reads* — and that needs no rule of its own:
/// [effect-state-store] already refuses consuming what the handler was built
/// with, and names `copy` as the remedy. A `Copy` scalar is exempt, which is
/// why the ordinary `capacity: room` shape above is fine [copy-scalar-free].
#[test]
fn a_mailbox_may_not_consume_a_parameter() {
    let errs = errors(
        "\
fn measure(names: List<Str>) [] -> Int => !names {
    return 1
}

handler Wasteful(names: List<Str>) of Log {
    mailbox { capacity: measure(names) }
    send fn note(what: Str) {}
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("cannot move `names`")
            && m.contains("constructor parameter of handler `Wasteful`")),
        "expected the consuming refusal: {errs:?}"
    );
}

/// [actor-mailbox] The slot is a struct literal, so the field diagnostics are
/// the ordinary ones — which is the whole point of the slot reading: a future
/// setting is a *field* on `Mailbox` rather than new grammar.
#[test]
fn the_mailbox_slot_is_an_ordinary_struct_literal() {
    let errs = errors(
        "\
handler Odd() of Log {
    mailbox { depth: 4 }
    send fn note(what: Str) {}
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("has no field `depth`")),
        "expected the unknown-field error: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("missing field `capacity`")),
        "expected the missing-field error: {errs:?}"
    );
}
