// R0 prototype, Kotlin: `passes.sv` as the emitters would produce it once
// `Iter<T>` is gone and `next` is the only protocol.
//
// Absent, compared with today's output: the `SalvoPass<T>` runtime base class,
// `Iterable<T>` as `Iter<T>`'s representation, the `fun interface` factory,
// the per-effect-set pass interface, the variance adapter. A pass is a class;
// a `for` calls `next` on it; a fresh pass is a fresh instance.
//
// New: `SalvoYield<T>` — the obligation group as an interface, used **only**
// as a bound (`P : SalvoYield<T>`), which is what [group-not-a-value] buys:
// no variable, field or parameter anywhere has an interface type.
//
//   kotlinc passes.kt -d ../../tmp/next/classes && kotlin -cp ../../tmp/next/classes salvo.PassesKt

package salvo

// =====================================================================
// generated: unions.kt
// =====================================================================

sealed interface Union2<out T1, out T2> {
    val value: Any?
}

data class U2_1<out T1, out T2>(override val value: T1) : Union2<T1, T2>
data class U2_2<out T1, out T2>(override val value: T2) : Union2<T1, T2>

// =====================================================================
// std/core/console.sv (unchanged)
// =====================================================================

interface Console {
    fun print(message: String)
}

class StdOutConsole : Console {
    override fun print(message: String) {
        kotlin.io.print(message)
    }
}

fun println(console: Console, message: String) {
    console.print(message)
    console.print("\n")
}

// =====================================================================
// std/core/iterator.sv, as the reduction leaves it
// =====================================================================

class Finished

fun <T> emitted(value: T): T {
    return value
}

fun finished(): Finished {
    return Finished()
}

/// [yield-group] `params Yield<T>` with `Self` bound to the declaring type.
/// An interface, because a generic composed pass has to call `next` on a type
/// it does not know — and a *bound* only, never a declared type.
interface SalvoYield<T> {
    fun __next(): Union2<T, Finished>
}

// =====================================================================
// 1. A hand-written pass
// =====================================================================

data class Countdown(var at: Int) : SalvoYield<Int> {
    // Generated from `struct Countdown : Yield<Int>`: forwards to the free
    // function ordinary call sites already use.
    override fun __next(): Union2<Int, Finished> {
        return next(this)
    }
}

fun next(c: Countdown): Union2<Int, Finished> {
    if (c.at <= 0) {
        return U2_2<Int, Finished>(finished())
    }
    val v = c.at
    c.at = c.at - 1
    return U2_1<Int, Finished>(emitted(v))
}

fun countdown(from: Int): Countdown {
    return Countdown(at = from)
}

// =====================================================================
// 2. A `yield`-generated pass, with a `defer` and an effect
// =====================================================================

/// Generated for `fn chatty(limit: Int) [Console] -> Chatty : Yield<Int>`.
/// The parameters and the body's locals are properties, `__d0` is the
/// `defer` site's flag — `generator.rs`'s plan, unchanged.
class Chatty(val limit: Int) {
    var i: Int = 0
    var __state: Int = 0
    var __d0: Boolean = false

    fun __run_d0(console: Console) {
        if (__d0) {
            __d0 = false
            println(console, "close")
        }
    }
}

fun chatty(limit: Int): Chatty {
    return Chatty(limit = limit)
}

/// The plan's `__advance`, spelled as the group member: handlers are
/// parameters, per resume, so nothing is captured.
fun next(c: Chatty, console: Console): Union2<Int, Finished> {
    while (true) {
        when (c.__state) {
            0 -> {
                println(console, "open")
                c.__d0 = true
                c.i = 0
                c.__state = 1
            }
            // `while i < limit {`
            1 -> {
                if (!(c.i < c.limit)) {
                    c.__state = 3
                    continue
                }
                println(console, "make ${c.i}")
                val v = c.i
                // The yield is at the end of the loop body, so its resume
                // point is the loop head; state 2 is the tail after it.
                c.__state = 2
                return U2_1<Int, Finished>(emitted(v))
            }
            2 -> {
                c.i = c.i + 1
                c.__state = 1
            }
            // the end of the fn block: the release path runs
            3 -> {
                c.__run_d0(console)
                c.__state = 4
                return U2_2<Int, Finished>(finished())
            }
            else -> return U2_2<Int, Finished>(finished())
        }
    }
}

