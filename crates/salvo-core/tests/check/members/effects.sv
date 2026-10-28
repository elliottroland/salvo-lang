// [effect-not-data] [handler-not-value] [effect-handler-deps] Effects and
// handlers are not data: an effect names a capability, a handler instance comes
// from `use`, and a handler's dependencies are an effect list.

effect Counter {
    fn bump() -> Int
}

handler MemCounter of Counter {
    state {
        n: Int = 0
    }

    fn bump() -> Int {
        n = n + 1
        return n
    }
}

// [actor-effect-kind] An `eff[…]` needs a *process* protocol behind it, so the
// handle cases below have one to name; `Counter` stays plain.
actor effect Chatter {
    send fn say(what: Str) => !what
}

effect Logger {
    fn log(m: Str) -> None => m
}

// [effect-handler] A state field's initializer is checked against its
// declared type, like a struct field's default. Until 2026-09-04 it was not
// checked *at all*, which both hid type errors and left the emitters with no
// types for the expression, which an intrinsic lowering may need in order to
// spell a type out [backend-intrinsic].
effect Sink {
    fn kept() -> Int
}

handler Bin of Sink {
    state {
        //~ error[default-fits:mismatch]
        held: Int = "not an int"
    }

    fn kept() -> Int {
        return held
    }
}

// A well-typed initializer is accepted.
fn well_typed_state() [use] -> Int {
    use MemCounter
    return bump()
}

// ([effect-not-data] An effect in each data position is `member_tests.rs`'s:
// a return type or a `let` annotation of an effect also reports the mismatch
// against it.)

// [effect-not-data] [effect-eff-handle] A first-class effect handle, `eff[E]`,
// carries the effect `E` as a holder's capability over it — and is an ordinary
// value, accepted in every data position. This is what lets a process and a
// locally `use`d handler stand behind one name.
struct Holder {
    c: eff[Chatter]
}

fn handle_parameter(c: eff[Chatter]) -> Int => c {
    return 1
}

fn handle_return(c: eff[Chatter]) -> eff[Chatter] => !c {
    return c
}

// And the handle is *only* a handle: a bare effect is still refused as any
// other type argument, so nothing was widened by the handle form.
//~ error[effect-not-data:data-position]
fn in_a_list(xs: List<Counter>) -> Int => xs {
    return 1
}

// [effect-not-data] The two positions that legitimately name an effect — a
// fn's effect list and a handler's `of` clause — keep working.
fn in_an_effect_list() [Counter] -> Int {
    return bump()
}

// [handler-not-value] A handler instance comes from `use`; it is not a value
// to bind, return, or pass. Rust could not render one anyway (`Name()` is not
// a constructor — `E0423`). The diagnostic names `use MemCounter(...)`.
fn handler_as_value() -> Int {
    //~ error[handler-not-value:called]
    let _c = MemCounter()
    return 1
}

// [handler-not-value] `use` itself is unaffected: it validates its own
// constructor call and never goes through the value path.
fn use_registers() [use] -> Int {
    use MemCounter()
    return bump()
}

// [effect-handler-deps] A handler declares its dependencies as an effect list,
// exactly as a fn does: the member bodies may use that effect.
handler CountingLogger [Counter] of Logger {
    fn log(m: Str) -> None => m {
        let _n = bump()
    }
}

// …and a *constructor parameter* of effect type is the plain [effect-not-data]
// error, with no exception left anywhere in the language (user decision
// 2026-09-14: the old names could not be used for anything).
//~ error[effect-not-data:data-position]
handler ParamLogger(counter: Counter) of Logger {
    fn log(m: Str) -> None => m {
    }
}

// [effect-handler-deps] What may *not* be in the list: `use` (a handler
// registers nothing) and `Throw` (a throw needs a delimiter, not a handler),
// and no effect twice.
//~ error[effect-handler-deps:use]
handler H1 [use] of Logger {
    fn log(m: Str) -> None => m {
    }
}

