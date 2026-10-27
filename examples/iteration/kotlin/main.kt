package salvo.main

import salvo.*

fun describeContainer(console: salvo.std.core.console.Console, xs: List<Int>) {
    var sum: Int = 0
    for (n in salvo.platform.std.core.list.each(xs)) {
        sum = (sum + n)
    }
    salvo.std.core.console.println(console, "1. list of ${salvo.std.core.list.sizePlatform(xs)} sums to ${sum}")
    val letters: salvo.platform.std.core.string.MutStr = salvo.std.core.string.Mut_Str(arrayOf<String>())
    for (c in salvo.platform.std.core.string.each("salvo")) {
        salvo.std.core.string.appendPlatform(letters, "${c}.")
    }
    salvo.std.core.console.println(console, "1. string: ${letters.toString()}")
    val arr: Array<Int> = arrayOf<Int>(10, 20, 30)
    var fromArray: Int = 0
    for (n in arr) {
        fromArray = (fromArray + n)
    }
    salvo.std.core.console.println(console, "1. array sums to ${fromArray}")
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

fun next__Countdown(p: Countdown): Union2<Int, salvo.std.core.iterator.Finished> {
    if ((p.at <= 0)) {
        return Union2.U2<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val now: Int = p.at
    p.at = (p.at - 1)
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(now))
}

fun skip(p: Countdown): Union2<Int, salvo.std.core.iterator.Finished> {
    if ((p.at <= 1)) {
        return Union2.U2<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val now: Int = p.at
    p.at = (p.at - 2)
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(now))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun take(console: salvo.std.core.console.Console, p: Countdown, count: Int) {
    var seen: Int = 0
    while (true) {
        val __step_2: Union2<Int, salvo.std.core.iterator.Finished> = salvo.main.next__Countdown(p)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: Int = ((__step_2 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_3
                salvo.std.core.console.println(console, "2. got ${n}")
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

data class Halving(
    var start: Int,
    var at: Int,
)

object __Codec_Halving : salvo.WireCodec<Halving> {
    override fun enc(v: Halving, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.start, out)
        salvo.IntCodec.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): Halving = Halving(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun halving(start: Int): Halving {
    return Halving(start = start, at = start)
}

fun next__Halving(__p: Halving): Union2<Int, salvo.std.core.iterator.Finished> {
    if ((__p.at <= 0)) {
        return Union2.U2<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val now: Int = __p.at
    __p.at = (__p.at / 2)
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(now))
}

fun halvingFromTen(): Halving {
    return salvo.main.halving(10)
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

data class BagIter(
    var bag: Bag,
    var at: Int,
)

fun iter(bag: Bag): BagIter {
    return BagIter(bag = bag, at = 0)
}

fun next__BagIter(__p: BagIter): Union2<Int, salvo.std.core.iterator.Finished> {
    val e: Int? = salvo.std.core.list.getPlatform(__p.bag.items, __p.at)
    if ((e == null)) {
        return Union2.U2<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    __p.at = (__p.at + 1)
    val e_1: Int = e!!
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(e_1))
}

data class Fibs(
    var count: Int,
    var a: Int,
    var b: Int,
    var made: Int,
)

object __Codec_Fibs : salvo.WireCodec<Fibs> {
    override fun enc(v: Fibs, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.count, out)
        salvo.IntCodec.enc(v.a, out)
        salvo.IntCodec.enc(v.b, out)
        salvo.IntCodec.enc(v.made, out)
    }
    override fun dec(inp: salvo.WireIn): Fibs = Fibs(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun fibs(count: Int): Fibs {
    return Fibs(count = count, a = 0, b = 1, made = 0)
}

fun next__Fibs(console: salvo.std.core.console.Console, __p: Fibs): Union2<Int, salvo.std.core.iterator.Finished> {
    if ((__p.made >= __p.count)) {
        salvo.std.core.console.println(console, "3. finished")
        return Union2.U2<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val now: Int = __p.a
    val sum: Int = (__p.a + __p.b)
    __p.a = __p.b
    __p.b = sum
    __p.made = (__p.made + 1)
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(now))
}

data class Naturals(
    var from: Int,
    var at: Int,
)

object __Codec_Naturals : salvo.WireCodec<Naturals> {
    override fun enc(v: Naturals, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.from, out)
        salvo.IntCodec.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): Naturals = Naturals(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun naturals(from: Int): Naturals {
    return Naturals(from = from, at = from)
}

fun next__Naturals(__p: Naturals): Union2<Int, salvo.std.core.iterator.Finished> {
    val now: Int = __p.at
    __p.at = (__p.at + 1)
    return Union2.U1<Int, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(now))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It> sumOf(it: It, next: (It) -> Union2<Int, salvo.std.core.iterator.Finished>): Int {
    var total: Int = 0
    while (true) {
        val __step_2: Union2<Int, salvo.std.core.iterator.Finished> = next(it)
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
fun<C, It> total(c: C, iter: (C) -> It, next: (It) -> Union2<Int, salvo.std.core.iterator.Finished>): Int {
    var total: Int = 0
    val __pass_1: It = iter(c)
    while (true) {
        val __step_2: Union2<Int, salvo.std.core.iterator.Finished> = next(__pass_1)
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
fun<It> first(it: It, next: (It) -> Union2<Int, salvo.std.core.iterator.Finished>): Int {
    while (true) {
        val __step_2: Union2<Int, salvo.std.core.iterator.Finished> = next(it)
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
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val xs: List<Int> = listOf<Int>(1, 2, 3, 4)
    salvo.main.describeContainer(__handle_2, xs)
    val p: Countdown = salvo.main.countdown(5)
    salvo.main.take(__handle_2, p, 2)
    salvo.std.core.console.println(__handle_2, "2. rest sums to ${salvo.main.sumOf(p, ::next__Countdown)}")
    val q: Countdown = salvo.main.countdown(6)
    while (true) {
        val __step_4: Union2<Int, salvo.std.core.iterator.Finished> = salvo.main.skip(q)
        when {
            (__step_4 is Union2.U1<*, *>) -> {
                val __emitted_5: Int = ((__step_4 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_5
                salvo.std.core.console.println(__handle_2, "2. skip ${n}")
            }
            else -> {
                break
            }
        }
    }
    val __pass_6: Halving = salvo.main.halving(20)
    while (true) {
        val __step_7: Union2<Int, salvo.std.core.iterator.Finished> = salvo.main.next__Halving(__pass_6)
        when {
            (__step_7 is Union2.U1<*, *>) -> {
                val __emitted_8: Int = ((__step_7 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_8
                salvo.std.core.console.println(__handle_2, "2b. halving ${n}")
            }
            else -> {
                break
            }
        }
    }
    val hp: Halving = salvo.main.halving(20)
    salvo.std.core.console.println(__handle_2, "2b. summed from a held iterator: ${salvo.main.sumOf(hp, ::next__Halving)}")
    salvo.std.core.console.println(__handle_2, "2b. first from a fn returning one: ${salvo.main.first(salvo.main.halvingFromTen(), ::next__Halving)}")
    val bag: Bag = Bag(items = listOf<Int>(7, 8))
    val __pass_9: BagIter = salvo.main.iter(bag)
    while (true) {
        val __step_10: Union2<Int, salvo.std.core.iterator.Finished> = salvo.main.next__BagIter(__pass_9)
        when {
            (__step_10 is Union2.U1<*, *>) -> {
                val __emitted_11: Int = ((__step_10 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_11
                salvo.std.core.console.println(__handle_2, "2c. bag ${n}")
            }
            else -> {
                break
            }
        }
    }
    salvo.std.core.console.println(__handle_2, "2c. total of a bag ${salvo.main.total(bag, ::iter, ::next__BagIter)}, of a list ${salvo.main.total(xs, { __a0 -> salvo.std.core.list.iter(__a0) }, { __a0 -> salvo.std.core.list.next__ListYield(__a0) })}")
    val __pass_12: Fibs = salvo.main.fibs(6)
    while (true) {
        val __step_13: Union2<Int, salvo.std.core.iterator.Finished> = salvo.main.next__Fibs(__handle_2, __pass_12)
        when {
            (__step_13 is Union2.U1<*, *>) -> {
                val __emitted_14: Int = ((__step_13 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_14
                salvo.std.core.console.println(__handle_2, "3. fib ${n}")
            }
            else -> {
                break
            }
        }
    }
    val __pass_15: Naturals = salvo.main.naturals(10)
    while (true) {
        val __step_16: Union2<Int, salvo.std.core.iterator.Finished> = salvo.main.next__Naturals(__pass_15)
        when {
            (__step_16 is Union2.U1<*, *>) -> {
                val __emitted_17: Int = ((__step_16 as Union2.U1<*, *>).value as Int)
                val n: Int = __emitted_17
                if ((n > 12)) {
                    break
                }
                salvo.std.core.console.println(__handle_2, "3. natural ${n}")
            }
            else -> {
                break
            }
        }
    }
    val doubled: salvo.platform.std.core.list.MutList<Int> = salvo.std.core.seq.map__List_Fn(xs, fun(n: Int): Int {
        return (n * 2)
    })
    val odd: salvo.platform.std.core.list.MutList<Int> = salvo.std.core.seq.filterPlatform(xs, fun(n: Int): Boolean {
        return ((n % 2) == 1)
    })
    val total: Int = salvo.std.core.seq.reduce__List_A_Fn(xs, 0, fun(acc: Int, n: Int): Int {
        return (acc + n)
    })
    salvo.std.core.console.println(__handle_2, "5. list: ${salvo.std.core.list.sizePlatform(doubled)} doubled, ${salvo.std.core.list.sizePlatform(odd)} odd, total ${total}")
    val words: List<String> = listOf<String>("ann", "bo", "carol")
    val lengths: salvo.platform.std.core.list.MutList<Int> = salvo.std.core.seq.map__It_Fn(salvo.std.core.list.iter(words), fun(w: String): Int {
        return salvo.std.core.string.sizePlatform(w)
    }, { __a0 -> salvo.std.core.list.next__ListYield(__a0) })
    salvo.std.core.console.println(__handle_2, "5. lengths: ${salvo.std.core.seq.reduce__It_A_Fn(salvo.std.core.list.iter(lengths), 0, fun(acc: Int, n: Int): Int {
        return (acc + n)
    }, { __a0 -> salvo.std.core.list.next__ListYield(__a0) })}")
    val word: String = "iteration"
    val vowels: salvo.platform.std.core.list.MutList<Char> = salvo.std.core.seq.filter(salvo.std.core.string.iter(word), fun(c: Char): Boolean {
        return ((c == 'i') || (c == 'o'))
    }, { __a0 -> salvo.std.core.string.next(__a0) })
    salvo.std.core.console.println(__handle_2, "5. vowels: ${salvo.std.core.list.sizePlatform(vowels)}")
    salvo.std.core.console.println(__handle_2, "5. halving total ${salvo.std.core.seq.reduce__It_A_Fn(salvo.main.halving(20), 0, fun(acc: Int, n: Int): Int {
        return (acc + n)
    }, ::next__Halving)}")
    val collected: salvo.platform.std.core.list.MutList<Int> = salvo.std.core.seq.mapTo(mutableListOf<Int>(), salvo.main.countdown(3), fun(n: Int): Int {
        return (n * 10)
    }, { __a0, __a1 -> salvo.std.core.list.addPlatform(__a0, __a1) }, ::next__Countdown)
    salvo.std.core.console.println(__handle_2, "6. collected ${salvo.std.core.list.sizePlatform(collected)}")
    val evens: salvo.platform.std.core.string.MutStr = salvo.std.core.string.Mut_Str(arrayOf<String>())
    val __pass_18: salvo.std.core.range.Range = salvo.std.core.range.range__Int_Int_Int(0, 10, 2)
    while (true) {
        val __step_19: Union2<Int, salvo.std.core.iterator.Finished> = salvo.std.core.range.next(__pass_18)
        when {
            (__step_19 is Union2.U1<*, *>) -> {
                val __emitted_20: Int = ((__step_19 as Union2.U1<*, *>).value as Int)
                val i: Int = __emitted_20
                salvo.std.core.string.appendPlatform(evens, "${i} ")
            }
            else -> {
                break
            }
        }
    }
    salvo.std.core.console.println(__handle_2, "7. evens ${evens.toString()}")
}

