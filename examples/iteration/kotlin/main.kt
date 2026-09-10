package salvo.main

import salvo.*
import salvo.core.array.*
import salvo.core.console.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.seq.*
import salvo.core.string.*

fun describe_container(console: Console, xs: List<Int>) {
    var sum = 0
    for (n in xs) {
        sum = sum + n
    }
    println(console, "1. list of ${xs.size} sums to $sum")
    val letters = StringBuilder()
    for (c in "salvo") {
        letters.append("$c.")
    }
    println(console, "1. string: ${letters.toString()}")
    val arr = arrayOf(10, 20, 30)
    var from_array = 0
    for (n in arr) {
        from_array = from_array + n
    }
    println(console, "1. array sums to $from_array")
}

data class Countdown(
    var at: Int,
)

fun countdown(from: Int): Countdown {
    return Countdown(at = from)
}

fun next__6(p: Countdown): Union2<Int, Finished> {
    if (p.at <= 0) {
        return U2_2<Int, Finished>(finished())
    }
    val now = p.at
    p.at = p.at - 1
    return U2_1<Int, Finished>(emitted(now))
}

fun take(console: Console, p: Countdown, count: Int) {
    var seen = 0
    while (true) {
        val __loop1_step = next__6(p)
        if (__loop1_step !is U2_1<Int, Finished>) { break }
        val n = __loop1_step.value
        println(console, "2. got $n")
        seen = seen + 1
        if (seen == count) {
            break
        }
    }
}

data class Halving(
    val start: Int,
)

data class __Pass_Halving(
    var at: Int,
)

fun iter__4(h: Halving): __Pass_Halving {
    return __Pass_Halving(at = h.start)
}

fun next__7(__p: __Pass_Halving): Union2<Int, Finished> {
    if (__p.at <= 0) {
        return U2_2<Int, Finished>(finished())
    }
    val now = __p.at
    __p.at = __p.at / 2
    return U2_1<Int, Finished>(emitted(now))
}

data class Fibs(
    val count: Int,
)

fun fibs(count: Int): Fibs {
    return Fibs(count = count)
}

class __Pass_Fibs(private var f: Fibs) {
    private var a: Int = 0
    private var b: Int = 0
    private var made: Int = 0
    private var sum: Int = 0
    private var __d0: Boolean = false
    private var __state: Int = 0
    private var __current: Any? = null

    fun __advance(console: Console): Boolean {
        while (true) {
            when (__state) {
                0 -> {
                    println(console, "3. opening")
                    __d0 = true
                    a = 0
                    b = 1
                    made = 0
                    __state = 1
                    continue
                }
                1 -> {
                    if (!(made < f.count)) {
                        __state = 3
                        continue
                    }
                    __current = a
                    __state = 2
                    return true
                }
                2 -> {
                    sum = a + b
                    a = b
                    b = sum
                    made = made + 1
                    __state = 1
                    continue
                }
                3 -> {
                    __run_d0(console)
                    __state = 4
                    return false
                }
                4 -> {
                    __state = 4
                    return false
                }
                else -> return false
            }
        }
    }

    fun __close(console: Console) {
        __run_d0(console)
        __state = 4
    }

    private fun __run_d0(console: Console) {
        if (__d0) {
            __d0 = false
            println(console, "3. closing")
        }
    }

    @Suppress("UNCHECKED_CAST")
    fun __current(): Int = __current as Int
}

data class Naturals(
    val from: Int,
)

fun naturals(from: Int): Naturals {
    return Naturals(from = from)
}

class __Pass_Naturals(private var n: Naturals) {
    private var i: Int = 0
    private var __state: Int = 0
    private var __current: Any? = null

    fun __advance(): Boolean {
        while (true) {
            when (__state) {
                0 -> {
                    i = n.from
                    __state = 1
                    continue
                }
                1 -> {
                    if (!(true)) {
                        __state = 3
                        continue
                    }
                    __current = i
                    __state = 2
                    return true
                }
                2 -> {
                    i = i + 1
                    __state = 1
                    continue
                }
                3 -> {
                    __state = 3
                    return false
                }
                else -> return false
            }
        }
    }

    fun __close() {
        __state = 3
    }

