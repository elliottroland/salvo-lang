package salvo.main

import salvo.*

fun describeContainer(console: salvo.core.console.Console, xs: List<Int>) {
    var sum: Int = 0
    for (n in salvo.platform.core.list.each(xs)) {
        sum = (sum + n)
    }
    salvo.core.console.println(console, "1. list of ${salvo.core.list.sizePlatform(xs)} sums to ${sum}")
    val letters: salvo.platform.core.string.MutStr = salvo.core.string.mutStr(arrayOf<String>())
    for (c in salvo.platform.core.string.each("salvo")) {
        salvo.core.string.appendPlatform(letters, "${c}.")
    }
    salvo.core.console.println(console, "1. string: ${letters.toString()}")
    val arr: Array<Int> = arrayOf<Int>(10, 20, 30)
    var fromArray: Int = 0
    for (n in arr) {
        fromArray = (fromArray + n)
    }
    salvo.core.console.println(console, "1. array sums to ${fromArray}")
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

fun next__Countdown(p: Countdown): Union2<Int, salvo.core.iterator.Finished> {
    if ((p.at <= 0)) {
        return Union2.U2<Int, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val now: Int = p.at
    p.at = (p.at - 1)
    return Union2.U1<Int, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(now))
}

fun skip(p: Countdown): Union2<Int, salvo.core.iterator.Finished> {
    if ((p.at <= 1)) {
        return Union2.U2<Int, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val now: Int = p.at
    p.at = (p.at - 2)
    return Union2.U1<Int, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(now))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun take(console: salvo.core.console.Console, p: Countdown, count: Int) {
    var seen: Int = 0
    while (true) {
        val __step_2: Union2<Int, salvo.core.iterator.Finished> = next__Countdown(p)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: Int = ((__step_2 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_3
                salvo.core.console.println(console, "2. got ${n}")
                seen = (seen + 1)
                if ((seen == count)) {
                    break
                }
            }
            else -> {
                break
            }
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

fun next__Iter_halving_Int(__p: __Iter_halving_Int): Union2<Int, salvo.core.iterator.Finished> {
    if ((__p.at <= 0)) {
        return Union2.U2<Int, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val now: Int = __p.at
    __p.at = (__p.at / 2)
    return Union2.U1<Int, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(now))
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

fun next__Iter_iter_Bag(__p: __Iter_iter_Bag): Union2<Int, salvo.core.iterator.Finished> {
    val e: Int? = salvo.core.list.getPlatform(__p.items, __p.at)
    if ((e == null)) {
        return Union2.U2<Int, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    __p.at = (__p.at + 1)
    val e_1: Int = e!!
    return Union2.U1<Int, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(e_1))
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

fun next__Iter_fibs_Int(console: salvo.core.console.Console, __p: __Iter_fibs_Int): Union2<Int, salvo.core.iterator.Finished> {
    if ((__p.made >= __p.count)) {
        salvo.core.console.println(console, "3. finished")
        return Union2.U2<Int, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val now: Int = __p.a
    val sum: Int = (__p.a + __p.b)
    __p.a = __p.b
    __p.b = sum
    __p.made = (__p.made + 1)
    return Union2.U1<Int, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(now))
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

fun next__Iter_naturals_Int(__p: __Iter_naturals_Int): Union2<Int, salvo.core.iterator.Finished> {
    val now: Int = __p.at
    __p.at = (__p.at + 1)
    return Union2.U1<Int, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(now))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It> sumOf(it: It, next: (It) -> Union2<Int, salvo.core.iterator.Finished>): Int {
    var total: Int = 0
    while (true) {
        val __step_2: Union2<Int, salvo.core.iterator.Finished> = next(it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: Int = ((__step_2 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_3
                total = (total + n)
            }
            else -> {
                break
            }
        }
    }
    return total
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<C, __It0> total(c: C, iter: (C) -> __It0, next: (__It0) -> Union2<Int, salvo.core.iterator.Finished>): Int {
    var total: Int = 0
    val __pass_1: __It0 = iter(c)
    while (true) {
        val __step_2: Union2<Int, salvo.core.iterator.Finished> = next(__pass_1)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: Int = ((__step_2 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_3
                total = (total + n)
            }
            else -> {
                break
            }
        }
    }
    return total
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<__It0> first(it: __It0, next: (__It0) -> Union2<Int, salvo.core.iterator.Finished>): Int {
    while (true) {
        val __step_2: Union2<Int, salvo.core.iterator.Finished> = next(it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: Int = ((__step_2 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_3
                return n
            }
            else -> {
                break
            }
        }
    }
    return (-1)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val xs: List<Int> = listOf<Int>(1, 2, 3, 4)
    describeContainer(__handle_2, xs)
    val p: Countdown = countdown(5)
    take(__handle_2, p, 2)
    salvo.core.console.println(__handle_2, "2. rest sums to ${sumOf(p, ::next__Countdown)}")
    val q: Countdown = countdown(6)
    while (true) {
        val __step_4: Union2<Int, salvo.core.iterator.Finished> = skip(q)
        when {
            (__step_4 is Union2.U1<*, *>) -> {
                val __emitted_5: Int = ((__step_4 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_5
                salvo.core.console.println(__handle_2, "2. skip ${n}")
            }
            else -> {
                break
            }
        }
    }
    val __pass_6: __Iter_halving_Int = halving(20)
    while (true) {
        val __step_7: Union2<Int, salvo.core.iterator.Finished> = next__Iter_halving_Int(__pass_6)
        when {
            (__step_7 is Union2.U1<*, *>) -> {
                val __emitted_8: Int = ((__step_7 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_8
                salvo.core.console.println(__handle_2, "2b. halving ${n}")
            }
            else -> {
                break
            }
        }
    }
    val hp: __Iter_halving_Int = halving(20)
    salvo.core.console.println(__handle_2, "2b. summed from a held iterator: ${sumOf(hp, ::next__Iter_halving_Int)}")
    salvo.core.console.println(__handle_2, "2b. first from a pattern-typed fn: ${first(halvingFromTen(), ::next__Iter_halving_Int)}")
    val bag: Bag = Bag(items = listOf<Int>(7, 8))
    val __pass_9: __Iter_iter_Bag = iter(bag)
    while (true) {
        val __step_10: Union2<Int, salvo.core.iterator.Finished> = next__Iter_iter_Bag(__pass_9)
        when {
            (__step_10 is Union2.U1<*, *>) -> {
                val __emitted_11: Int = ((__step_10 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_11
                salvo.core.console.println(__handle_2, "2c. bag ${n}")
            }
            else -> {
                break
            }
        }
    }
    salvo.core.console.println(__handle_2, "2c. total of a bag ${total(bag, ::iter, ::next__Iter_iter_Bag)}, of a list ${total(xs, { __a0 -> salvo.core.list.iter(__a0) }, { __a0 -> salvo.core.list.next__ListYield(__a0) })}")
    val __pass_12: __Iter_fibs_Int = fibs(6)
    while (true) {
        val __step_13: Union2<Int, salvo.core.iterator.Finished> = next__Iter_fibs_Int(__handle_2, __pass_12)
        when {
            (__step_13 is Union2.U1<*, *>) -> {
                val __emitted_14: Int = ((__step_13 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_14
                salvo.core.console.println(__handle_2, "3. fib ${n}")
            }
            else -> {
                break
            }
        }
    }
    val __pass_15: __Iter_naturals_Int = naturals(10)
    while (true) {
        val __step_16: Union2<Int, salvo.core.iterator.Finished> = next__Iter_naturals_Int(__pass_15)
        when {
            (__step_16 is Union2.U1<*, *>) -> {
                val __emitted_17: Int = ((__step_16 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_17
                if ((n > 12)) {
                    break
                }
                salvo.core.console.println(__handle_2, "3. natural ${n}")
            }
            else -> {
                break
            }
        }
    }
    val doubled: salvo.platform.core.list.MutList<Int> = salvo.core.seq.map__List_Fn(xs, fun(n: Int): Int {
        return (n * 2)
    })
    val odd: salvo.platform.core.list.MutList<Int> = salvo.core.seq.filterPlatform(xs, fun(n: Int): Boolean {
        return ((n % 2) == 1)
    })
    val total: Int = salvo.core.seq.reduce__List_A_Fn(xs, 0, fun(acc: Int, n: Int): Int {
        return (acc + n)
    })
    salvo.core.console.println(__handle_2, "5. list: ${salvo.core.list.sizePlatform(doubled)} doubled, ${salvo.core.list.sizePlatform(odd)} odd, total ${total}")
    val words: List<String> = listOf<String>("ann", "bo", "carol")
    val lengths: salvo.platform.core.list.MutList<Int> = salvo.core.seq.map__It_Fn(salvo.core.list.iter(words), fun(w: String): Int {
        return salvo.core.string.sizePlatform(w)
    }, { __a0 -> salvo.core.list.next__ListYield(__a0) })
    salvo.core.console.println(__handle_2, "5. lengths: ${salvo.core.seq.reduce__It_A_Fn(salvo.core.list.iter(lengths), 0, fun(acc: Int, n: Int): Int {
        return (acc + n)
    }, { __a0 -> salvo.core.list.next__ListYield(__a0) })}")
    val word: String = "iteration"
    val vowels: salvo.platform.core.list.MutList<Char> = salvo.core.seq.filter(salvo.core.string.iter(word), fun(c: Char): Boolean {
        return ((c == 'i') || (c == 'o'))
    }, { __a0 -> salvo.core.string.next(__a0) })
    salvo.core.console.println(__handle_2, "5. vowels: ${salvo.core.list.sizePlatform(vowels)}")
    salvo.core.console.println(__handle_2, "5. halving total ${salvo.core.seq.reduce__It_A_Fn(halving(20), 0, fun(acc: Int, n: Int): Int {
        return (acc + n)
    }, ::next__Iter_halving_Int)}")
    val collected: salvo.platform.core.list.MutList<Int> = salvo.core.seq.mapTo(mutableListOf<Int>(), countdown(3), fun(n: Int): Int {
        return (n * 10)
    }, { __a0, __a1 -> salvo.core.list.addPlatform(__a0, __a1) }, ::next__Countdown)
    salvo.core.console.println(__handle_2, "6. collected ${salvo.core.list.sizePlatform(collected)}")
    val evens: salvo.platform.core.string.MutStr = salvo.core.string.mutStr(arrayOf<String>())
    val __pass_18: salvo.core.range.__Iter_range_Int_Int_Int = salvo.core.range.range__Int_Int_Int(0, 10, 2)
    while (true) {
        val __step_19: Union2<Int, salvo.core.iterator.Finished> = salvo.core.range.next(__pass_18)
        when {
            (__step_19 is Union2.U1<*, *>) -> {
                val __emitted_20: Int = ((__step_19 as Union2.U1<*, *>).value as Int)
                val i: Int = __emitted_20
                salvo.core.string.appendPlatform(evens, "${i} ")
            }
            else -> {
                break
            }
        }
    }
    salvo.core.console.println(__handle_2, "7. evens ${evens.toString()}")
}

