# Backends

One of the aims of Salvo is to make it easy to integrate Salvo code with the backend code. To achieve this, the Salvo compiler builds an internal representation (in Rust), and passes this on to the configured backend implementation to write out the relevant target source code. In order to support this, we distinguish between two layers: the `intrinsic` layer, which is the compiler's, and the `platform` layer, which is yours. The core library is entirely intrinsic; everything an application needs from its target language is a platform effect.

## Intrinsic

The `intrinsic` layer sits in a backend specific module inside the compiler. This handles complex language-specific logic, and core functionality: how to encode union types, what the `None` type transpiles to in different cases, how to pass parameters to functions, how function naming works, how imports are handled, and more. These can only be changed by making changes to the compiler itself. Anything involving syntax will appear here, and all `intrinsic` backend definitions are declared as part of the standard library (defined in `std`).

`intrinsic` is the standard library's alone. Customer code cannot declare one, because there would be no lowering in any backend to give it meaning — an `intrinsic` with no compiler support behind it is a promise nothing keeps. Application code reaches the target language the other way, through the `platform` declarations — a `platform effect`, or a `platform handler` implementing an ordinary effect; that is the single interop path. This is also the one exception to a plain structural rule: a top-level `fn` must have a body and a `type` must have a definition (`= ...`). The bodyless declaration forms customer code does have are the `platform` ones, whose contract the *build* fulfils; `intrinsic` (and the bodyless `intrinsic handler`) is what lets the standard library state a contract the compiler fulfils in place of one.

For example, the basic types (`Int`, `Str`, `List<T>`, ...) are declared as `intrinsic type`s, and each backend maps them natively:

```
intrinsic type Str
```

When building the compiler, _all_ `intrinsic` declarations must be handled by _every_ backend module.

Functions can be intrinsic too. An `intrinsic fn` carries the signature and deductions the checker uses and has no body; each backend lowers calls to it directly, seeing the resolved argument type at every call site. That is what makes type-directed lowering possible where one generic template could not express it — the standard library's `copy` is the canonical example:

```
intrinsic fn copy<T>(value: proj T) -> T => value
```

A backend that does not implement an intrinsic fn, or cannot lower it for a particular argument type, reports a compile-time error — never wrong code.

The standard library's collection and string surface (`list`, `mut_list_of`, `add`, `get`, `first`, `size`, `iter`, and the string functions from `char_at` to `split`, `trim`, `join` and `parse_int`) is intrinsic for the same reason, which is what keeps `size(xs)` compiling to `xs.size` in Kotlin and `(xs.len() as i32)` in Rust rather than to a wrapper function nobody wants. Because the lowering sees the *resolved* declaration, the three `size` overloads — on `Str`, on `List<T>`, and on an array — are three separate lowerings rather than one template guessing from arity.

Handlers can be intrinsic as well: `intrinsic handler StdOutConsole of Console` is bodyless in Salvo, and each backend emits a real class or trait impl for it.

The `Mut` auto-qualifier is also handled at this level: a type declaration can opt into it with `canbe Mut` (`intrinsic type List<T> canbe Mut`), and each backend decides what `Mut` means. Kotlin maps `Mut List<T>` to `MutableList<T>` and `Mut Str` to `StringBuilder` — the one place a qualifier survives erasure — while Rust maps both `List<T>` and `Mut List<T>` to `Vec<T>`, and both `Str` and `Mut Str` to `String`, because there mutability shows up in bindings and references instead. Where the two types really differ, a *drop* of the `Mut` is a conversion (`.toString()` for a Kotlin builder) and the compiler records the drop for the backend to render; where they do not — `MutableList<T>` is a `List<T>` — it renders nothing.

## Platform

The `platform` layer is where an application reaches its target language. Where `intrinsic` is the compiler's, `platform` is yours: you declare what you need from the host, and the compiler generates an interface for the host to implement.

A platform declaration is an *effect*:

```
platform effect Telemetry {
    fn record(name: Str, value: Int) [] -> None => name, value
}
```

That is an ordinary effect in every respect except where its implementation comes from. A function that records telemetry declares `[Telemetry]`, its callers declare it too, and the value threads through exactly as any handler would:

```
fn work(n: Int) [Telemetry] -> Int {
    record("work", n)
    return n + 1
}
```

Grouping the functions under an effect, rather than declaring them one at a time, is what makes the interop boundary something you choose: one effect for telemetry, another for storage, each generating its own interface. It also gives the implementation somewhere to keep state and dependencies, because the host constructs it.

