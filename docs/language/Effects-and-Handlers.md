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

## Starting up: `init` and `self@Face`

A handler's state fields have initialisers, but an initialiser is an expression that sees only the constructor parameters. Anything a handler must *do* as it starts — register itself somewhere, compute state from a dependency — goes in its **`init` block**, which runs once, first:

```
handler Working(registry: Addr<Registry>, who: Str) of Worker {
    mailbox { capacity: 8 }
    started: Int = 0
    init {
        started = 1
        registry.register(self@Worker)      // this actor's own address, as its Worker face
    }
    send fn work(out: Reply<Str>) => !out { out.send("${who} (started ${started})") }
}
```

For a spawned handler, `init` is the actor's **first activation**: the spawn enqueues it before it hands the address back, so nothing anyone sends can overtake it, and it runs on the actor's pool like every member. For a `use`-bound handler it runs inline, right after construction — a constructor body. It sees constructor parameters, state and the handler's dependencies, declares no effects, and is checked as a member; it is in fact a private send member named `init` that only the runtime sends.

`self@Face` is the handler's own address as one of its actor faces, an `Addr<Face>`, legal in `init` and in send members. It is how an actor hands itself to something else — a registry, a group, the runtime's peer events — without the spawner having to pass the address back in a first message. A handler that names its own address is spawn-only, because a `use`-bound instance has none; the `use` says so, as it does for a handler that parks with `replyto`.

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
    use Counting()          // registers *both* effects
    bump(2)
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

The price of sharing used to be "a monitor declares no dependencies"; since 2026-09-20 a monitor **may** declare dependencies. They are captured as **handles at construction**, resolved from the enclosing scope exactly as a `use` resolves them, so the bindings are fixed for the instance's life. The availability rule keeps the capture acyclic — a dependency was bound before this handler, so no lock order can cycle by construction — and where a member *can* wait (a dependency with mixed handlers, a `waitfor` in a member), the wait is **priced, not refused**: the handler gets a node in the same deadlock graph actors use (`H's lock`), and a cycle through held locks is reported before it runs.

A monitor serializes with a lock where an actor serializes with a mailbox, and one piece of state can be under only one of them — so a handler is one or the other, read off its shape: mutable state with `send fn` members is an actor's, mutable state with only plain members shares as a monitor. Fit: passive shared state — counters, caches, configuration, cursors, `Random`. A generic effect instance shares like any other: `handler CyclicRandom of Random<Int>` gets a monitor of `Random<Int>`, so genericity costs nothing here.

## Every binding is a handle

```
use Counting()          // a handle: callees receive it, spawns and handlers capture it

fn tally(n: Int) [Tally] -> None {
    …                   // receives the handle of whatever `Tally` is bound
}
```

A `use` makes one value — the **handle** of the instance — and everything downstream is that value or a copy of it: a function's `[E]` parameter, a dependent handler's captured dependency, a spawn's or task's inherited effect, a fn value's effect (user decision 2026-09-28: *one shape*, after a round that had distinguished shareable from scope-local bindings). A `use` of an addr or of a spawn expression is already a handle.

The one thing a handle asks of a handler is whether it is **stateful**. A stateless handler — no `state` fields, no `replyto` mint, no stored function value — runs its members on the shared instance with nothing held: `StdOutConsole`, a pure forwarder, the send stub behind a bound `Addr<E>`, a mixed handler's façade. A stateful one runs them under a lock through its handle — a **monitor**, `let h = spawn H(args); use h` in effect — so two spawns capturing the same stateful binding share its state, exactly as they would through an addr. The distinction is read off the declaration and is the same on both backends; a `platform handler` states it with `threadsafe` ([Backends](Backends.md)), since the compiler cannot see a host's fields.

The motivating goal is *spawn-inheritance*: for `spawn H on pool(2)` to pick up the scope's effects without re-declaration, a bare `[E]` in a signature has to be something that may cross a seam — and a handle is. Since every binding is one, nothing has to be annotated to be inherited, captured, or handed to a scheduled body, and a function called from inside a lambda declares `[E]` like any other.

A dependent handler captures its dependencies as handles **at construction** — from a binding in the same function, or from an effect the function received through its own signature (see the next section). `DefaultFs [RawFs]` is an ordinary dependent handler over a platform handler.

In the generated code the handle carries the effect's name: `fn work(console: &Console)` on Rust, `fun work(console: Console)` on Kotlin. The mangled names (`__Stateless_E`, `__Stateful_E`, `__Mon_E`) are the traits and wrappers behind it.

## Many instances: `[any E]` and `of any E`

A bare `[E]` claims **identity**: that `E` is *one* instance, so two sends to it arrive in order and see the same state. A function written `bump(2); bump(3); total(out)` relies on that, and nothing in its body says so. A binding spread over many instances — a router in front of a fleet — cannot give it, so the weakening is spelled where the requirement is:

```
handler RoundRobin(members: List<Addr<Resizer>>) of any Resizer {   // forwards; promises no order
    mailbox { capacity: 16 }
    next: Int = 0
    send fn resize(name: Str, w: Int, out: Reply<Str>) => !name, !out {
        let target = copy(get(members, next % size(members))!)
        next = next + 1
        target.resize(name, w, out)
    }
}

