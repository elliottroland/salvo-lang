package salvo.main

import salvo.*

data class Point(
    val x: Int,
    val y: Int,
)

object __Codec_Point : salvo.WireCodec<Point> {
    override fun enc(v: Point, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.x, out)
        salvo.IntCodec.enc(v.y, out)
    }
    override fun dec(inp: salvo.WireIn): Point = Point(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

data class Note(
    val text: String,
)

object __Codec_Note : salvo.WireCodec<Note> {
    override fun enc(v: Note, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.text, out)
    }
    override fun dec(inp: salvo.WireIn): Note = Note(salvo.StrCodec.dec(inp))
}

fun byLen(a: String, b: String): Int {
    return (salvo.core.string.sizePlatform(a)).compareTo(salvo.core.string.sizePlatform(b))
}

fun countUnique(xs: List<Int>): Int {
    return salvo.core.list.sizePlatform(xs)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val primes: List<Int> = listOf<Int>(2, 3, 5, 7)
    val vowels: salvo.platform.core.set.Set<String> = salvo.core.set.setOfPlatform(arrayOf<String>("a", "e", "i", "o", "u"), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    val ages: salvo.platform.core.map.Map<String, Int> = salvo.core.map.mapOfPlatform(arrayOf<Pair<String, Int>>(Pair("ada", 36), Pair("grace", 45)), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    salvo.core.console.println(__handle_2, "1. list ${salvo.core.list.toStr(primes, { __a0 -> (__a0).toString() })}")
    salvo.core.console.println(__handle_2, "1. set ${salvo.core.set.toStr(vowels, { __a0 -> __a0 })} of ${salvo.core.set.sizePlatform(vowels)}")
    salvo.core.console.println(__handle_2, "1. map ${salvo.core.map.toStr(ages, { __a0 -> __a0 }, { __a0 -> (__a0).toString() }, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })}")
    val note: Note = Note(text = "still a struct literal")
    salvo.core.console.println(__handle_2, "1. struct ${note.text}")
    val seen: salvo.platform.core.set.MutSet<String> = salvo.core.set.mutSetOfPlatform(arrayOf<String>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    salvo.core.set.addPlatform(seen, "first", { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    salvo.core.console.println(__handle_2, "1. empty then filled ${salvo.core.set.toStr(seen, { __a0 -> __a0 })}")
    val tally: salvo.platform.core.map.MutMap<String, Int> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<String, Int>>(Pair("pear", 1), Pair("apple", 2)), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    salvo.core.map.putPlatform(tally, "fig", 3, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    salvo.core.map.putPlatform(tally, "pear", 99, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    salvo.core.console.println(__handle_2, "2. insertion order kept ${salvo.core.map.toStr(tally, { __a0 -> __a0 }, { __a0 -> (__a0).toString() }, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })}")
    val ranked: salvo.platform.core.sorted.MutSortedSet<String> = salvo.core.sorted.mutSortedSetOfPlatform(arrayOf<String>("pear", "apple", "fig"), { __a0, __a1 -> salvo.__salvoCompare(__a0, __a1) })
    salvo.core.console.println(__handle_2, "2. key order ${salvo.core.sorted.toStr__SortedSet(ranked, { __a0 -> __a0 })}")
    val smallest: String? = salvo.core.sorted.minPlatform(ranked)
    if ((smallest != null)) {
        val smallest_3: String = smallest!!
        salvo.core.console.println(__handle_2, "2. min is cheap here ${smallest_3}")
    }
    val corners: salvo.platform.core.set.MutSet<Point> = salvo.core.set.mutSetOfPlatform(arrayOf<Point>(), ::hash, ::eq__Point_Point)
    salvo.core.set.addPlatform(corners, Point(x = 0, y = 0), ::hash, ::eq__Point_Point)
    val again: Boolean = salvo.core.set.addPlatform(corners, Point(x = 0, y = 0), ::hash, ::eq__Point_Point)
    salvo.core.console.println(__handle_2, "3. struct key: size ${salvo.core.set.sizePlatform(corners)}, second add ${again}")
    val labels: salvo.platform.core.map.MutMap<Point, String> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<Point, String>>(), ::hash, ::eq__Point_Point)
    salvo.core.map.putPlatform(labels, Point(x = 1, y = 1), "diagonal", ::hash, ::eq__Point_Point)
    val found: String? = salvo.core.map.getPlatform(labels, Point(x = 1, y = 1), ::hash, ::eq__Point_Point)
    if ((found != null)) {
        val found_4: String = found!!
        salvo.core.console.println(__handle_2, "3. looked up by value ${found_4}")
    }
    val a: Point = Point(x = 1, y = 2)
    val b: Point = Point(x = 1, y = 2)
    val c: Point = Point(x = 1, y = 9)
    val same: Boolean = eq__Point_Point(a, b)
    val before: Boolean = (cmp(a, c) < 0)
    salvo.core.console.println(__handle_2, "4. equal ${same}, ordered ${before}")
    val n1: Note = Note(text = "same")
    val n2: Note = Note(text = "same")
    val notesEqual: Boolean = eq__Note_Note(n1, n2)
    salvo.core.console.println(__handle_2, "4. plain struct equality ${notesEqual}")
    val squares: List<Int> = salvo.core.list.listBy(4, fun(i: Int): Int {
        return (i * i)
    })
    salvo.core.console.println(__handle_2, "5. generated ${salvo.core.list.toStr(squares, { __a0 -> (__a0).toString() })}")
    val deduped: salvo.platform.core.set.Set<Int> = salvo.core.set.toSetPlatform(primes, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    salvo.core.console.println(__handle_2, "5. to_set ${salvo.core.set.toStr(deduped, { __a0 -> (__a0).toString() })}")
    val words: List<String> = listOf<String>("alpha", "be")
    val lengths: salvo.platform.core.map.Map<String, Int> = salvo.core.map.toMap(words, fun(w: String): Pair<String, Int> {
        return Pair(w, salvo.core.string.sizePlatform(w))
    }, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    salvo.core.console.println(__handle_2, "5. to_map with a rule ${salvo.core.map.toStr(lengths, { __a0 -> __a0 }, { __a0 -> (__a0).toString() }, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })}")
    val filled: List<String> = listOf<String>("ada", "grace")
    salvo.core.console.println(__handle_2, "6. first is ${salvo.core.list.first(filled)}, no optional")
    val growing: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    salvo.core.list.addPlatform(growing, 7)
    salvo.core.console.println(__handle_2, "6. after add, first is ${salvo.core.list.first(growing)}")
    val ordered: List<Int> = salvo.core.list.sort(listOf<Int>(40, 10, 30, 20), { __a0, __a1 -> (__a0).compareTo(__a1) })
    salvo.core.console.println(__handle_2, "6. sorted ${salvo.core.list.toStr(ordered, { __a0 -> (__a0).toString() })}")
    val __subject_5: Int? = salvo.core.list.binarySearch(ordered, 30, { __a0, __a1 -> (__a0).compareTo(__a1) })
    if ((__subject_5 != null)) {
        val at: Int = __subject_5!!
        salvo.core.console.println(__handle_2, "6. found 30 at ${at}")
    }
    val live: salvo.platform.core.list.MutList<Int> = salvo.core.list.mutSort(listOf<Int>(10, 30), { __a0, __a1 -> (__a0).compareTo(__a1) })
    salvo.core.list.addSorted(live, 20, { __a0, __a1 -> (__a0).compareTo(__a1) })
    salvo.core.list.addSorted(live, 5, { __a0, __a1 -> (__a0).compareTo(__a1) })
    salvo.core.console.println(__handle_2, "6. still sorted ${salvo.core.list.toStr(live, { __a0 -> (__a0).toString() })}")
    val bylen: List<String> = salvo.core.list.sort(listOf<String>("alpha", "be", "z"), ::byLen)
    salvo.core.console.println(__handle_2, "6. by length ${salvo.core.list.toStr(bylen, { __a0 -> __a0 })}")
    val __subject_6: Int? = salvo.core.list.binarySearch(bylen, "hi", ::byLen)
    if ((__subject_6 != null)) {
        val atLen: Int = __subject_6!!
        salvo.core.console.println(__handle_2, "6. a two-letter word at ${atLen}")
    }
    val unique: List<Int> = salvo.core.set.toListPlatform(deduped)
    salvo.core.console.println(__handle_2, "6. distinct ${salvo.core.list.toStr(unique, { __a0 -> (__a0).toString() })} of ${countUnique(unique)}")
    val __pass_7: salvo.core.set.SetYield<String> = salvo.core.set.iter(vowels)
    while (true) {
        val __step_8: Union2<String, salvo.core.iterator.Finished> = salvo.core.set.next(__pass_7)
        when {
            (__step_8 is Union2.U1<*, *>) -> {
                val __emitted_9: String = ((__step_8 as Union2.U1<*, *>).value as String)
                val v: String = __emitted_9
                __handle_2.print(v)
            }
            else -> {
                break
            }
        }
    }
    salvo.core.console.println(__handle_2, "")
    val __pass_10: salvo.core.map.MapKeyYield<String> = salvo.core.map.iter(ages)
    while (true) {
        val __step_11: Union2<String, salvo.core.iterator.Finished> = salvo.core.map.next__MapKeyYield(__pass_10)
        when {
            (__step_11 is Union2.U1<*, *>) -> {
                val __emitted_12: String = ((__step_11 as Union2.U1<*, *>).value as String)
                val name: String = __emitted_12
                val age: Int? = salvo.core.map.getPlatform(ages, name, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
                if ((age != null)) {
                    val age_13: Int = age!!
                    salvo.core.console.println(__handle_2, "7. ${name} is ${age_13}")
                }
            }
            else -> {
                break
            }
        }
    }
}

fun cmp(a: Point, b: Point): Int {
    val c__c1: Int = (a.x).compareTo(b.x)
    if (!(((c__c1) == (0)))) {
        return c__c1
    }
    val c__c2: Int = (a.y).compareTo(b.y)
    if (!(((c__c2) == (0)))) {
        return c__c2
    }
    return 0
}

fun hash(value: Point): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, (value.x).hashCode().toLong())
    h = salvo.core.compare.mixHash(h, (value.y).hashCode().toLong())
    return h
}

fun eq__Point_Point(a: Point, b: Point): Boolean {
    if (!(((a.x) == (b.x)))) {
        return false
    }
    if (!(((a.y) == (b.y)))) {
        return false
    }
    return true
}

fun eq__Note_Note(a: Note, b: Note): Boolean {
    if (!(((a.text) == (b.text)))) {
        return false
    }
    return true
}

