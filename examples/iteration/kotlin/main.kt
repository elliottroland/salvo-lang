package salvo.main

import salvo.*
import salvo.core.array.next
import salvo.core.console.Console
import salvo.core.console.__Platform_StdOutConsole
import salvo.core.console.println
import salvo.core.iterator.Finished
import salvo.core.iterator.emitted
import salvo.core.iterator.finished
import salvo.core.list.ListYield
import salvo.core.list.addPlatform
import salvo.core.list.count
import salvo.core.list.get
import salvo.core.list.getPlatform
import salvo.core.list.iter as iter__core_list
import salvo.core.list.next__ListYield
import salvo.core.list.sizePlatform
import salvo.core.list.sizePlatform as sizePlatform__core_list
import salvo.core.map.get
import salvo.core.map.sizePlatform
import salvo.core.range.__Iter_range_Int_Int_Int
import salvo.core.range.next
import salvo.core.range.next as next__core_range
import salvo.core.range.range__Int_Int_Int
import salvo.core.seq.count
import salvo.core.seq.filter
import salvo.core.seq.filterPlatform
import salvo.core.seq.mapTo
import salvo.core.seq.map__It_Fn
import salvo.core.seq.map__List_Fn
import salvo.core.seq.reduce__It_A_Fn
import salvo.core.seq.reduce__List_A_Fn
import salvo.core.set.next
import salvo.core.set.sizePlatform
import salvo.core.string.StrYield
import salvo.core.string.appendPlatform
import salvo.core.string.iter as iter__core_string
import salvo.core.string.mutStr
import salvo.core.string.next
import salvo.core.string.next as next__core_string
import salvo.core.string.sizePlatform
import salvo.core.string.sizePlatform as sizePlatform__core_string

fun describeContainer(console: Console, xs: List<Int>) {
    var sum = 0
    for (n in salvo.platform.core.list.each(xs)) {
        sum = sum + n
    }
    println(console, "1. list of ${sizePlatform__core_list(xs)} sums to $sum")
    val letters = mutStr(arrayOf())
    for (c in salvo.platform.core.string.each("salvo")) {
        appendPlatform(letters, "$c.")
    }
    println(console, "1. string: ${letters.toString()}")
    val arr = arrayOf<Int>(10, 20, 30)
    var fromArray = 0
    for (n in arr) {
        fromArray = fromArray + n
    }
    println(console, "1. array sums to $fromArray")
}

data class Countdown(
    var at: Int,
)

object __Codec_Countdown : salvo.WireCodec<Countdown> {
    override fun enc(v: Countdown, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): Countdown = Countdown(salvo.IntCodec.dec(inp))
}

fun countdown(from: Int): Countdown {
    return Countdown(at = from)
}

fun next__Countdown(p: Countdown): Union2<Int, Finished> {
    if (p.at <= 0) {
        return Union2.U2<Int, Finished>(finished())
    }
    val now = p.at
    p.at = p.at - 1
    return Union2.U1<Int, Finished>(emitted(now))
}

fun skip(p: Countdown): Union2<Int, Finished> {
    if (p.at <= 1) {
        return Union2.U2<Int, Finished>(finished())
    }
    val now = p.at
    p.at = p.at - 2
    return Union2.U1<Int, Finished>(emitted(now))
}

fun take(console: Console, p: Countdown, count: Int) {
    var seen = 0
    while (true) {
        val __loop1_step = next__Countdown(p)
        if (__loop1_step !is Union2.U1<Int, Finished>) { break }
        val n = __loop1_step.value
        println(console, "2. got $n")
        seen = seen + 1
        if (seen == count) {
            break
        }
    }
}

data class __Iter_halving_Int(
    var start: Int,
    var at: Int,
)

