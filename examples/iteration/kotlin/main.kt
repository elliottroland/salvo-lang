package salvo.main

import salvo.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.console.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.range.*
import salvo.core.seq.*
import salvo.core.set.*
import salvo.core.sorted.*
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
    val arr = arrayOf<Int>(10, 20, 30)
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

fun next__10(p: Countdown): Union2<Int, Finished> {
    if (p.at <= 0) {
        return U2_2<Int, Finished>(finished())
    }
    val now = p.at
    p.at = p.at - 1
    return U2_1<Int, Finished>(emitted(now))
}

fun take__2(console: Console, p: Countdown, count: Int) {
    var seen = 0
    while (true) {
        val __loop1_step = next__10(p)
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

fun iter__10(h: Halving): __Pass_Halving {
    return __Pass_Halving(at = h.start)
}

fun next__11(__p: __Pass_Halving): Union2<Int, Finished> {
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

data class __Pass_Fibs(
    var count: Int,
    var a: Int,
    var b: Int,
    var made: Int,
)

fun iter__11(f: Fibs): __Pass_Fibs {
    return __Pass_Fibs(count = f.count, a = 0, b = 1, made = 0)
}

fun next__12(console: Console, __p: __Pass_Fibs): Union2<Int, Finished> {
    if (__p.made >= __p.count) {
        println(console, "3. finished")
        return U2_2<Int, Finished>(finished())
    }
    val now = __p.a
    val sum = __p.a + __p.b
    __p.a = __p.b
    __p.b = sum
    __p.made = __p.made + 1
    return U2_1<Int, Finished>(emitted(now))
}

data class Naturals(
    val from: Int,
)

fun naturals(from: Int): Naturals {
    return Naturals(from = from)
}

data class __Pass_Naturals(
    var at: Int,
)

fun iter__12(n: Naturals): __Pass_Naturals {
    return __Pass_Naturals(at = n.from)
}

fun next__13(__p: __Pass_Naturals): Union2<Int, Finished> {
    val now = __p.at
    __p.at = __p.at + 1
    return U2_1<Int, Finished>(emitted(now))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
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
    take__2(console, p, 2)
    println(console, "2. rest sums to ${sum_of(p, ::next__10)}")
    val h = Halving(start = 20)
    var __loop3_pass = iter__10(h)
    while (true) {
        val __loop3_step = next__11(__loop3_pass)
        if (__loop3_step !is U2_1<Int, Finished>) { break }
        val n = __loop3_step.value
        println(console, "2b. halving $n")
    }
    val hp = iter__10(h)
    println(console, "2b. summed from a held pass: ${sum_of(hp, ::next__11)}")
    var __loop4_pass = iter__11(fibs(6))
    while (true) {
        val __loop4_step = next__12(console, __loop4_pass)
        if (__loop4_step !is U2_1<Int, Finished>) { break }
        val n = __loop4_step.value
        println(console, "3. fib $n")
    }
    var __loop5_pass = iter__12(naturals(10))
    while (true) {
        val __loop5_step = next__13(__loop5_pass)
        if (__loop5_step !is U2_1<Int, Finished>) { break }
        val n = __loop5_step.value
        if (n > 12) {
            break
        }
        println(console, "3. natural $n")
    }
    val doubled = xs.map({ n -> n * 2 }).toMutableList()
    val odd = xs.filter({ n -> n % 2 == 1 }).toMutableList()
    val total = xs.fold(0, { acc, n -> acc + n })
    println(console, "5. list: ${doubled.size} doubled, ${odd.size} odd, total $total")
    val words = listOf<String>("ann", "bo", "carol")
    val lengths = map(iter__3(words), { w -> w.length }, ::next__5)
    println(console, "5. lengths: ${reduce(iter__3(lengths), 0, { acc, n -> acc + n }, ::next__5)}")
    val word = "iteration"
    val vowels = filter(iter__9(word), { c -> c == 'i' || c == 'o' }, ::next__9)
    println(console, "5. vowels: ${vowels.size}")
    println(console, "5. halving total ${reduce(iter__10(Halving(start = 20)), 0, { acc, n -> acc + n }, ::next__11)}")
    val collected = map_to(mutableListOf<Int>(), countdown(3), { n: Int -> n * 10 }, { __i0, __i1 -> __i0.add(__i1) }, ::next__10)
    println(console, "6. collected ${collected.size}")
}
