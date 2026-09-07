// I1b prototype, Kotlin: the same source.sv under the pull-iterator design.
// Deliberately the *same* structure as main.rs rather than an idiomatic
// Kotlin iterator — one lowering on both backends (decision 6), so the
// operational semantics are identical by construction rather than by
// argument. This is what replaces `Iterable { iterator { … } }`.
//
// The consequence to notice: `iterator { … }` could not have taken the
// handlers per resume, only captured them. Threading them is what lets an
// iterator function perform effects, and what keeps creation-time and
// consumption-time binding from diverging between the backends.

package salvo

// ---------------------------------------------------------------- runtime
// Would live in runtime/iter.kt. In the real emission this is the generated
// sealed union wrapper for `Next T | Stopped`.

sealed class SalvoStep<out T> {
    class Next<out T>(val value: T) : SalvoStep<T>()
    object Stopped : SalvoStep<Nothing>()
}

// ---------------------------------------------------------------- effects

interface Console {
    fun println(line: String)
}

interface FileSystem {
    fun open(path: String): Int
    fun readLine(handle: Int): String?
    fun close(handle: Int)
}

// --------------------------------------------------------------- handlers

class StdOutConsole : Console {
    override fun println(line: String) {
        kotlin.io.println(line)
    }
}

/// Four lines, so the consumer's `break` really is early.
class FakeFileSystem : FileSystem {
    private val lines = listOf("alpha", "beta", "gamma", "delta")
    private var at = 0

    override fun open(path: String): Int {
        at = 0
        return 7
    }

    override fun readLine(handle: Int): String? {
        val line = lines.getOrNull(at)
        if (line != null) {
            at += 1
        }
        return line
    }

    override fun close(handle: Int) {}
}

// ------------------------------------------------------- the pass: lines()

class LinesPass(private val path: String) {
    /// A body local (`let f = open(path)`), live from state 1 onwards.
    private var f: Int = 0

    /// Which resume point to enter on the next call.
    /// 0 = entry, 1 = the `while true` head, 2 = leaving, 3 = finished.
    private var state: Int = 0

    /// One flag per `defer` site, set when the site is passed.
    private var deferClosePending: Boolean = false

    /// The effects the declaration lists arrive per resume, in declaration
    /// order — the same convention as any other emitted Salvo function.
    fun next(fs: FileSystem, console: Console): SalvoStep<String> {
        while (true) {
            when (state) {
                // Entry: everything up to the first `yield`.
                0 -> {
                    console.println("opening $path")
                    f = fs.open(path)
                    deferClosePending = true
                    state = 1
                    return SalvoStep.Next("-- $path --")
                }
                // The `while true` head, which is also where the second
                // `yield` resumes to.
                1 -> {
                    val l = fs.readLine(f)
                    if (l != null) {
                        state = 1
                        return SalvoStep.Next(l)
                    }
                    // The bare `return`: leave the body, running defers.
                    state = 2
                }
                // Leaving normally: the block's deferred code runs here, on
                // the way out, exactly as in a non-iterator fn.
                2 -> {
                    runPendingDefers(fs, console)
                    state = 3
                    return SalvoStep.Stopped
                }
                // Finished. Driving a finished pass keeps reporting Stopped.
                else -> return SalvoStep.Stopped
            }
        }
    }

    /// The injected close path: the same deferred code, reached because the
    /// consumer stopped rather than because the body did. Idempotent, so one
    /// call after the loop covers exhaustion and `break` alike.
    fun close(fs: FileSystem, console: Console) {
        runPendingDefers(fs, console)
        state = 3
    }

    /// Deferred blocks run latest-registered first, and may perform effects —
    /// which is why this takes the handlers, and why the JVM's lack of
    /// deterministic destruction costs nothing here.
    private fun runPendingDefers(fs: FileSystem, console: Console) {
        if (deferClosePending) {
            deferClosePending = false
            console.println("closing $path")
            fs.close(f)
        }
    }
}

fun lines(path: String): LinesPass = LinesPass(path)

// -------------------------------------------------------------------- main

fun main() {
    val console: Console = StdOutConsole()
    val fs: FileSystem = FakeFileSystem()

    var seen = 0

    // `for l in lines("data.txt") { ... }`
    val pass = lines("data.txt")
    while (true) {
        val step = pass.next(fs, console)
        if (step !is SalvoStep.Next) {
            break
        }
        val l = step.value
        console.println("line $l")
        seen += 1
        if (seen == 3) {
            break
        }
    }
    // Injected on the way out of the loop: covers exhaustion and every
    // `break`, because both land here.
    pass.close(fs, console)

    console.println("done")
}
