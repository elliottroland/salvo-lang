# Backends

One of the aims of Salvo is to make it easy to integrate Salvo code with the backend code. To achieve this, the Salvo compiler builds an internal representation (in Rust), and passes this on to the configured backend implementation to write out the relevant target source code. In order to support this, we distinguish between two layers: the `intrinsic` layer, which is the compiler's, and the `platform` layer, which is yours. The core library is entirely intrinsic; everything an application needs from its target language goes through a `platform` declaration.

## Intrinsic

The `intrinsic` layer sits in a backend specific module inside the compiler. This handles complex language-specific logic, and core functionality: how to encode union types, what the `None` type transpiles to in different cases, how to pass parameters to functions, how function naming works, how imports are handled, and more. These can only be changed by making changes to the compiler itself. Anything involving syntax will appear here, and all `intrinsic` backend definitions are declared as part of the standard library (defined in `std`).

`intrinsic` is the standard library's alone. Customer code cannot declare one, because there would be no lowering in any backend to give it meaning — an `intrinsic` with no compiler support behind it is a promise nothing keeps. Application code reaches the target language the other way, through the `platform` declarations — a `platform handler` implementing an ordinary effect; that is the single interop path. This is also the one exception to a plain structural rule: a top-level `fn` must have a body and a `type` must have a definition (`= ...`). The bodyless declaration forms customer code does have are the `platform` ones — `platform handler` and `platform fn` — whose contract the *build* fulfils; `intrinsic` (and the bodyless `intrinsic handler`) is what lets the standard library state a contract the compiler fulfils in place of one.

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

You declare an ordinary effect for what you need, and a **`platform handler`** of it whose implementation is host code. The compiler generates an interface for the host to implement — `interface ClockPlatform` in Kotlin, a trait `ClockPlatform` in Rust (`ClockPlatformSync` for a `threadsafe` handler, whose members take `&self`) — and an adapter between it and the program, which is where values the host returns are checked. Write `salvo platform generate` to get the skeleton:

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

Inside a root the files mirror the source layout: `main.kt` implements the platform declarations of `main.sv`, `app/entry.rs` those of `app/entry.sv`. Both languages can sit side by side in one root — a build only ever picks up the extensions of the backend it is compiling for — so one source tree stays buildable for both targets; or each backend can have a root of its own. A program with platform declarations and no root for a backend it builds is refused, naming the key to add. The examples below assume `platform = "platform"`.

The skeleton is generated **once**: run the command again and it reports that the file exists and leaves it alone, because from that point on it is yours. Forgetting to run it at all is an ordinary compile error that names the command.

The adapter, and the wrapper the program calls for a `platform fn`, **check what the host hands back** wherever the Salvo type promises more than the host type can say: a result declared `"gold" | "silver"` must be one of the two, a `NonEmpty List<Str>` must pass `NonEmpty`'s `qualifies`, and so on inside lists, maps, structs and union arms — including a value the host sends later on a `Reply<T>`. A failure panics (Rust) or throws (Kotlin) where the value crosses, naming the declaration, the way Kotlin meets a Java `null` it was promised would not come. `Ok`, `Err` and `Other` say where a value came from rather than what it holds, so they are taken on trust; a qualifier with no `qualifies` is trusted only from its own module. A check that has to walk a collection is a warning on the declaration, since its cost grows with what the host returns. Sets and maps keep Salvo's order across the boundary: on Kotlin a returned `HashSet` or a `TreeSet` under natural ordering (which sorts by UTF-16 code unit, not by code point) is copied into Salvo's shape — build sorted ones with `salvo.salvoSortedSetOf(…)` to avoid the copy — and on Rust `collect()` builds them. A collection keyed by an identity your program defines (`SortedSet<Str>(by_len)`) cannot be built by host code at all, so a platform declaration may not return one.

Unions keep their positional representation (`Union2.U1(x)` in Kotlin, `Union2::U1(x)` in Rust), and the compiler adds **factories** so host code need not count arms. A named union gets one per arm — `FsErrors.notFound(NotFound(path))` in Kotlin, `FsError::not_found(…)` in Rust — and the union a platform fn or member returns or replies with gets an object named after it: `ReadToStr.err(FsErrors.notFound(NotFound(path)))`. A struct arm is named after the struct, a qualified arm after its qualifier (`ok`, `err`), a base type after the base (`str`); the literals of one base share a single `str(value)` that checks its argument, and two arms that would share a name get no factory.

