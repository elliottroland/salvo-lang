# Effects and handlers

## Effects

Some modern languages have been playing with algebraic _effects_. At this early stage, we are just using them as a way of passing concrete implementations of "interfaces" around. An effect defines a set of functions which will be available to functions which declare a dependency on it:

```
// An effect which can generate range T's
effect Random<T> {
    fn next_random() -> T
}
```

Effects have _handlers_, which define their concrete implementations:

```
// A handler which returns successive values in an array, and loops back when done.
// Handlers have their own state, which can be used through their lifetime. Here `numbers` are passed in and `i` is defined at construction time with a default value.
handler CyclicRandom<T>(values: T[]) of Random<T> {
    i: Int = 0

    // Every function must be defined. In this case just `next_random`
    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}
```

A handler can be "registered" in the current context using the `use` keyword. This operates similarly to `with` in Koka and `use` in Gleam:

```
fn main() [use] -> None {
    // Register the CyclicRandom as the implementation of Random<Int> for the rest of this function.
    // A later `use` for the same effect shadows this one from that point on.
    use CyclicRandom(array_of(1,2,3,4))

    // Here we can call the function next_random()
    let num = next_random()
}
```

The effects a function depends on are declared in the square brackets before its arrow. In the above example, the `main` function (which is also the entry point to any Salvo program) starts with the special `use` effect, which is what allows it to use the `use` keyword. If a function does not declare a dependency on this, then `use` is not available to it. If the initial square brackets are not present in a function declaration, then it is assumed to be empty and that function is "pure".

An effect names a *capability*, not a type of values. It can appear in a
function's effect list and in a handler's `of` clause, and nowhere else: a
struct field, parameter, return type or `let` annotation of effect type is
a compile-time error. Handlers are not values either — `use` is the only
thing that produces one, and you reach it by calling the effect's members
rather than by holding the handler:

```
let c: Console = StdOutConsole()   // error: `Console` is an effect, not a data type
let h = StdOutConsole()            // error: `StdOutConsole` is a handler, not a value
use StdOutConsole()                // this is how you register it
println("...")                     // and this is how you use it
```

A generic handler can be registered at a type by writing it on the `use`:

```
handler Plain<T> of Show<T> { ... }

use Plain<Int>()        // registers `Show<Int>`
```

For a handler with a constructor argument the type is usually implied by it
(`use Prefixed("p")`), and writing it as well is fine as long as the two
agree — a disagreement is an error rather than one quietly winning. A handler
with *no* argument has nothing to imply it, so there the written form is the
only way.

Almost every action other than simple data transformation needs to be encoded in an effect. For example, printing to the console is managed by an effect:

```
effect Console {
    fn println(message: Str) -> None => message
}
```

Suppose we want to write a function which uses the Random and Console effects. Then we can write the following:

```
fn age_prediction(person: Surname Person) [Random<Int>, Console] -> None => person {
    // next_random() is accessible because of Random<Int> effect
    let years: Int = next_random()

    // println() is accessible because of Console effect
    println("In ${years} years, ${full_name(person)} will be ${person.age + years}")
}
```

When calling a function, all its effects either need to be `use`d or be declared in the calling function's effect dependencies. Two effects of the same type can be in the dependencies, so long as they have different generic values:

```
fn random_numbers() [Random<Int>, Random<Double>] -> None {
    // Ambiguous: will result in compile-time error
    let number = next_random()

    // Will use Random<Int> handler
    let int: Int = next_random()

    // Will use Random<Double> handler
    let double: Double = next_random()

    // The type can also be specified in the function call
    let number = next_random<Int>()
}
```

## Handlers with dependencies, and interception

A handler may need an effect of its own to do its job. It declares that the way a function does — an effect list before the `of` clause — and the compiler supplies it where the handler is registered. Callers never mention it:

```
handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")      // Console reaches the body
    }
}

fn main() [use] -> None {
    use StdOutConsole
    use ConsoleLogger                   // the Console is not written here
    log("hello")
}
```

