# Linear types

Salvo's ownership rules make values *affine*: they can be used at most once (see [Deductions and ownership](Deductions-and-Ownership.md)). Resource types want the other half too — a file handle that is never closed, a transaction that is never committed or rolled back, is a bug. A type declares **linearity** with a modifier, and its file supplies the death:

```
linear struct FileHandle {
    fd: Int
}

fn close(handle: FileHandle) -> None => !handle {
    // release the resource
    discard(handle)
}
```

The **discharge set** is every function — and every *effect member* — declared in the *type's own file* that consumes a parameter of the type — `close` for a file, `stop` *or* `join` for a thread handle, `remove(cache, entry)` for a pooled one (the extra parameters are ordinary parameters). A `linear struct` whose file has no such function is an error at the struct: the obligation would have no legal death. Leak diagnostics name the whole set. `discard(handle)` is the obligation's terminal, legal **only** inside a discharger — and a discharger gets no exemption: its own body must terminate the obligation on every path, by `discard` or by forwarding into another discharger (`fn shutdown(t: Thread) => !t { stop(t) }`).

When the discharger is an effect member, the declaration is what carries that status, so **every handler's implementation of it** is a discharge context — the real one, a test double in another module, an interceptor that discharges by forwarding into the handler it wraps. That is how a stream token stays linear while `close` is a member of `Fs`: one declaration, many implementations, all of them allowed to end the obligation and none of them able to end anybody else's (a member consuming an `InStream` may not `discard` an `OutStream`).

Every value of a linear type carries an **obligation**: on every path, it must be *moved onward* before it goes out of scope. Moving is anything the ownership system already recognizes — passing it to a consuming call (`close(handle)`), returning it, spreading it, a move-mode binding handing it to a new owner. Each move transfers the obligation with the value: a function that receives a linear value by move must discharge it in turn; a function that *keeps* (borrows) a linear parameter leaves the obligation with its caller; a derived (fate-linked) variable is an alias and carries no obligation of its own.

Dropping the obligation is a compile-time error, wherever it would happen:

```
fn leak() {
    let h = open("data.txt")
}                               // ERROR: `h` still owns a linear value at scope exit

fn maybe_leak(flag: Bool) {
    let h = open("data.txt")
    if flag {
        close(h)
    }
}                               // ERROR: `h` is consumed on some paths only

fn drop_result() {
    open("data.txt")            // ERROR: a linear value is dropped immediately
}

fn overwrite() {
    let h = open("a.txt")
    h = open("b.txt")           // ERROR: overwriting drops the first handle
    close(h)
}
```

There is no escape hatch: `discard(x)`, which deliberately drops an ordinary value, refuses a linear one and names its `close` — dropping a handle is exactly the leak the obligation exists to prevent.

```
fn deliberate() {
    let h = open("data.txt")
    discard(h)                  // ERROR: `discard` cannot drop a linear value; call `close(h)`
}
```

The `maybe_leak` shape above — a value that must be released however the block ends — is written out: `close(h)` on each path. The compiler names the path you missed, which is the whole point; there is no construct that discharges an obligation implicitly (`defer` did, and was removed 2026-09-10 for exactly that reason).

```
fn no_leak(flag: Bool) {
    let h = open("data.txt")
    if flag {
        close(h)
        return                  // fine: this path releases it
    }
    close(h)                    // and so does this one
}
```

Rules that keep the obligation sound:

- **Linearity is declared, not applied**: `linear` cannot be written in a use-site type — every value of a `linear struct` type is linear, always. (A spelling you could forget would defeat the point.) `canbe linear` on a declaration is an error naming the modifier; on a *type parameter* it keeps its spelling, where it means something else entirely (below).
- **A container is linear exactly when its element is.** A `List<FileHandle>` owes; a `List<Int>` does not; nothing at a use site says which — the element's declaration is the source, and the container opts a parameter in on *its* declaration (`intrinsic type List<T canbe linear>`, `struct Box<T canbe linear>`). Obligations enter with `add`/`replace`, leave one at a time with `remove_first`/`remove_at`/`remove` (each answering `T?`, so the emptiness check is the ordinary narrow), and the container's **terminal** is `drain(container, each)`: it consumes the container and hands every element to a callback that consumes it. A container that is neither drained nor moved onward is an ordinary leak, and the diagnostic names `drain`.

  What a container may *not* do is drop an element on your behalf, so the surface has no `clear`, no positional list write, and no `get` for obligations (that would hand out an alias). `Set`, `SortedSet` and map **keys** refuse obligations outright: insertion deduplicates, and dedup *is* dropping — an equal element or a repeated key discards one of the two values, which no API reshaping can fix. A struct field holding either an obligation or a container of them makes the struct a resource too, so it takes the `linear struct` marker: contagion is spelled, never inferred. Arrays and tuples stay out for now, and a linear value still cannot travel through a *variadic* position (those are untracked).

- **Handler state may hold obligations, and the actor owes until it ends.** A queue of parked reply tokens (`waiting: Mut List<Reply<Str>>`) is what the concurrency surface is for, so a handler field may hold a container of obligations. Within one activation the discipline is unchanged — take one out, and either discharge it or put something back — and a member that *returns* with a state field moved out is an error, because it would leave the actor with a hole a later activation would read. This is the one place the promise weakens: static analysis cannot know what an actor holds at an arbitrary future point, so the guarantee becomes "the actor owes until it ends", and what a *death* does with parked obligations is what `watch` reports. A **bare** obligation in a field is refused, naming the container: taking it out would leave that hole and nothing could be put back, so its obligation would have no reachable discharge at all.
- **Generics opt in per type parameter**: an unconstrained `T` cannot be instantiated with a linear type, but a function may declare `fn hold<T canbe linear>(value: T) -> T` — the same `canbe linear` phrase as on type declarations, now opting the *function's handling* in. Inside the body, `T` values are treated as linear (they must be discharged on every path); in exchange, callers may instantiate `T` with linear types, and an opted `T` forwarded to another generic requires that one to be opted too. The standard library's collection surface is audited and opted where sound (`list_of`, `mut_list_of`, `add`, `size`, `remove_first`, `remove_at`, `drain`, and a map's `remove`/`replace`/`drain`), and since containers carry obligations those opt-ins are permissions to *store* as well as to call. `get` stays out (it returns an alias of an element, which would duplicate the obligation) and `copy` refuses linear values outright. `discard`'s declaration is simply `intrinsic fn discard<T canbe linear>(value: T) -> None => !value`. One extra rule: a linear value cannot be passed in a *variadic* position (those are untracked).
- **Lambdas may read but not swallow**: a lambda can read-capture a linear value (an alias), but a capture the body mutates would move the obligation into the closure — an error.
- **Purely static, on both backends**: like the rest of the ownership system, linearity is a protocol the compiler enforces; there is no runtime component and no destructor on either backend, and the discipline is identical on the JVM and in Rust.