/// The `Linear` member: **consuming** in Salvo, which this backend cannot
/// enforce — so the flag guard inside is what keeps a second call harmless,
/// and the checker is what keeps a use-after-close from being written.
fun close(c: Chatty, console: Console) {
    c.__run_d0(console)
}

// =====================================================================
// 3. A composed pass — rendering (A), the group as a generic bound
// =====================================================================

/// Generated for `fn map_a<P: Yield<T>, T, U>(…) -> MapA<P, T, U> : Yield<U>`.
/// The source is a property (it must survive suspensions) and so is the
/// `for`'s element binding, as a nullable slot because `T` has no zero.
class MapA<P : SalvoYield<T>, T, U>(val src: P, val f: (T) -> U) : SalvoYield<U> {
    var __x: T? = null
    var __state: Int = 0

    override fun __next(): Union2<U, Finished> {
        return next(this)
    }
}

fun <P : SalvoYield<T>, T, U> mapA(src: P, f: (T) -> U): MapA<P, T, U> {
    return MapA(src = src, f = f)
}

/// The bound is what justifies `m.src.__next()`.
fun <P : SalvoYield<T>, T, U> next(m: MapA<P, T, U>): Union2<U, Finished> {
    while (true) {
        when (m.__state) {
            // `for x in src {`
            0 -> {
                val step = m.src.__next()
                if (step is U2_1<T, Finished>) {
                    m.__x = step.value
                    val v = m.f(m.__x!!)
                    // yield at the end of the loop body: back to the head
                    m.__state = 0
                    return U2_1<U, Finished>(emitted(v))
                }
                m.__state = 1
            }
            1 -> {
                m.__state = 2
                return U2_2<U, Finished>(finished())
            }
            else -> return U2_2<U, Finished>(finished())
        }
    }
}

// =====================================================================
// 4. A composed pass — rendering (B), the member as an implicit parameter
// =====================================================================

/// The source's protocol arrives as stored function values rather than as a
/// bound — which is how the *existing* generated `map_lazy` already carries
/// its `?Iterable` member, so this rendering needs nothing new.
///
/// The source sits in a nullable slot because closing it consumes it.
class MapB<P, T, U>(
    var src: P?,
    val f: (T) -> U,
    val nextSrc: (P, Console) -> Union2<T, Finished>,
    val closeSrc: (P, Console) -> Unit,
) {
    var __x: T? = null
    var __state: Int = 0

    /// The plan's `ClosePass` step: close the nested pass, once. The local
    /// binding is not decoration — kotlinc will not smart-cast a mutable
    /// property.
    fun __close_src(console: Console) {
        val p = src
        if (p != null) {
            src = null
            closeSrc(p, console)
        }
    }
}

fun <P, T, U> mapB(
    src: P,
    f: (T) -> U,
    nextSrc: (P, Console) -> Union2<T, Finished>,
    closeSrc: (P, Console) -> Unit,
): MapB<P, T, U> {
    return MapB(src = src, f = f, nextSrc = nextSrc, closeSrc = closeSrc)
}

fun <P, T, U> next(m: MapB<P, T, U>, console: Console): Union2<U, Finished> {
    while (true) {
        when (m.__state) {
            0 -> {
                val p = m.src!!
                val step = m.nextSrc(p, console)
                if (step is U2_1<T, Finished>) {
                    m.__x = step.value
                    val v = m.f(m.__x!!)
                    m.__state = 0
                    return U2_1<U, Finished>(emitted(v))
                }
                m.__state = 1
            }
            1 -> {
                m.__close_src(console)
                m.__state = 2
                return U2_2<U, Finished>(finished())
            }
            else -> return U2_2<U, Finished>(finished())
        }
    }
}