The dependencies have no names, because there would be nothing to do with one: code inside a handler reaches an effect the way all Salvo code does, by calling its members. So the list says only *what the handler needs to run*, which is exactly what a function's effect list says.

The dependency is declared on the *handler*, not on the effect: two implementations of one effect need different things, and a `Logger` that writes to a file has no business making every `Logger` mention a console. A `use` whose dependency has no handler in scope is an error that names it, so a dependency is always registered before its dependent — which is also why dependency cycles need no separate check: `A` needing `B` and `B` needing `A` fails at whichever is registered first.

The list may name **the very effect the handler implements**, which is *interception*: a handler that wraps the one already in scope.

```
handler Loud [Greeter] of Greeter {
    fn greet(name: Str) -> Str => name {
        return "${greet(name)}!"        // the *wrapped* greeter
    }
}

fn main() [use] -> None {
    use StdOutConsole
    use Plain                           // greet -> "hello world"
    use Loud                            // greet -> "hello world!"
    println(greet("world"))
}
```

The rule that makes this well-defined is that such a dependency **binds strictly outward**: it is the instance registered *before* this `use`, never the handler being registered. So a call inside `Loud.greet` goes one layer out rather than back to itself, interceptors stack (`use Loud` twice gives `"hello world!!"`), and the acyclicity argument is untouched — every dependency still points at an earlier registration. Registering an interceptor with nothing to intercept is an error that says so.

Interception is what makes a *policy* handler writable in Salvo: a restricting file system that checks paths and then delegates, a logging or retrying handler over whatever was there before, a test double wrapped around a real implementation. It is per instance of a generic effect, so intercepting `Store<Int>` leaves `Store<Str>` alone.

Both of these rest on shadowing, which is worth stating on its own: a `use` for an effect instance already in scope is not an error — it takes over for the rest of the block, and the handler it shadowed answers again when that block ends. Shadowing needs no dependency (registering a second, unrelated `Greeter` simply replaces the first), and the innermost registration always wins, exactly as a shadowing local variable does.

One thing a handler cannot do yet: call **its own** effect's other members. `Twice.greet` cannot call `greet` on itself — a handler does not dispatch to itself, and declaring the effect as a dependency means the handler *outside*, not this one. Move the shared logic into a function both members call; the diagnostic says as much.

## One handler, several effects

A handler may implement more than one effect, and the effects are simply listed: `handler H() of Public, Admin`. It stays one handler with one piece of state — what it gains is a *face* per effect, which is how a public protocol and an administrative one share an implementation without a second handler forwarding into the first.

```
effect Tally { fn bump(n: Int) -> None => !n }
effect Stats { fn total() -> Int }

handler Counting() of Tally, Stats {
    sum: Int = 0

    fn bump(n: Int) { sum = sum + n }
    fn total() -> Int { return sum }
}

fn main() [use] {
    use local Counting()    // registers *both* effects — `local` because one
    bump(2)                 // lock behind several faces has no shared form yet
    println("${total()}")
}
```

One `use` binds every face, so a function declaring `[Tally]` and one declaring `[Stats]` both reach this instance. For an actor the same declaration is what makes least authority ordinary: `spawn` answers **one addr per face**, in declaration order, so who holds which face decides what they may do.

```
let (timer, ctl) = spawn ManualTime() on pool(1)
//   ^Addr<Timer>  ^Addr<TimerCtl> — code holding `timer` cannot name `advance`
```

A single face answers a bare `Addr<E>` as it always did; several answer a tuple. The rules the list brings with it are the ones you would guess: every member of every effect must be implemented, each effect may be named once, and the effects must be of one kind — all `actor effect`s or none, because a handler is bound one way or the other. Where two of the effects declare the same member *name*, one handler method may implement both when their signatures are identical, or two methods may implement them when the parameters differ (ordinary overloading); what is refused is the case overloading cannot see — same parameters, a different return type — since no single method could answer for both. Callers still pick the face with `@` where two effects in scope declare the name, exactly as below.

