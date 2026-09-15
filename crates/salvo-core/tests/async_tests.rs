//! [async-spawn-expr] [async-replyto] [async-waitfor] [async-use-addr] The
//! asynchronous surface's *checker* rules: what a `spawn` needs to be legal,
//! what a `replyto` may target, where a `waitfor` may stand, and how an `Addr`
//! is called. The forms' syntax is tested in `salvo-syntax`; the types they
//! produce and consume are `core.process`'s ([async-types]).
//!
//! The shape every test here rests on: a process is a handler bound
//! asynchronously, so almost every rule is one `use` already had, moved to
//! the spawn site.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// A stand-in for `core.process` and the little of std these tests need.
/// `intrinsic` is std-only [intrinsic-std-only], so it is loaded as a std
/// file rather than pasted into the source under test.
const STD_PRELUDE: &str = "\
intrinsic type Int
intrinsic type Str
intrinsic type Bool
intrinsic type List<T> canbe Mut
intrinsic fn discard<T canbe linear>(value: T) [] -> None => !value
intrinsic type Addr<E>
linear intrinsic type Reply<T>
intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply, !value
intrinsic fn array_of<T>(...elems: T[]) [] -> T[]
intrinsic type Pool
intrinsic fn pool(size: Int) [spawn] -> Pool => size
";

/// The effects and handlers the cases share: a `Counter` protocol with a
/// request/response pair, a `Log` dependency, and handlers for both.
const PRELUDE: &str = r#"
async effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

async effect Log {
    send fn note(what: Str) => !what
}

handler Printing() of Log {
    send fn note(what: Str) {}
}