fn thumbnail(name: Str) [any Resizer] -> Str {                       // any Resizer will do
    return waitfor out: Reply<Str> { resize(name, 100, out) }         // one send, one reply
}

fn tally(names: List<Str>) [Resizer] -> Str { … }                    // needs ONE Resizer

use RoundRobin([a, b])
thumbnail("cat.png")     // fine
tally(["dog.png"])       // error: `tally` declares `[Resizer]` — one instance, sends in order —
                         //        but `Resizer` is bound to many instances here
```

- **`of any E`** on a handler's face says the handler forwards to many instances. A `use` of it binds `any E`, which satisfies only `[any E]`. It is also the one handler of an actor effect that `use` binds *shareable* — bare when stateless, a monitor when it keeps a cursor — since a group handle is always shareable.
- **`[any E]`** in an effect list says "each send may go to a different instance; I assume no order and no shared state between them." It accepts every binding, a single instance being a group of one, and it is viral downward: a body holding `[any E]` may call `E` directly and callees declaring `[any E]`, never a callee that assumes one instance. Bare `[E]` keeps the strong meaning, so a program written before groups existed cannot be broken by binding one under it.
- A fn type carries no strength (no `any`): the function that takes the value writes `[any E]` itself if any instance will do, which weakens the requirement it inherits.

`any` is a claim about the guarantee the effect gives, not about where the instances are: a local group of a hundred on `pool(4)` is `any`; a single actor on another machine, reached through its `Addr`, is not.

## Inheriting the scope, and overriding it with `with`

A spawned handler **inherits its dependencies from the spawning scope** (user decision 2026-09-20). A handler declares what it needs, the spawn says where it runs, and the wiring in between is the compiler's:

```
handler Drawing() [Random] of Drawer { … }

fn main() [use, spawn] {
    use CyclicRandom(7)                   // one shared instance
    let d = spawn Drawing() on pool(1)    // …inherited, with nothing written
}
```

A bare `[Random]` is a handle, so the scope's registration can be captured and travel with the child — checked one function at a time, with no whole-program analysis and no runtime check. Before handles, every dependency had to be written at every spawn.

Two candidate instances in scope is an ambiguity the compiler will not guess at, and nothing in scope is an error — naming both ways to fix it.

Where the scope's instance is *not* what a handler should get, **`with` names what it should**:

```
use Plain
use Loud with Formal()        // Loud wraps this fresh Formal, not the Plain in scope
let d = spawn Drawing() with FixedRandom() on pool(1)
```

A `with` item is a **private instance**: constructed at the clause, owned by the handler or child it is given to, shared with nothing. The clause is *partial* — items satisfy the dependencies they match, and the rest still inherit — and it may supply the self-dependency, which is the one case where an interceptor wraps something other than what it shadows. The clause chooses *which* instance; how it is bound is not its business.

The same lift applies to `use` inside a function that *received* the effects it wires:

```
fn interception() [Logger, Clock, use] -> None {
    use Stamped                 // captures handles for Logger and Clock —
    work("stamped")             // both arrived through this signature
}
```

The effects a function receives *are* handles — on the Kotlin backend an object reference, on the Rust backend a cloneable wrapper — so capturing one costs a copy and nothing else.

One shape still refuses: a **platform effect** cannot be captured this way, because the host owns that instance and hands it to `main` as a borrow — there is no handle to make. Put an ordinary Salvo handler over it (`DefaultFs [RawFs]` is exactly this).


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

A member and an ordinary **function** may also share a name, and they are one overload set too. std's own filesystem needs it: `close` is an `Fs` member per stream token *and* the function that closes a `Lines` iterator.

```
fn close(p: Lines) [Fs] -> Ok None | Err Checked<FsError> => !p {         // a function
    return close(p.s)                                            // ...calling the member
}
```

The argument types decide, ranked exactly as two function overloads are: the more specific signature wins, so a concrete function beats a generic member. Two rules keep it predictable:

* **Availability first.** A member is a candidate only where its effect has a handler in scope. Without one, the name is the function — no `@` needed. That is what lets you write a `close` of your own in a program that never opens a file.
* **A tie is an error.** If both sides fit and neither is more specific, the call must say which it means: `close@Fs(…)` for the member, `close@my.module(…)` for the function. Nothing is preferred silently.
