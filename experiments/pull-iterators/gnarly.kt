// I3 prototype, Kotlin: gnarly.sv as the same hand-written state machine.
// Deliberately the same structure as gnarly.rs — one lowering on both
// backends — and it must print what gnarly_oracle.rs prints.

package salvo

sealed class Step<out T> {
    class Emitted<out T>(val value: T) : Step<T>()
    object Finished : Step<Nothing>()
}

interface Console {
    fun println(line: String)
}

class StdOutConsole : Console {
    override fun println(line: String) {
        kotlin.io.println(line)
    }
}

// ------------------------------------------------ the hand-written pass

class Countdown(private var at: Int) {
    fun next(): Step<Int> {
        if (at <= 0) {
            return Step.Finished
        }
        val v = at
        at -= 1
        return Step.Emitted(v)
    }
}

fun countdown(from: Int): Countdown = Countdown(from)

// ------------------------------------------------------ walk, as a pass
//
// Resume points, as in gnarly.rs:
//   0 entry, 1 outer head, 2 inner head (and the first yield's resume),
//   3 the second yield, 4 its resume (the loop-body tail), 5 the yield after
//   the loop, 6 its resume (the end of the fn block), 7 finished.

class WalkPass(private val limit: Int) {
    private var row = 0
    private var r = 0

    /// The inner pass outlives the outer body's suspensions, so it is a field.
    private var inner: Countdown? = null

    private var state = 0

    /// `defer { println("close") }` — fn-block level, no captures.
    private var d0Pending = false

    /// `defer { println("row ${r} end") }` — loop-body level, reads `r`. One
    /// slot is enough: a loop-body defer cannot outlive its iteration, so it
    /// is discharged (and the flag cleared) before the back edge.
    private var d1Pending = false

    fun next(console: Console): Step<Int> {
        while (true) {
            when (state) {
                0 -> {
                    console.println("open")
                    d0Pending = true
                    row = 0
                    state = 1
                }
                // `while row < limit {`
                1 -> {
                    if (row >= limit) {
                        state = 5
                        continue
                    }
                    r = row
                    d1Pending = true
                    inner = countdown(2)
                    state = 2
                }
                // `for col in countdown(2) {` — and the resume point of the
                // yield inside it, so `continue` is just staying here.
                2 -> {
                    val step = inner!!.next()
                    if (step is Step.Emitted) {
                        val col = step.value
                        if (col == 1) {
                            continue
                        }
                        val v = r * 10 + col
                        state = 2
                        return Step.Emitted(v)
                    }
                    inner = null
                    state = 3
                }
                // `yield r * 100`
                3 -> {
                    val v = r * 100
                    state = 4
                    return Step.Emitted(v)
                }
                // The tail of the outer loop body: the increment, then the end
                // of the block, which is where its `defer` runs.
                4 -> {
                    row += 1
                    runD1(console)
                    state = 1
                }
                // `yield 999`, after the loop.
                5 -> {
                    state = 6
                    return Step.Emitted(999)
                }
                // The end of the fn block: its `defer` runs, then the body is
                // over.
                6 -> {
                    runD0(console)
                    state = 7
                    return Step.Finished
                }
                else -> return Step.Finished
            }
        }
    }

    /// The injected release path: latest-registered first, and idempotent, so
    /// one call after the loop covers `break` and exhaustion alike.
    fun close(console: Console) {
        runD1(console)
        runD0(console)
        state = 7
    }

    private fun runD1(console: Console) {
        if (d1Pending) {
            d1Pending = false
            console.println("row $r end")
        }
    }

    private fun runD0(console: Console) {
        if (d0Pending) {
            d0Pending = false
            console.println("close")
        }
    }
}

fun walk(limit: Int): WalkPass = WalkPass(limit)

// -------------------------------------------------------------------- main

fun main() {
    val console: Console = StdOutConsole()

    var seen = 0
    val pass = walk(2)
    while (true) {
        val step = pass.next(console)
        if (step !is Step.Emitted) {
            break
        }
        console.println("got ${step.value}")
        seen += 1
        if (seen == 4) {
            break
        }
    }
    pass.close(console)

    console.println("done")
}