## Monitors — sharing a plain-effect handler

A handler of a *plain* effect can also be spawned. That does not make it an actor — there is no mailbox and no messages — it makes it a **monitor**: one shared instance behind a lock, whose members run on the callers' threads under mutual exclusion. The spawn answers the same `Addr<E>` an actor spawn answers, the handle is freely copyable and sendable, and `use` binds it like any handle — so a caller of `next()` never learns whether the `Random` in scope is a scope-local handler or a monitor three threads share.

```
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    cursor: Int = 0
    fn next() -> Int {
        cursor = (cursor * 31 + seed) % 100000
        return cursor
    }
}

fn main() [use, spawn] {
    let rng = spawn CyclicRandom(12345)   // one shared instance, no `on` —
    use rng                               // a monitor runs on its callers' threads
    let n = next()                        // lock, advance, unlock
    let child = spawn Worker() with rng    // the same instance, from another thread
}
```

The price of sharing used to be "a monitor declares no dependencies"; since the shareable-by-default round (2026-09-20) a monitor **may** declare dependencies, provided every one is the shareable default (`[E]` — no `local E`, no `use`, no `spawn` capability). They are captured as **owned handles at construction**, resolved from the enclosing scope exactly as a `use` resolves them, so the bindings are fixed for the instance's life. The availability rule keeps the capture acyclic — a dependency was bound before this handler, so no lock order can cycle by construction — and where a member *can* wait (a dependency with mixed handlers, a `waitfor` in a member), the wait is **priced, not refused**: the handler gets a node in the same deadlock graph actors use (`H's lock`), and a cycle through held locks is reported before it runs. A handler that cannot be shared at all — unsendable state, the `use`/`spawn` capabilities, a `local` dependency — is refused with the opt-out named.

A monitor serializes with a lock where an actor serializes with a mailbox, and one piece of state can be under only one of them — so a handler is one or the other, read off its shape: mutable state with `send fn` members is an actor's, mutable state with only plain members shares as a monitor. Fit: passive shared state — counters, caches, configuration, cursors, `Random`. A generic effect instance shares like any other: `handler CyclicRandom of Random<Int>` gets a monitor of `Random<Int>`, so genericity costs nothing here.

## Shareable by default: `use`, `use local`, and `local E`

```
use Counting()          // shareable: the instance may be captured by a spawn
use local Counting()    // this frame only — nothing may capture it

fn tally(n: Int) [local Tally] -> None {
    …                   // declares that its handler need not be shareable
}
```

`use H(args)` binds **shareable by default** (user decision 2026-09-20). A stateless handler binds *bare* — shareable without a lock, so `StdOutConsole` and friends pay nothing — and a stateful one binds as a **monitor**: lock-shaped from birth, effectively `let h = spawn H(args); use h`. A `use` of an addr or of a spawn expression is already a handle and needs no words. The motivating goal is *spawn-inheritance*: for `spawn H on pool(2)` to pick up the scope's effects without re-declaration, a bare `[E]` in a signature has to guarantee something that may cross a seam.

That is what `[E]` now means: **a shareable `E`** — the function may pass it across seams. The opt-outs are spelled:

- **`use local H(args)`** binds scope-local and lock-free — the pre-2026-09-20 meaning. It is *required*, by an error naming it, for handlers that cannot be shared: unsendable state (a stored lambda), the `use`/`spawn` capabilities, a `local E` dependency, an actor-effect or generic-instance dependency (both pin the fusion form).
- **`[local E]`** in an effect list accepts a scope-local binding and disclaims seam rights for `E`. The call-site rule: a `use local` binding satisfies only `[local E]` requirements; a shareable binding satisfies both, since `local` is the weaker claim. The annotation is viral down call chains that traffic in local bindings — an accepted cost, to be lifted later by inference — and std's own effect-forwarding functions (`println`, the `Fs` surface, `elapsed`) declare `[local E]`, being pure forwarders that never cross a seam.
- A **fn type's** effects are always call-only — a function value cannot spawn — so writing `local` there is refused as redundant, and a lambda's availabilities are local: a function called from inside a lambda declares `[local E]`.

A dependent handler bound shareable captures its dependencies as owned handles **at construction** — from a binding in the same function, or from an effect the function received through its own signature, in which case the handle is threaded in by the caller (see the next section). A `platform handler` is **assumed thread-safe by its design** (user decision 2026-09-20): it classifies bare, so nothing that depends on a platform-backed effect ever writes `local` — `DefaultFs [RawFs]` stays annotation-free, as does the production interceptor chain, which was the point of lifting monitor dependencies. The assumption is unvalidated for now; a way for a host to state (and the compiler to check) its thread-safety is future work.

## Inheriting the scope, and overriding it with `with`

A spawned handler **inherits its dependencies from the spawning scope** (user decision 2026-09-20). A handler declares what it needs, the spawn says where it runs, and the wiring in between is the compiler's:

```
handler Drawing() [Random] of Drawer { … }