The compiler generates the interface next to the rest of the emitted code — `interface Telemetry` in Kotlin, `pub trait Telemetry` in Rust — and nothing else. There is no handler to write in Salvo, and writing one is an error: the host's implementation *is* the handler.

Because the instance is constructed outside the Salvo program, it cannot be registered with `use`. It arrives as a parameter instead, and that changes who owns the entry point: a `main` that declares a platform effect is emitted as `salvoMain` (Kotlin) or `salvo_main` (Rust), taking one parameter per platform effect it declares, and the *host's* `main` constructs the implementations and calls it.

You do not write that file from scratch. `salvo platform generate` writes it for you:

```bash
salvo platform generate --backend kotlin --src ./my_project
```

The host code lives in a **platform root** the project's `salvo.toml` names — there is no default, so a project with platform code says where it is:

```toml
[build]
platform = "salvo/platform"    # every backend's platform files

[kotlin]
platform = "kotlin"            # or one root per backend, over the shared one
```

Inside a root the files mirror the source layout: `main.kt` implements the platform effects declared in `main.sv`, `app/entry.rs` those of `app/entry.sv`. Both languages can sit side by side in one root — a build only ever picks up the extension of the backend it is compiling for — so one source tree stays buildable for both targets; or each backend can have a root of its own. A program with platform declarations and no root for a backend it builds is refused, naming the key to add. The examples below assume `platform = "platform"`.

The generated file is a skeleton: one class per platform effect, implementing the generated interface, with every member stubbed, plus the `main` the toolchain will run:

```kotlin
// platform/main.kt, as generated
package salvo.platform.main

import salvo.main.*

class TelemetryHost : Telemetry {
    override fun record(name: String, value: Int) {
        TODO("implement Telemetry.record")
    }
}

fun main() {
    salvoMain(TelemetryHost())
}
```

Fill in the bodies and `salvo run` works. The file is generated **once**: run the command again and it reports that the file exists and leaves it alone, because from that point on it is yours. Forgetting to run it at all is an ordinary compile error that names the command — a program whose `main` needs a platform effect has no entry point without a host.

This is the point of the design: because the interface is generated and the implementation is real target-language code, the target's own compiler checks the two against each other. Add a member and the implementation fails to compile until you write it; remove one and the leftover override fails; change a signature and the mismatch is a type error. Nothing needs to be validated by Salvo, and nothing can drift silently — which is also why the generator never has to touch the file twice.

A Salvo handler may *depend* on a platform effect, which is how a handler written in Salvo reaches the host:

```
handler AuditLogger [Telemetry] of Logger {
    fn log(message: Str) -> None => message {
        record(message)
    }
}
```

Two restrictions follow from the host implementing one concrete interface: neither a platform effect nor its members may be generic. Member names may be shared with other effects like any effect's, and overloaded within the effect like any effect's (see "Two effects, one member name").

## A host implementation of an ordinary effect

A `platform effect` says *the whole effect is the host's*. Sometimes the effect is Salvo's own — declared here, handled here, with several handlers — and only one of those handlers is host code: the one that actually touches the outside world. That handler is a `platform handler`:

```
effect RawClock {
    fn raw_now() [] -> Int
}

platform handler HostRawClock of RawClock
```

It is bodyless, because its members live in the target language, and it is otherwise an ordinary handler: registered with `use`, one instance per registration, constructor parameters passed through to the host class.

```
fn main() [use] {
    use HostRawClock()       // constructs the host's class
    use DefaultClock()       // ordinary Salvo, depends on RawClock
    ...
}
```

The implementation goes in the platform root as a **platform template** (below), and `salvo platform generate` writes the skeleton for it — the handler's signature and each member of its effect, stubbed:

```kotlin
// platform/main.sv.kt, as generated
`platform handler HostRawClock of RawClock` {
    `fn raw_now() -> Int` {
        TODO("implement RawClock.raw_now")
    }
}
```

The compiler emits the class, named after the *handler* — the `use` site constructs that name, so it is not the host's to choose. A hand-written host class in `platform/main.kt` still works, and `generate` leaves a module that has one alone.

Nothing else moves: `main` stays the program's entry point, because the instance is constructed *inside* the program rather than handed to it. Constructor parameters are how a host implementation is configured — `platform handler HostS3(bucket: Str) of Store`, registered as `use HostS3("my-bucket")`, becomes a class with a `bucket` parameter.

Three restrictions, each following from the implementation not being Salvo's:

* **No body in Salvo** — no members. Its state is laid out by the compiler when a template implements it.
* **No effect dependencies.** A handler's dependencies are supplied to its *members*, and these members are host code, which performs no Salvo effect: the host reaches the outside world directly. Write an ordinary Salvo handler that depends on this one's effect when something has to sit in between — `handler DefaultClock [RawClock] of Clock` is exactly that.
* **Not generic**, for the reason a platform effect is not: the host writes one concrete class.

### The thread-safety contract: `threadsafe`

A `use` of a platform handler is a handle like any handler's, and a handle locks a *stateful* handler's members and runs a *stateless* handler's on the shared instance. Which a host is, is a fact about the *host class*, which the compiler cannot see — so the declaration states it:

```
threadsafe platform handler HostTcpTransport(port: Int) of Transport
platform handler HostRawFs of RawFs
```

**Without the word, the compiler serializes the instance** — every member runs under one lock, on both backends — so a host that keeps plain fields and never thought about threads works identically everywhere and pays only the lock. `HostRawFs` is written that way. **With `threadsafe`, the host promises it may be entered concurrently** (it synchronizes internally, or holds nothing that needs it) and the compiler shares the raw instance with no lock. On Rust the promise is half-checked: the host's members take `&self`, so a field that is not `Sync` does not compile; on Kotlin the host is trusted.

`salvo platform generate` prints whichever contract the declaration made into the skeleton it writes, so the person implementing the host signs what the compiler assumes — and regenerating after adding or removing the word changes the skeleton's shape (the Rust receivers switch between `&self` and `&mut self`).

A member may take a `Reply<T>` — a continuation — and **return at once**, answering later from host code. The host calls `hosted()` on the token, which tells the scheduler the answer is coming from outside (so a `waitfor` on it is not reported as a deadlock), and completes it with `send(value)` from any thread, exactly once. This is how an asynchronous host API — a Kotlin coroutine, a Rust future — becomes a Salvo call that blocks no worker:

```
platform effect Slow {
    fn later(n: Int, done: Reply<Int>) [] -> None => !n, !done
}
```
```kotlin
override fun later(n: Int, done: salvo.SalvoReply) {
    val host = done.hosted()
    Thread { Thread.sleep(50); host.send(n + 1) }.start()
}
```

On Rust a host reply dropped without being sent is reported to the pool's fault sink; the JVM cannot observe that, so on Kotlin a lost reply leaves its waiter waiting.

A host file may use any library of its language, provided the manifest declares it — `[rust] crates`, `[kotlin] libs` and `artifacts`; see [Modules](Modules.md) "Host libraries". Rust then builds with `cargo` instead of bare `rustc`, and Kotlin puts the declared jars on the classpath.

The two forms answer different questions. Use a `platform effect` when the *capability* is the host's and the program is a guest in the host's process — the host constructs everything and owns `main`. Use a `platform handler` when the capability is the language's, several implementations exist, and one of them is host code: a real filesystem beside an in-memory one, a host clock beside a fake, an S3-backed store beside a local directory. The standard library uses the second form itself, and ships its host classes the same way — under `std`'s own platform root, one file per backend.

# Specific backend details

## Platform templates

A platform handler or a function can be implemented in host code that still speaks Salvo: a **platform template**, `<module>.sv.kt` for Kotlin and `.sv.rs` for Rust, under the backend's platform root beside its other files. `salvo platform generate` writes one for every module with platform handlers or bodiless functions. The `.sv` file declares the Salvo side with no body:

```
// counter.sv
fn shout(g: Greeting) [] -> Str => g

effect Counter {
    fn next(step: Int) -> Int => step
}

platform handler HostCounter(start: Int) of Counter {
    at: Int = start
}
```

and the template is ordinary host code in which anything between backticks is Salvo, rendered by the compiler:

```kotlin
// platform/counter.sv.kt
`fn shout(g: Greeting) -> Str` {
    return if (`g.loud`) `g.text`.uppercase() else `g.text`
}

`platform handler HostCounter(start: Int) of Counter` {
    private val log = mutableListOf<Int>()

    `fn next(step: Int) -> Int` {
        `at` += step
        log.add(`at`)
        return `at`
    }
}
```