handler Counting() [Log] of Counter {
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
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
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
fn main() [use, spawn] {
    let logger = spawn Printing() capacity 4 on pool(1)
    let counter = spawn Counting() use logger capacity 16 on pool(2)
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

// ===== [async-spawn-expr] The spawn =====

/// [async-spawn-effect] The capability gate: creating a process is something
/// a function must declare, exactly as registering a handler is.
#[test]
fn a_spawn_requires_the_spawn_capability() {
    let errs = errors(
        "\
fn start() [use] -> Int {
    let c = spawn Printing() capacity 1 on pool(1)
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
fn main() [use, spawn] {
    use Printing()
    let counter = spawn Counting() capacity 1 on pool(1)
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
fn main() [use, spawn] {
    let counter = spawn Printing() use Printing() capacity 1 on pool(1)
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
fn main() [use, spawn] {
    let c = spawn Counting() use Counting() capacity 1 on pool(1)
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("cannot be constructed in a spawn's `use` clause")),
        "expected the nested-dependency refusal: {errs:?}"
    );
}

/// The two required clauses are typed: a mailbox bound is an `Int`, and a
/// spawn runs on a `Pool` — which `pool(n)` is the way to get.
#[test]
fn the_capacity_and_pool_clauses_are_typed() {
    let errs = errors(
        "\
fn main() [use, spawn] {
    let a = spawn Printing() capacity \"lots\" on pool(1)
    let b = spawn Printing() capacity 1 on 7
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("mailbox bound is an `Int`")),
        "expected the capacity type error: {errs:?}"
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("runs on a `Pool`") && m.contains("on pool(2)")),
        "expected the pool type error naming the remedy: {errs:?}"
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
fn main() [use, spawn] {
    let c: Int = spawn Counting() use Printing() capacity 1 on pool(1)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("Addr<Counter>")),
        "the spawn's type must be `Addr<Counter>`: {errs:?}"
    );
}

// ===== [async-use-addr] Sends through an addr =====

/// A dot-call through an addr resolves against the effect the process serves,
/// with the receiver naming *where* the message goes rather than being the
/// first argument.
#[test]
fn a_addr_call_resolves_against_the_served_effect() {
    let errs = errors(
        "\
fn main() [use, spawn] {
    let counter = spawn Counting() use Printing() capacity 1 on pool(1)
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
fn main() [use, spawn] {
    let counter = spawn Counting() use Printing() capacity 1 on pool(1)
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
fn main() [use, spawn] {
    let counter = spawn Counting() use Printing() capacity 1 on pool(1)
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
fn main() [use, spawn] {
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

// ===== [async-replyto] The mint =====

/// `replyto` targets a member of the handler it is written in, so it is
/// illegal outside one — and the diagnostic names `main`'s alternative.
#[test]
fn replyto_is_illegal_outside_a_handler() {
    let errs = errors(
        "\
fn main() [use, spawn] {
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
async effect Ask {
    send fn go(out: Reply<Int>) => !out
    fn ready() -> Bool
}

handler Asker() [Counter] of Ask {
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
async effect Ask {
    send fn go() 
    send fn arrived(id: Int, sum: Int) => !id, !sum
}

handler Asker() [Counter] of Ask {
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
async effect Ask {
    send fn go()
    send fn arrived(id: Int, sum: Int) => !id, !sum
    send fn wrong(id: Int, text: Str) => !id, !text
}

handler Asker() [Counter] of Ask {
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
async effect Ask {
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

// ===== [async-waitfor] The bridge =====

/// `waitfor` blocks a real thread, so only `main` may write one.
#[test]
fn waitfor_is_legal_only_in_main() {
    let errs = errors(
        "\
fn helper() [use, spawn] -> Int {
    let counter = spawn Counting() use Printing() capacity 1 on pool(1)
    return waitfor out: Reply<Int> {
        counter.total(out)
    }
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("only\n `main` may do") || m.contains("only `main` may do")),
        "expected the main-only rule: {errs:?}"
    );
}

/// Its binding is a reply token; anything else is a mistake about what the
/// form does.
#[test]
fn waitfor_binds_a_reply_token() {
    let errs = errors(
        "\
fn main() [use, spawn] {
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
fn main() [use, spawn] {
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
fn main() [use, spawn] {
    let counter = spawn Counting() use Printing() capacity 1 on pool(1)
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

/// [async-use-addr] A receiver is any **place** whose type is an addr, not just
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

fn main() [use, spawn] {
    let counter = spawn Counting() use Printing() capacity 1 on pool(1)
    let r = Registry { counter: counter }
    r.counter.bump(1)
    let many: Addr<Counter>[] = array_of(r.counter)
    many[0].bump(2)
}
",
    );
    assert!(errs.is_empty(), "an addr place must be a receiver: {errs:?}");
}

/// [async-spawn-expr] [async-waitfor] [async-replyto] The asynchronous forms
/// stop at a **closure**: a function value's body runs wherever it is called,
/// a fn type cannot declare `spawn`, `waitfor` needs `main`'s own thread, and
/// a continuation belongs to the handler that minted it. Each diagnostic says
/// so in the closure's terms rather than repeating the general rule.
#[test]
fn the_asynchronous_forms_stop_at_a_lambda() {
    let spawned = errors(
        "\
fn main() [use, spawn] {
    let f = () -> spawn Printing() capacity 1 on pool(1)
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
fn main() [use, spawn] {
    let counter = spawn Counting() use Printing() capacity 1 on pool(1)
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
                && m.contains("write the bridge in `main`")),
        "expected the lambda-waitfor refusal: {waited:?}"
    );

    let minted = errors(
        "\
async effect Ask {
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
async effect Sink {
    send fn put(a: Int) => !a
    send fn put(a: Int, b: Int) => !a, !b
}

handler Dropping() of Sink {
    send fn put(a: Int) {}
    send fn put(a: Int, b: Int) {}
}

fn main() [use, spawn] {
    let s = spawn Dropping() capacity 1 on pool(1)
    s.put(1)
    s.put(1, 2)
}
",
    );
    assert!(ok.is_empty(), "arity must settle the overload: {ok:?}");

    let tie = errors(
        "\
async effect Sink {
    send fn put(a: Int) => !a
    send fn put(a: Str) => !a
}

handler Dropping() of Sink {
    send fn put(a: Int) {}
    send fn put(a: Str) {}
}

fn main() [use, spawn] {
    let s = spawn Dropping() capacity 1 on pool(1)
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

// ===== [async-self-send] `k@self(args)` =====

/// The form's point is *ordering*: an unqualified call would run `k` inside
/// this activation, and a self-send runs it as its own later one — which is
/// how "finish this, then continue with `k`" is written. Legal in a member
/// body, with the arguments checked like any send's.
#[test]
fn a_member_can_send_to_its_own_process() {
    let errs = errors(
        "\
async effect Work {
    send fn start(n: Int) => !n
    send fn step(n: Int) => !n
}

handler Working() of Work {
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
async effect Work {
    send fn start(n: Int) => !n
    fn ready() -> Bool
}

handler Working() of Work {
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
fn main() [use, spawn] {
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
async effect Work {
    send fn start(n: Int) => !n
    send fn step(n: Int) => !n
}

handler Working() of Work {
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

/// [async-self-send] `self` is **contextual**, not reserved: it means the
/// enclosing handler only immediately after `@`, so it stays an ordinary name
/// elsewhere — and a local of that name can never shadow the form, which is
/// what the selector spelling buys over the `self.k(…)` receiver it replaced.
#[test]
fn self_stays_an_ordinary_name_away_from_the_selector() {
    let errs = errors(
        "\
handler Working() of Log {
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
            .any(|m| m.contains("send it to this process instead: `bump@self(…)`")),
        "expected the diagnostic to name the self-send: {errs:?}"
    );
}

// ===== [async-effect-kind] [async-sendable] The effect kind =====

/// The kind is declared, not diagnosed (user decision 2026-09-15, EU-5): an
/// author choosing between `effect` and `async effect` is deciding whether the
/// protocol crosses threads, so `send fn` needs the async kind and the
/// diagnostic names the marker.
#[test]
fn a_send_member_needs_an_async_effect() {
    let errs = errors(
        "\
effect Plain {
    send fn nope(n: Int) => !n
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("needs an `async effect`") && m.contains("async effect Plain")),
        "expected the kind requirement: {errs:?}"
    );
}

/// And inside an `async effect`, everything a seam cannot carry is refused
/// **at the declaration**, where the choice is being made: a kept parameter, a
/// `Mut` parameter, and a non-sendable payload — each naming the law rather
/// than the symptom.
#[test]
fn an_async_effect_refuses_what_cannot_cross_a_seam() {
    let errs = errors(
        "\
struct Job {
    name: Str,
    run: () -> Int
}

async effect Bad {
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

/// [async-sendable] The other half of C-4(a): a `proj` view borrows the
/// sender's value, so it cannot cross either.
#[test]
fn a_view_is_not_sendable() {
    let errs = errors(
        "\
struct Window {
    over: proj List<Int>
}

async effect Peek {
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

/// [async-effect-kind] The binding gate, which is what closes the design's
/// carried named question: a plain effect is **never** process-backed, so
/// `spawn`, `use addr` and even naming `Addr<E>` require the async kind. The
/// last is reported where the type is written, before any spawn exists.
#[test]
fn only_an_async_effect_can_be_bound_to_a_process() {
    let errs = errors(
        "\
effect Plain {
    fn ping() -> Int
}

handler Pinging() of Plain {
    fn ping() -> Int {
        return 1
    }
}

fn hold(a: Addr<Plain>) -> Int => a {
    return 1
}

fn main() [use, spawn] {
    let p = spawn Pinging() capacity 1 on pool(1)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("`Addr<Plain>` needs an `async effect`")),
        "expected the addr-type gate: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("`spawn` cannot bind `Plain` to a process")
            && m.contains("no mailbox")),
        "expected the spawn gate: {errs:?}"
    );
}

/// A **plain** handler binding of an async effect stays legal — that is
/// Example 6's binding swap, and the reason the kind sits on the effect rather
/// than on the handler.
#[test]
fn an_async_effect_can_still_be_used_synchronously() {
    let errs = errors(
        "\
fn main() [use, spawn] {
    use Printing()
    note(\"inline\")
}
",
    );
    assert!(errs.is_empty(), "the binding swap must stay legal: {errs:?}");
}