fn main() [use, spawn] {
    use CyclicRandom(7)                   // one shared instance
    let d = spawn Drawing() on pool(1)    // …inherited, with nothing written
}
```

This is what the shareable-by-default round bought. A bare `[Random]` guarantees a shareable instance, so the scope's registration can be captured as a handle and travel with the child — checked one function at a time, with no whole-program analysis and no runtime check. Before it, every dependency had to be written at every spawn.

Only a shareable binding can be inherited: a `use local` one exists precisely so that it does not cross a seam, and the error names the remedy (bind it shareable, or give the child its own). Two candidate instances in scope is an ambiguity the compiler will not guess at, and nothing in scope is still an error — now naming both ways to fix it.

Where the scope's instance is *not* what a handler should get, **`with` names what it should**:

```
use Plain
use Loud with Formal()        // Loud wraps this fresh Formal, not the Plain in scope
let d = spawn Drawing() with FixedRandom() on pool(1)
```

A `with` item is a **private instance**: constructed at the clause, owned by the handler or child it is given to, shared with nothing. The clause is *partial* — items satisfy the dependencies they match, and the rest still inherit — and it may supply the self-dependency, which is the one case where an interceptor wraps something other than what it shadows. It works on both binding forms, and `use local H with …` is fine too: the clause chooses which instance, which has nothing to do with locality.

The same lift applies to `use` inside a function that *received* the effects it wires:

```
fn interception() [Logger, Clock, use] -> None {
    use Stamped                 // captures handles for Logger and Clock —
    work("stamped")             // both arrived through this signature
}
```

Nothing in that function says `local`. On the Kotlin backend an object reference already is a handle, so this costs nothing; on the Rust backend the caller passes one extra hidden parameter — a small bundle of the handles the callee has to capture — and only on call chains that actually capture one. A function may only have such a parameter if its effect list carries `use` or `spawn`, so the possibility is visible in the signature even though the parameter is not.

One shape still refuses: a **platform effect** cannot be captured this way, because the host owns that instance and hands it to `main` as a borrow — there is no handle to make. Put an ordinary Salvo handler over it (`DefaultFs [RawFs]` is exactly this) or declare the dependency `local`.


## Mixed handlers — a servant behind a plain effect

The other way to share mutable state keeps a mailbox: a handler of a plain effect may declare `send fn` members beside its plain ones, and the two halves divide the work. The send members and the state form the **servant** — an ordinary actor over a handler-local protocol, with a `mailbox` like any other — and the plain members form the **façade**, running on the caller's thread: a façade member sends to its own servant and waits for the answer.

```
handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    cursor: Int = 0                          // the servant's, and only the servant's

    send fn advance(out: Reply<Int>) => !out {
        cursor = (cursor * 31 + seed) % 100000
        send(out, cursor)
    }

    fn next() -> Int {                       // the façade: the caller's thread
        return waitfor got {                 // type read off `advance`'s parameter
            advance(got)                     // a send to its own servant
        }
    }
}