object __Codec___Iter_halving_Int : salvo.WireCodec<__Iter_halving_Int> {
    override fun enc(v: __Iter_halving_Int, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.start, out)
        salvo.IntCodec.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): __Iter_halving_Int = __Iter_halving_Int(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun halving(start: Int): __Iter_halving_Int {
    return __Iter_halving_Int(start = start, at = start)
}

fun next__Iter_halving_Int(__p: __Iter_halving_Int): Union2<Int, Finished> {
    if (__p.at <= 0) {
        return Union2.U2<Int, Finished>(finished())
    }
    val now = __p.at
    __p.at = __p.at / 2
    return Union2.U1<Int, Finished>(emitted(now))
}

fun halvingFromTen(): __Iter_halving_Int {
    return halving(10)
}

data class Bag(
    val items: List<Int>,
)

object __Codec_Bag : salvo.WireCodec<Bag> {
    override fun enc(v: Bag, out: salvo.WireOut) {
        salvo.ListCodec(salvo.IntCodec).enc(v.items, out)
    }
    override fun dec(inp: salvo.WireIn): Bag = Bag(salvo.ListCodec(salvo.IntCodec).dec(inp))
}

data class __Iter_iter_Bag(
    var items: List<Int>,
    var at: Int,
)

fun iter(bag: Bag): __Iter_iter_Bag {
    return __Iter_iter_Bag(items = bag.items, at = 0)
}

fun next__Iter_iter_Bag(__p: __Iter_iter_Bag): Union2<Int, Finished> {
    val e = getPlatform(__p.items, __p.at)
    if (e == null) {
        return Union2.U2<Int, Finished>(finished())
    }
    __p.at = __p.at + 1
    return Union2.U1<Int, Finished>(emitted(e))
}

data class __Iter_fibs_Int(
    var count: Int,
    var a: Int,
    var b: Int,
    var made: Int,
)

object __Codec___Iter_fibs_Int : salvo.WireCodec<__Iter_fibs_Int> {
    override fun enc(v: __Iter_fibs_Int, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.count, out)
        salvo.IntCodec.enc(v.a, out)
        salvo.IntCodec.enc(v.b, out)
        salvo.IntCodec.enc(v.made, out)
    }
    override fun dec(inp: salvo.WireIn): __Iter_fibs_Int = __Iter_fibs_Int(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun fibs(count: Int): __Iter_fibs_Int {
    return __Iter_fibs_Int(count = count, a = 0, b = 1, made = 0)
}

fun next__Iter_fibs_Int(console: Console, __p: __Iter_fibs_Int): Union2<Int, Finished> {
    if (__p.made >= __p.count) {
        println(console, "3. finished")
        return Union2.U2<Int, Finished>(finished())
    }
    val now = __p.a
    val sum = __p.a + __p.b
    __p.a = __p.b
    __p.b = sum
    __p.made = __p.made + 1
    return Union2.U1<Int, Finished>(emitted(now))
}

data class __Iter_naturals_Int(
    var from: Int,
    var at: Int,
)

object __Codec___Iter_naturals_Int : salvo.WireCodec<__Iter_naturals_Int> {
    override fun enc(v: __Iter_naturals_Int, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.from, out)
        salvo.IntCodec.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): __Iter_naturals_Int = __Iter_naturals_Int(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun naturals(from: Int): __Iter_naturals_Int {
    return __Iter_naturals_Int(from = from, at = from)
}

fun next__Iter_naturals_Int(__p: __Iter_naturals_Int): Union2<Int, Finished> {
    val now = __p.at
    __p.at = __p.at + 1
    return Union2.U1<Int, Finished>(emitted(now))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It> sumOf(it: It, next: (It) -> Union2<Int, Finished>): Int {
    var total = 0
    while (true) {
        val __loop2_step = next(it)
        if (__loop2_step !is Union2.U1<*, *>) { break }
        val n = __loop2_step.value as Int
        total = total + n
    }
    return total
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<C, __It0> total(c: C, iter: (C) -> __It0, next: (__It0) -> Union2<Int, Finished>): Int {
    var total = 0
    var __loop3_pass = iter(c)
    while (true) {
        val __loop3_step = next(__loop3_pass)
        if (__loop3_step !is Union2.U1<*, *>) { break }
        val n = __loop3_step.value as Int
        total = total + n
    }
    return total
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<__It0> first(it: __It0, next: (__It0) -> Union2<Int, Finished>): Int {
    while (true) {
        val __loop4_step = next(it)
        if (__loop4_step !is Union2.U1<*, *>) { break }
        val n = __loop4_step.value as Int
        return n
    }
    return -1
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val xs = listOf<Int>(1, 2, 3, 4)
    describeContainer(console, xs)
    val p = countdown(5)
    take(console, p, 2)
    println(console, "2. rest sums to ${sumOf(p, ::next__Countdown)}")
    val q = countdown(6)
    while (true) {
        val __loop5_step = skip(q)
        if (__loop5_step !is Union2.U1<*, *>) { break }
        val n = __loop5_step.value as Int
        println(console, "2. skip $n")
    }
    var __loop6_pass = halving(20)
    while (true) {
        val __loop6_step = next__Iter_halving_Int(__loop6_pass)
        if (__loop6_step !is Union2.U1<Int, Finished>) { break }
        val n = __loop6_step.value
        println(console, "2b. halving $n")
    }
    val hp = halving(20)
    println(console, "2b. summed from a held iterator: ${sumOf(hp, ::next__Iter_halving_Int)}")
    println(console, "2b. first from a pattern-typed fn: ${first(halvingFromTen(), ::next__Iter_halving_Int)}")
    val bag = Bag(items = listOf<Int>(7, 8))
    var __loop7_pass = iter(bag)
    while (true) {
        val __loop7_step = next__Iter_iter_Bag(__loop7_pass)
        if (__loop7_step !is Union2.U1<Int, Finished>) { break }
        val n = __loop7_step.value
        println(console, "2c. bag $n")
    }
    println(console, "2c. total of a bag ${total(bag, ::iter, ::next__Iter_iter_Bag)}, of a list ${total(xs, ::iter__core_list, ::next__ListYield)}")
    var __loop8_pass = fibs(6)
    while (true) {
        val __loop8_step = next__Iter_fibs_Int(console, __loop8_pass)
        if (__loop8_step !is Union2.U1<Int, Finished>) { break }
        val n = __loop8_step.value
        println(console, "3. fib $n")
    }
    var __loop9_pass = naturals(10)
    while (true) {
        val __loop9_step = next__Iter_naturals_Int(__loop9_pass)
        if (__loop9_step !is Union2.U1<Int, Finished>) { break }
        val n = __loop9_step.value
        if (n > 12) {
            break
        }
        println(console, "3. natural $n")
    }
    val doubled = map__List_Fn(xs, { n -> n * 2 })
    val odd = filterPlatform(xs, { n -> n % 2 == 1 })
    val total = reduce__List_A_Fn(xs, 0, { acc, n -> acc + n })
    println(console, "5. list: ${sizePlatform__core_list(doubled)} doubled, ${sizePlatform__core_list(odd)} odd, total $total")
    val words = listOf<String>("ann", "bo", "carol")
    val lengths = map__It_Fn(iter__core_list(words), { w -> sizePlatform__core_string(w) }, ::next__ListYield)
    println(console, "5. lengths: ${reduce__It_A_Fn(iter__core_list(lengths), 0, { acc, n -> acc + n }, ::next__ListYield)}")
    val word = "iteration"
    val vowels = filter(iter__core_string(word), { c -> c == 'i' || c == 'o' }, ::next__core_string)
    println(console, "5. vowels: ${sizePlatform__core_list(vowels)}")
    println(console, "5. halving total ${reduce__It_A_Fn(halving(20), 0, { acc, n -> acc + n }, ::next__Iter_halving_Int)}")
    val collected = mapTo(mutableListOf<Int>(), countdown(3), { n: Int -> n * 10 }, ::addPlatform, ::next__Countdown)
    println(console, "6. collected ${sizePlatform__core_list(collected)}")
    val evens = mutStr(arrayOf())
    var __loop10_pass = range__Int_Int_Int(0, 10, 2)
    while (true) {
        val __loop10_step = next__core_range(__loop10_pass)
        if (__loop10_step !is Union2.U1<Int, Finished>) { break }
        val i = __loop10_step.value
        appendPlatform(evens, "$i ")
    }
    println(console, "7. evens ${evens.toString()}")
}