//~ error[effect-handler-deps:throw]
handler H2 [Throw<Str>] of Logger {
    fn log(m: Str) -> None => m {
    }
}

//~ error[effect-handler-deps:duplicate]
handler H3 [Counter, Counter] of Logger {
    fn log(m: Str) -> None => m {
    }
}

// [effect-handler-deps] A handler member cannot call **its own** effect's
// members: self-dispatch does not exist, and the diagnostic says so rather than
// naming two remedies a member cannot use (a member may not declare effects
// [effect-member-no-effects] and may not `use`). Recorded as a gap in
// ROADMAP.md.
handler Twice of Counter {
    fn bump() -> Int {
        //~ error[effect-available:own-effect]
        //~ error[effect-available:own-effect]
        return bump() + bump()
    }
}

// [effect-intercept] A handler *may* depend on the effect it implements — that
// is interception — and the declaration alone says nothing about
// availability: the binding rule lives at the `use`.
handler Wrapper [Counter] of Counter {
    fn bump() -> Int {
        return bump() + 1
    }
}

// [effect-intercept] [effect-use-no-dup] An intercepting `use` **binds
// strictly outward**: it wraps the instance registered before it, so
// registering it with nothing to wrap is the error, in interception's own
// words — and shadowing an instance that *is* there is not.
fn intercept_alone() [use] -> Int {
    //~ error[effect-intercept:nothing-to-wrap]
    use Wrapper()
    return bump()
}

// Over an instance already in scope: accepted, and the *same* effect being
// registered twice is no error [effect-use-no-dup].
fn intercept_wrapped() [use] -> Int {
    use MemCounter()
    use Wrapper()
    return bump()
}

// And it stacks: an interceptor may wrap an interceptor.
fn intercept_twice() [use] -> Int {
    use MemCounter()
    use Wrapper()
    use Wrapper()
    return bump()
}

// [effect-use-no-dup] Shadowing needs no dependency: a second `use` for an
// effect instance already in scope registers a handler that wins for the rest
// of the scope, rather than being rejected (which is what it was until
// 2026-09-14).
fn shadowing() [use] -> Int {
    use MemCounter()
    use MemCounter()
    return bump()
}

// [effect-handler-deps] Dependency *cycles* need no separate check: a
// dependency must already be registered, so a cycle cannot be constructed in
// either order. Verified both ways round, since "it happens to fail" and "it
// cannot be written" are different claims.
effect Alpha {
    fn a(m: Str) -> None => m
}

effect Beta {
    fn b(m: Str) -> None => m
}

handler AlphaViaBeta [Beta] of Alpha {
    fn a(m: Str) -> None => m {
        b(m)
    }
}

handler BetaViaAlpha [Alpha] of Beta {
    fn b(m: Str) -> None => m {
        a(m)
    }
}

fn alpha_first() [use] -> None {
    //~ error[effect-handler-deps:missing]
    use AlphaViaBeta()
    use BetaViaAlpha()
}

fn beta_first() [use] -> None {
    //~ error[effect-handler-deps:missing]
    use BetaViaAlpha()
    use AlphaViaBeta()
}

// [effect-handler-deps] Dependencies are supplied by the compiler from the
// enclosing scope, so they are not written at the `use` site — and one must be
// available there. Registered *after* its dependency: fine, and no argument is
// written.
fn dependency_in_scope() [use] -> None {
    use MemCounter()
    use CountingLogger()
    log("x")
}

// Without the dependency in scope: rejected at the registration.
fn dependency_missing() [use] -> None {
    //~ error[effect-handler-deps:missing]
    use CountingLogger()
    log("x")
}

//--- the Addr type name is not user-spellable
// [effect-eff-handle] The old spelling `Addr<E>` is no longer a type a user
// writes: `Addr` is the compiler's internal name for the handle, and the
// surface is `eff[E]`. It is a plain parse error naming the replacement.
actor effect Chatter {
    send fn say(what: Str) => !what
}

//~ error[effect-eff-handle:addr-type]
fn f(c: Addr<Chatter>) -> Int => c {
    return 1
}