A marker that declares something — `` `fn …` ``, `` `platform handler …` ``, and on Rust `` `struct H` `` for host fields — repeats the full signature, which the compiler checks against the `.sv` declaration and replaces with the host header it emits. Every other marker renders Salvo: a field read (`` `g.text` ``), a type (`` `AwsError` ``), a struct built from host values (`` `AwsError { code: @code, message: @{e.message ?: ""} }` ``, where `@name` and `@{…}` go back to host code), and a value wrapped into a union host code cannot see (`` `ok(value) : Ok Out | Err Str` `` — or just `` return `ok(value)` ``, which takes the function's return type). `` `v : Attr` `` declares a host name with a Salvo type, so later markers can read `` `v.data_type` ``.

A union is written once: after the colon, a lowercase name is a **place** whose type the value takes — a declared name, a parameter or a state field — and `return` the function's return type. The branches of host control flow then name the place instead of repeating its type:

```kotlin
val `answer : Ok Out | Err Checked<Failure>` = try {
    `ok(value) : answer`
} catch (e: Exception) {
    `err(checked<Failure>(failure)) : answer`
}
```

A type alias in the `.sv` file is the other way to shorten a long type. Strings and comments are never scanned; ```` `` ```` is a literal backtick.

The handler's Salvo state is laid out by the compiler on both backends. Host-only fields go in the Kotlin class body as usual, and on Rust in a `` `struct H` { name: Type = init } `` block, initialised in order after the constructor's parameters are available:

```rust
`struct HostCounter` {
    log: std::sync::Mutex<Vec<i32>> = std::sync::Mutex::new(Vec::new()),
}
```

A mistake in a template is reported at its line in the template, and a template naming something its `.sv` file does not declare is an error. **Which backends are required** is the project manifest's call: every backend `[build] backend` names needs an implementation. A declaration implemented only in Kotlin is allowed, with a warning, so a code base can reach parity before its Rust backend is enabled.

## Kotlin

* When `None` is the only return type of a function, it should be translated to `Unit`.
* The backend should define generic union type wrappers using a sealed interface. If the larger union type is of size N, then the backend should define union types for each number from 1 to N. The qualifier checks then reduce down to checking which of the sealed types a value results in.
* Effects and handlers map to interfaces and implementations of those interfaces. The effects a function declares are its first parameters, typed by the interfaces, and every use of an effect in the body is a call on the relevant parameter. A `use` of a stateful handler wraps it in the effect's `synchronized` monitor (`__Mon_E`), so it is safe to hand to a spawn; a stateless one binds raw. A dependent handler takes its dependencies as trailing constructor parameters.
* `Mut Str` maps to `StringBuilder`, which — unlike `MutableList<T>` — is *not* a subtype of the immutable form, so dropping the `Mut` emits `.toString()`. `copy` of a `Mut Str` is `StringBuilder(sb)`, not the identity.
* `Byte` maps to `UByte`, not to Kotlin's signed `Byte`: an octet has to print and compare the same on both backends, and a signed byte would render 255 as `-1` where Rust's `u8` renders `255`.
* `Bytes` and `Mut Bytes` both map to one **shipped runtime class** (`salvo.SalvoBytes`, emitted per program that names the type): a growable byte array with structural `equals`/`hashCode` and an `iterator()`. Neither stdlib shape would do — `List<UByte>` boxes every element, and `UByteArray` is fixed-size *and* is not a `List<T>`, so generic code could not take one. Rust needs no such class: `Bytes` is a `Vec<u8>`.

## Rust

* When `None` is the only return type of a function, the return type is omitted (`()`).
* `T?` maps to a physical `Option<T>`; union types map to generated enums (`Union2<T1, T2>` with one variant per non-`None` arm).
* Deductions determine ownership: a parameter that appears in a function's deductions is passed by reference (`&T`, or `&mut T` when its declared type carries `Mut`), while a parameter omitted from the deductions is moved (passed by value) — the calling code no longer has access to it in Salvo, so the move is always legal. Copy scalar types are always passed by value.
* Every effect `E` is a **handle** struct of that name, with `&self` members dispatching either to an `Arc<dyn __Stateless_E>` (a stateless handler, no lock) or an `Arc<Mutex<dyn __Stateful_E>>` (a stateful one); handlers implement one of the two traits by their statefulness. Every binding is a handle: `use` makes `let e = E::shared(H::new(…))` or `E::locked(…)`, a fn declaring `[A, B]` takes `a: &A, b: &B`, a dependent handler holds `__dep_E: E` fields, and a spawn or a task clones the handles it inherits.
* `Str` and `Mut Str` are both `String`, so dropping a `Mut` emits nothing. String indexes are *characters*, not bytes, on both backends, so the lowerings convert where Rust counts bytes.
* See BACKEND_SPEC.rust.md for the full rules.
