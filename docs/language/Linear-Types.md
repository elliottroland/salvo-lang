# Linear types

Salvo's ownership rules make values *affine*: they can be used at most once (see [Deductions and ownership](Deductions-and-Ownership.md)). Resource types want the other half too — a file handle that is never closed, a transaction that is never committed or rolled back, is a bug. A type declares **linearity** with a modifier, and its file supplies the death:

```
linear struct FileHandle {
    fd: Int
}

fn close(handle: FileHandle) -> None => !handle {
    // release the resource; consuming `handle` here is what ends it
}
```

The **discharge set** is every function — and every *effect member* — declared in the *type's own file* that consumes a parameter of the type — `close` for a file, `stop` *or* `join` for a thread handle, `remove(cache, entry)` for a pooled one (the extra parameters are ordinary parameters). A `linear struct` whose file has no such function is an error at the struct: the obligation would have no legal death. Leak diagnostics name the whole set. A discharger's body ending is the obligation's end: the parameter it consumes is settled however the body finishes, and it may read the value up to then. Consuming the value in the type's own file is the explicit act, so a discharger may also forward instead (`fn shutdown(t: Thread) => !t { stop(t) }`). A type with linear fields is the exception: its discharger still moves each linear field out, since only the container's own obligation is the body's to end.

When the discharger is an effect member, the declaration is what carries that status, so **every handler's implementation of it** is a discharge context — the real one, a test double in another module, an interceptor that discharges by forwarding into the handler it wraps. That is how a stream token stays linear while `close` is a member of `Fs`: one declaration, many implementations, all of them ending the obligation and none of them able to end anybody else's (a member consuming an `InStream` does not end an `OutStream` it makes).

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

There is no escape hatch: `drop(x)`, which deliberately drops an ordinary value, refuses a linear one — dropping a handle is exactly the leak the obligation exists to prevent.

The `maybe_leak` shape above — a value that must be released however the block ends — is written out: `close(h)` on each path. The compiler names the path you missed, which is the whole point; outside a discharger nothing discharges an obligation implicitly (`defer` did, and was removed 2026-09-10 for exactly that reason).

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

  What a container may *not* do is drop an element on your behalf, so the surface has no `clear`, and no positional list write. `get` and `first` still work on a container of obligations: they answer a *projection*, which you can read (and mutate, through `Mut`) but cannot give away or copy, so the element keeps owing inside its container. `Set`, `SortedSet` and map **keys** refuse obligations outright: insertion deduplicates, and dedup *is* dropping — an equal element or a repeated key discards one of the two values, which no API reshaping can fix. A struct field holding either an obligation or a container of them makes the struct a resource too, so it takes the `linear struct` marker: contagion is spelled, never inferred. A **tuple** of obligations owes the same way, `(FileHandle, Int)` exactly when a component does. Destructure it (`let (h, n) = open_sized(p)`) or move each linear component out (`close(p.0)`); a component you leave behind, or skip with `_`, is a leak. That is what lets a map of obligations be built in one call, since its entries are pairs: `Map("a": reply_a, "b": reply_b)`.

  An **array** holds obligations the same way, and needs no opt-in: `FileHandle[]` owes exactly when its element does. A move out of an array is always the whole array: pass it on, `append(list, items)` its elements onto a list, or `drain(items, each)` them into a callback. A `for` may read a linear array but not move its elements out, since a loop can stop early and the elements it did not reach would be owed by nobody. Because constructors consume their variadic tail, a container of obligations is built in one call:

  ```
  fn close_all(...hs: FileHandle[]) [] -> None => !hs {
      drain(hs, close)
  }

  let pending = List(open_file(1), open_file(2))   // a List<FileHandle>, owing
  close_all(open_file(3), open_file(4))
  ```

  A variadic tail the callee *keeps* cannot take linear values: it is lent to the call and handed back, so a value moved into it would still be owed.

- **Handler state may hold obligations, and the actor owes until it ends.** A queue of parked reply tokens (`waiting: Mut List<Reply<Str>>`) is what the concurrency surface is for, so a handler field may hold a container of obligations. Within one activation the discipline is unchanged — take one out, and either discharge it or put something back — and a member that *returns* with a state field moved out is an error, because it would leave the actor with a hole a later activation would read. This is the one place the promise weakens: static analysis cannot know what an actor holds at an arbitrary future point, so the guarantee becomes "the actor owes until it ends", and what a *death* does with parked obligations is what `watch` reports. A **bare** obligation in a field is refused, naming the container: taking it out would leave that hole and nothing could be put back, so its obligation would have no reachable discharge at all.
- **Generics opt in per type parameter**: an unconstrained `T` cannot be instantiated with a linear type, but a function may declare `fn hold<T canbe linear>(value: T) -> T` — the same `canbe linear` phrase as on type declarations, now opting the *function's handling* in. Inside the body, `T` values are treated as linear (they must be discharged on every path); in exchange, callers may instantiate `T` with linear types, and an opted `T` forwarded to another generic requires that one to be opted too. The standard library's collection surface is audited and opted where sound (`List`, `Mut List`, `add`, `size`, `remove_first`, `remove_at`, `drain`, and a map's `remove`/`replace`/`drain`), and since containers carry obligations those opt-ins are permissions to *store* as well as to call. `get` and `first` are opted too, since they answer a projection that cannot be consumed, and `copy` refuses linear values outright. One extra rule: a linear value cannot be passed to a variadic tail the callee keeps (see above).
- **Lambdas may read but not swallow**: a lambda can read-capture a linear value (an alias), but a capture the body mutates would move the obligation into the closure — an error.
- **Purely static, on both backends**: like the rest of the ownership system, linearity is a protocol the compiler enforces; there is no runtime component and no destructor on either backend, and the discipline is identical on the JVM and in Rust.
