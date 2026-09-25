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

The host code lives in a `platform/` directory at the root of your sources, mirroring the source layout: `platform/main.kt` implements the platform effects declared in `main.sv`, `platform/app/entry.rs` those of `app/entry.sv`. Both languages can sit side by side in the same tree — a build only ever picks up the extension of the backend it is compiling for — so one source tree stays buildable for both targets.

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

The implementation goes in the same `platform/` tree as a platform effect's, as a class named after the *handler* — the `use` site constructs that name, so it is not the host's to choose — and `salvo platform generate` writes the skeleton for it too:

```kotlin
// platform/main.kt, as generated
class HostRawClock : RawClock {
    override fun raw_now(): Int {
        TODO("implement RawClock.raw_now")
    }
}
```

Nothing else moves: `main` stays the program's entry point, because the instance is constructed *inside* the program rather than handed to it. Constructor parameters are how a host implementation is configured — `platform handler HostS3(bucket: Str) of Store`, registered as `use HostS3("my-bucket")`, becomes a class with a `bucket` parameter.

Three restrictions, each following from the implementation not being Salvo's:

* **No body in Salvo** — no members, no state. The host class holds both.
* **No effect dependencies.** A handler's dependencies are supplied to its *members*, and these members are host code, which performs no Salvo effect: the host reaches the outside world directly. Write an ordinary Salvo handler that depends on this one's effect when something has to sit in between — `handler DefaultClock [RawClock] of Clock` is exactly that.
* **Not generic**, for the reason a platform effect is not: the host writes one concrete class.

The two forms answer different questions. Use a `platform effect` when the *capability* is the host's and the program is a guest in the host's process — the host constructs everything and owns `main`. Use a `platform handler` when the capability is the language's, several implementations exist, and one of them is host code: a real filesystem beside an in-memory one, a host clock beside a fake, an S3-backed store beside a local directory. The standard library uses the second form itself, and ships its host classes the same way — under `std`'s own `platform/` tree, one file per backend.

# Specific backend details

## Kotlin

* When `None` is the only return type of a function, it should be translated to `Unit`.
* The backend should define generic union type wrappers using a sealed interface. If the larger union type is of size N, then the backend should define union types for each number from 1 to N. The qualifier checks then reduce down to checking which of the sealed types a value results in.
* Effects and handlers can map to interfaces and implementations of those interfaces. The effects are passed to a function as the first arguments of that function, and all uses of those effects is mapped to the relevant parameter name.
* `Mut Str` maps to `StringBuilder`, which — unlike `MutableList<T>` — is *not* a subtype of the immutable form, so dropping the `Mut` emits `.toString()`. `copy` of a `Mut Str` is `StringBuilder(sb)`, not the identity.
* `Byte` maps to `UByte`, not to Kotlin's signed `Byte`: an octet has to print and compare the same on both backends, and a signed byte would render 255 as `-1` where Rust's `u8` renders `255`.
* `Bytes` and `Mut Bytes` both map to one **shipped runtime class** (`salvo.SalvoBytes`, emitted per program that names the type): a growable byte array with structural `equals`/`hashCode` and an `iterator()`. Neither stdlib shape would do — `List<UByte>` boxes every element, and `UByteArray` is fixed-size *and* is not a `List<T>`, so generic code could not take one. Rust needs no such class: `Bytes` is a `Vec<u8>`.

## Rust

* When `None` is the only return type of a function, the return type is omitted (`()`).
* `T?` maps to a physical `Option<T>`; union types map to generated enums (`Union2<T1, T2>` with one variant per non-`None` arm).
* Deductions determine ownership: a parameter that appears in a function's deductions is passed by reference (`&T`, or `&mut T` when its declared type carries `Mut`), while a parameter omitted from the deductions is moved (passed by value) — the calling code no longer has access to it in Salvo, so the move is always legal. Copy scalar types are always passed by value.
* Effects map to traits with `&mut self` methods; effect dependencies become leading `&mut dyn` parameters, and `use` instantiates a handler into a local that is threaded as `&mut local`.
* `Str` and `Mut Str` are both `String`, so dropping a `Mut` emits nothing. String indexes are *characters*, not bytes, on both backends, so the lowerings convert where Rust counts bytes.
* See BACKEND_SPEC.rust.md for the full rules.