fun <P, T, U> close(m: MapB<P, T, U>, console: Console) {
    m.__close_src(console)
}

// =====================================================================
// 5 & 6. A linear pass
// =====================================================================

data class Lines(val name: String, val count: Int, var at: Int) : SalvoYield<String> {
    override fun __next(): Union2<String, Finished> {
        return next(this)
    }
}

fun next(l: Lines): Union2<String, Finished> {
    if (l.at >= l.count) {
        return U2_2<String, Finished>(finished())
    }
    l.at = l.at + 1
    val row: String = "line ${l.at} of ${l.name}"
    return U2_1<String, Finished>(emitted(row))
}

fun close(l: Lines, console: Console) {
    println(console, "closing ${l.name}")
}

fun openLines(name: String, count: Int, console: Console): Lines {
    println(console, "opening $name")
    return Lines(name = name, count = count, at = 0)
}

// =====================================================================
// The program
// =====================================================================

fun main() {
    val console: Console = StdOutConsole()

    // --- 1: a pass beside a List ---
    var __loop1_pass = countdown(3)
    while (true) {
        val __loop1_step = next(__loop1_pass)
        if (__loop1_step !is U2_1<Int, Finished>) { break }
        val n = __loop1_step.value
        println(console, "n $n")
    }
    val names = listOf<String>("ada", "grace")
    for (s in names) {
        println(console, "s $s")
    }

    // --- 2: a generated pass, abandoned after two elements. `close` lands in
    // a `finally`, which is how `defer` is lowered here anyway. ---
    var seen = 0
    var __loop3_pass = chatty(3)
    try {
        while (true) {
            val __loop3_step = next(__loop3_pass, console)
            if (__loop3_step !is U2_1<Int, Finished>) { break }
            val v = __loop3_step.value
            println(console, "got $v")
            seen = seen + 1
            if (seen == 2) {
                break
            }
        }
    } finally {
        close(__loop3_pass, console)
    }

    // --- 3: composition by bound, over a pure source ---
    var __loop4_pass = mapA<Countdown, Int, Int>(countdown(3), { n -> n * 2 })
    while (true) {
        val __loop4_step = next(__loop4_pass)
        if (__loop4_step !is U2_1<Int, Finished>) { break }
        val v = __loop4_step.value
        println(console, "A $v")
    }

    // --- 4: composition by implicit, over an *effectful* source ---
    var __loop5_pass = mapB<Chatty, Int, Int>(
        chatty(3),
        { n -> n * 2 },
        { p, c -> next(p, c) },
        { p, c -> close(p, c) },
    )
    try {
        while (true) {
            val __loop5_step = next(__loop5_pass, console)
            if (__loop5_step !is U2_1<Int, Finished>) { break }
            val v = __loop5_step.value
            println(console, "B $v")
        }
    } finally {
        close(__loop5_pass, console)
    }

    // --- 5: a linear pass, drained ---
    var __loop6_pass = openLines("a.txt", 2, console)
    try {
        while (true) {
            val __loop6_step = next(__loop6_pass)
            if (__loop6_step !is U2_1<String, Finished>) { break }
            val row = __loop6_step.value
            println(console, row)
        }
    } finally {
        close(__loop6_pass, console)
    }

    // --- 6: a linear pass, abandoned ---
    var __loop7_pass = openLines("b.txt", 2, console)
    try {
        while (true) {
            val __loop7_step = next(__loop7_pass)
            if (__loop7_step !is U2_1<String, Finished>) { break }
            val row = __loop7_step.value
            println(console, row)
            break
        }
    } finally {
        close(__loop7_pass, console)
    }

    println(console, "done")
}
