// I4 prototype, Kotlin: effectful.sv as the emitters would produce it.
//
// The same three shapes as effectful.rs, with the differences the JVM forces:
//
//  1. **An interface per effect set**, generated like the union wrappers. The
//     pass cannot be a Kotlin `Iterator<T>` — its `advance` takes a handler —
//     so it is not one, and the `for` lowering is ours anyway.
//  2. **Advance-and-report, not `next()`**: `advance` returns whether there is
//     an element and leaves it in `current`, because a `T?` return could not
//     tell "no more" from "the element is null". Same reason the pure runtime's
//     `SalvoPass` holds `Any?` and the protocol tags its end.
//  3. **Variance is an adapter here too** — a pure `Iterable<T>` used where a
//     claiming producer is expected gets wrapped, and its `advance` ignores the
//     handler.
//
// Prints byte-for-byte what effectful.rs prints.

package salvo

// -------------------------------------------------------- the effect (as today)

interface Console {
    fun println(line: String)
}

class StdOutConsole : Console {
    override fun println(line: String) {
        kotlin.io.println(line)
    }
}

// ------------------------------ runtime: generated per effect set, here {Console}

interface SalvoPassConsole<T> {
    /// Advance one element; `true` means `current()` holds it.
    fun advance(console: Console): Boolean
    fun current(): T
    /// The release path: pending deferred blocks, latest first. Idempotent.
    fun close(console: Console)
}

/// `Console Iter<T>`: a factory of those, so it stays replayable.
fun interface SalvoIterConsole<T> {
    fun mint(): SalvoPassConsole<T>
}

/// [iter-effects] The variance adapter: a producer performing *fewer* effects
/// fits where more are expected, and the two representations differ, so the
/// compiler inserts this at the boundary.
class PureAsConsole<T>(private val source: Iterable<T>) : SalvoIterConsole<T> {
    override fun mint(): SalvoPassConsole<T> {
        val it = source.iterator()
        return object : SalvoPassConsole<T> {
            private var value: Any? = null
            override fun advance(console: Console): Boolean {
                if (!it.hasNext()) {
                    return false
                }
                value = it.next()
                return true
            }

            @Suppress("UNCHECKED_CAST")
            override fun current(): T = value as T
            override fun close(console: Console) {}
        }
    }
}

// -------------------------------------------------------- chatty, as a pass

private class PassChatty(private var limit: Int) : SalvoPassConsole<Int> {
    private var i: Int = 0
    private var state: Int = 0
    private var d0: Boolean = false
    private var value: Any? = null

    private fun runD0(console: Console) {
        if (d0) {
            d0 = false
            console.println("close")
        }
    }

    override fun advance(console: Console): Boolean {
        while (true) {
            when (state) {
                0 -> {
                    console.println("open")
                    d0 = true
                    i = 0
                    state = 1
                    continue
                }
                1 -> {
                    if (!(i < limit)) {
                        state = 3
                        continue
                    }
                    console.println("make $i")
                    value = i
                    state = 2
                    return true
                }
                2 -> {
                    i = i + 1
                    state = 1
                    continue
                }
                3 -> {
                    runD0(console)
                    state = 4
                    return false
                }
                else -> return false
            }
        }
    }

    @Suppress("UNCHECKED_CAST")
    override fun current(): Int = value as Int

    override fun close(console: Console) {
        runD0(console)
        state = 4
    }
}

fun chatty(limit: Int): SalvoIterConsole<Int> = SalvoIterConsole { PassChatty(limit) }

// --------------------------------------------------------- plain, as today

private class PassPlain(private var limit: Int) : Iterator<Int> {
    private var i: Int = 0
    private var state: Int = 0
    private var ready = false
    private var value: Int = 0

    private fun step(): Boolean {
        while (true) {
            when (state) {
                0 -> {
                    i = 0
                    state = 1
                    continue
                }
                1 -> {
                    if (!(i < limit)) {
                        state = 2
                        continue
                    }
                    value = i
                    state = 3
                    return true
                }
                3 -> {
                    i = i + 1
                    state = 1
                    continue
                }
                else -> return false
            }
        }
    }

    override fun hasNext(): Boolean {
        if (!ready) {
            ready = step()
        }
        return ready
    }

    override fun next(): Int {
        if (!hasNext()) {
            throw NoSuchElementException()
        }
        ready = false
        return value
    }
}

fun plain(limit: Int): Iterable<Int> = Iterable<Int> { PassPlain(limit) }

// ----------------------------------------------------------------- total

/// Inherits `Console` from its parameter, so the handler is a leading
/// parameter exactly as a declared effect's would be.
fun total(xs: SalvoIterConsole<Int>, console: Console): Int {
    var sum = 0
    val pass = xs.mint()
    while (pass.advance(console)) {
        sum = sum + pass.current()
    }
    // The injected close: one call covers exhaustion and every early exit,
    // because the flags make it idempotent.
    pass.close(console)
    return sum
}

// ------------------------------------------------------------------ main

fun main() {
    val console: Console = StdOutConsole()

    val pass = chatty(3).mint()
    while (pass.advance(console)) {
        val v = pass.current()
        console.println("got $v")
        if (v == 1) {
            break
        }
    }
    pass.close(console)

    val sum = total(chatty(2), console)
    console.println("sum $sum")

    // The variance boundary: the adapter goes in here and nowhere else.
    val plainSum = total(PureAsConsole(plain(4)), console)
    console.println("plain $plainSum")
}