    @Suppress("UNCHECKED_CAST")
    fun __current(): Int = __current as Int
}

fun<It> sum_of(it: It, next: (It) -> Union2<Int, Finished>): Int {
    var total = 0
    while (true) {
        val __loop2_step = next(it)
        if (__loop2_step !is U2_1<*, *>) { break }
        val n = __loop2_step.value as Int
        total = total + n
    }
    return total
}

fun main() {
    val console: Console = StdOutConsole()
    val xs = listOf<Int>(1, 2, 3, 4)
    describe_container(console, xs)
    val p = countdown(5)
    take(console, p, 2)
    println(console, "2. rest sums to ${sum_of(p, ::next__6)}")
    val h = Halving(start = 20)
    var __loop3_pass = iter__4(h)
    while (true) {
        val __loop3_step = next__7(__loop3_pass)
        if (__loop3_step !is U2_1<Int, Finished>) { break }
        val n = __loop3_step.value
        println(console, "2b. halving $n")
    }
    val hp = iter__4(h)
    println(console, "2b. summed from a held pass: ${sum_of(hp, ::next__7)}")
    val __loop4_pass = __Pass_Fibs(fibs(6))
    try {
    while (__loop4_pass.__advance(console)) {
        val n = __loop4_pass.__current()
        println(console, "3. fib $n")
    }
    } finally {
        __loop4_pass.__close(console)
    }
    println(console, "3. again sums to ${run { val __mint1 = __Pass_Fibs(fibs(6)); val __call = sum_of(__mint1, fun(__p: __Pass_Fibs): Union2<Int, Finished> { return if (__p.__advance(console)) U2_1<Int, Finished>(__p.__current()) else U2_2<Int, Finished>(finished()) }); __mint1.__close(console); __call }}")
    val __loop5_pass = __Pass_Naturals(naturals(10))
    try {
    while (__loop5_pass.__advance()) {
        val n = __loop5_pass.__current()
        if (n > 12) {
            break
        }
        println(console, "3. natural $n")
    }
    } finally {
        __loop5_pass.__close()
    }
    val doubled = xs.map({ n -> n * 2 }).toMutableList()
    val odd = xs.filter({ n -> n % 2 == 1 }).toMutableList()
    val total = xs.fold(0, { acc, n -> acc + n })
    println(console, "5. list: ${doubled.size} doubled, ${odd.size} odd, total $total")
    val words = listOf<String>("ann", "bo", "carol")
    val lengths = map(iter__2(words), { w -> w.length }, ::next__2)
    println(console, "5. lengths: ${reduce(iter__2(lengths), 0, { acc, n -> acc + n }, ::next__2)}")
    val vowels = filter(iter__3("iteration"), { c -> c == 'i' || c == 'o' }, ::next__5)
    println(console, "5. vowels: ${vowels.size}")
    println(console, "5. fibs total ${run { val __mint2 = __Pass_Fibs(fibs(6)); val __call = reduce(__mint2, 0, { acc, n -> acc + n }, fun(__p: __Pass_Fibs): Union2<Int, Finished> { return if (__p.__advance(console)) U2_1<Int, Finished>(__p.__current()) else U2_2<Int, Finished>(finished()) }); __mint2.__close(console); __call }}")
    val squares = run { val __mint3 = __Pass_Naturals(naturals(1)); val __call = map_lazy(__mint3, { n: Int -> n * n }, fun(__p: __Pass_Naturals): Union2<Int, Finished> { return if (__p.__advance()) U2_1<Int, Finished>(__p.__current()) else U2_2<Int, Finished>(finished()) }); __call }
    val big = filter_lazy(squares, { n: Int -> n > 10 }, ::next__3)
    var __loop6_pass = big
    while (true) {
        val __loop6_step = next__4(__loop6_pass)
        if (__loop6_step !is U2_1<*, *>) { break }
        val n = __loop6_step.value as Int
        println(console, "6. big square $n")
        if (n > 50) {
            break
        }
    }
    val collected = map_to(mutableListOf<Int>(), countdown(3), { n: Int -> n * 10 }, { __i0, __i1 -> __i0.add(__i1) }, ::next__6)
    println(console, "6. collected ${collected.size}")
}