Every build also writes a **host project** into the root, so the root opens in an IDE as an ordinary Kotlin or Rust project and the implementation files compile there with nothing else present: a `build.gradle.kts` and `settings.gradle.kts` for Kotlin, a `Cargo.toml` and a crate root `lib.sv.rs` for Rust, both naming the host libraries the manifest declares. Beside them are the **declarations** the implementation files use — every struct, type alias and effect the platform declarations reach, emitted as the build emits them, one `<module>.sv.kt` / `.sv.rs` per declaring module, and the runtime they need under `salvo/`. These files start with a `GENERATED` header and are rewritten by `compile`, `run`, `test` and `platform generate` (never by `analyze` or the language server); the build itself never reads them, since it emits its own copies. Check them in, so a fresh checkout opens before its first build. A library others depend on must: each generated file's header carries a stamp (`salvo-abi 1 <hash>`, the compiler's ABI revision and a hash of the library's platform signatures), and a project building the library's platform code compares it first, so a library whose host project is missing, was generated by a compiler with another ABI, or no longer matches its own signatures is reported by name, with the command that fixes it.

(Platform effects — an effect implemented wholly by the host and handed to `main` — were removed on 2026-10-01; a platform handler covers the same ground.)

## A host implementation of an ordinary effect

The effect is Salvo's own — declared here, handled here, possibly by several handlers — and only one of those handlers is host code: the one that actually touches the outside world. That handler is a `platform handler`:

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

The implementation goes in the platform root, as a class named after the *handler* — the `use` site constructs that name, so it is not the host's to choose — and `salvo platform generate` writes the skeleton for it:

```kotlin
// platform/main.kt, as generated
class HostRawClock : RawClockPlatform {
    override fun rawNow(): Int {
        TODO("implement RawClock.raw_now")
    }
}
```

`main` stays the program's entry point, because the instance is constructed *inside* the program. Constructor parameters are how a host implementation is configured — `platform handler HostS3(bucket: Str) of Store`, registered as `use HostS3("my-bucket")`, becomes a class with a `bucket` parameter.

Three restrictions, each following from the implementation not being Salvo's:

* **No body in Salvo** — no members, no state. The host class holds both.
* **No effect dependencies.** A handler's dependencies are supplied to its *members*, and these members are host code, which performs no Salvo effect: the host reaches the outside world directly. Write an ordinary Salvo handler that depends on this one's effect when something has to sit in between — `handler DefaultClock [RawClock] of Clock` is exactly that.
* **Not generic**: the host writes one concrete class.

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
effect Slow {
    fn later(n: Int, done: Reply<Int>) [] -> None => !n, !done
}

threadsafe platform handler HostSlow of Slow
```
```kotlin
override fun later(n: Int, done: salvo.SalvoReply) {
    val host = done.hosted()
    Thread { Thread.sleep(50); host.send(n + 1) }.start()
}
```

On Rust a host reply dropped without being sent is reported to the pool's fault sink; the JVM cannot observe that, so on Kotlin a lost reply leaves its waiter waiting.

A host file may use any library of its language, provided the manifest declares it — `[rust] crates`, `[kotlin] libs` and `artifacts`; see [Modules](Modules.md) "Host libraries". Rust then builds with `cargo` instead of bare `rustc`, and Kotlin puts the declared jars on the classpath.

A platform handler fits wherever several implementations of a capability exist and one of them is host code: a real filesystem beside an in-memory one, a host clock beside a fake, an S3-backed store beside a local directory. The standard library uses it itself, and ships its host classes the same way — under `std`'s own platform root, one file per backend.

# Specific backend details

## Platform functions

A single function can be implemented by the host too: a **`platform fn`**, a signature with no body.

```
// counter.sv
platform fn shout(g: Greeting) [] -> Str => g
```

The implementation is a function of the same name in the module's implementation file under the platform root, which `salvo platform generate` writes alongside the module's handlers:

```kotlin
// platform/counter.kt
fun shout(g: Greeting): String = if (g.loud) g.text.uppercase() else g.text
```
```rust
// platform/counter.rs
pub fn shout(g: &Greeting) -> String {
    if g.loud { g.text.to_uppercase() } else { g.text.clone() }
}
```

The program calls a generated wrapper (`shoutPlatform` / `shout_platform`), which calls the implementation; the wrapper is where the boundary checks the planned ABI adds will go. A platform fn is not generic, takes no implicit parameters and declares no effects, and two platform fns may not overload each other.

## Kotlin

* Generated Kotlin uses Kotlin's naming: every function, member, parameter, field and local is written in camel case (`read_to_str` becomes `readToStr`, a field `data_type` becomes `dataType`). Rust keeps Salvo's snake case. So that a program means the same on both, two names that differ only in this way — `foo_bar` and `fooBar` in one scope — are an error on every backend.
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