fn main() [use, spawn] {
    let rng = spawn CyclicRandom(12345)      // one servant; the handle is the façade
    use rng
    let n = next()                           // send, wait, answer — no [waitfor] anywhere
}
```

A `waitfor` names its token and infers the token's type from the send the block
makes: `advance` declares `out: Reply<Int>`, so `got` is a `Reply<Int>` and the
wait yields an `Int`. Write the type out — `waitfor got: Reply<Int> { … }` —
where several members of that name would make it ambiguous, which is an error
naming that remedy.

State is **confined**: only send members touch it, so every access is a serialized activation, and a sync member that reads a field is refused by name — its environment is its own parameters, the constructor parameters, and sends to its own servant. Nothing here declares `waitfor`: a call occupying its thread until it returns is what a call is, and the façade's wait serves its pool while it waits. What a caller of `next()` can never learn is whether the `Random` in scope is a scope-local handler, a monitor, or three threads' shared servant — which is the point.

Send members reach their siblings the same way: a bare call naming another of the handler's `send fn` members enqueues on the servant's own mailbox — "finish this activation, then that one" — so a request can travel member to member without leaving the actor. `k@self(…)` is the explicit spelling of the same send, available in either member kind, and it is how the call says what it means where a bare name would be ambiguous with an effect member in scope.

A mixed handler is spawn-only (`use` would leave its sends nowhere to arrive). By default its servant answers every request within the activation that received it, which is what makes a façade's wait end after one straight-line activation rather than after an event that may never come. A send member that declares `defer` on a reply parameter opts out of that default: the answer may outlive the activation — forwarded to another actor whose discharge ends the caller's wait, or parked in a continuation on one of the servant's own members with `replyto`, exactly as an actor parks — and the deadlock graph prices the deferral.

## Two effects, one member name

Different effects may declare the same member name — `close` on a file system and `close` on a network effect is the natural spelling, not a collision. A bare call resolves through whichever effect actually has a handler in scope; when more than one does, the call picks its effect with `@`, the same selector that picks a module's overload:

```
fn shut(h: Int) [Fs] -> Str {
    return close(h)          // only Fs is available here: unambiguous
}

fn both(h: Int) [Fs, Net] -> None {
    close@Fs(h)              // explicit: the Fs member
    h.close@Net()            // dot form, like any member call
}
```

The name after `@` is capitalized, which is what distinguishes an effect selector from a module path (`size@core.list(xs)`). A generic effect's instance is pinned by the call's type arguments, exactly as without the selector: `next_random@Random<Int>()`. A selected member is a call form, not a value.

Within a *single* effect a member name may recur too, as an ordinary **overload**: one name, different parameter types, picked by the argument types like any function overload.

```
effect Fs {
    fn close(s: InStream) -> Ok None | Err Checked<FsError> => !s
    fn close(s: OutStream) -> Ok None | Err Checked<FsError> => !s
}
```

Two members with the same name *and* the same parameter types are the error they look like — no call could tell them apart. The selector and the overload compose: `@Fs` picks the effect, the arguments pick the member.

A member and an ordinary **function** may also share a name, and they are one overload set too. std's own filesystem needs it: `close` is an `Fs` member per stream token *and* the function that closes a `Lines` pass.

```
fn close(p: Lines) [local Fs] -> Ok None | Err Checked<FsError> => !p {   // a function
    return close(p.s)                                            // ...calling the member
}
```

The argument types decide, ranked exactly as two function overloads are: the more specific signature wins, so a concrete function beats a generic member. Two rules keep it predictable:

* **Availability first.** A member is a candidate only where its effect has a handler in scope. Without one, the name is the function — no `@` needed. That is what lets you write a `close` of your own in a program that never opens a file.
* **A tie is an error.** If both sides fit and neither is more specific, the call must say which it means: `close@Fs(…)` for the member, `close@my.module(…)` for the function. Nothing is preferred silently.
