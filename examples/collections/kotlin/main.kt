package salvo.main

import salvo.*
import salvo.core.console.Console
import salvo.core.console.println
import salvo.core.list.addPlatform as addPlatform__core_list
import salvo.core.list.addSorted
import salvo.core.list.at
import salvo.core.list.binarySearch
import salvo.core.list.first
import salvo.core.list.listBy
import salvo.core.list.mutSort
import salvo.core.list.sizePlatform as sizePlatform__core_list
import salvo.core.list.sort
import salvo.core.list.toStr as toStr__core_list
import salvo.core.map.getPlatform
import salvo.core.map.iter as iter__core_map
import salvo.core.map.next as next__core_map
import salvo.core.map.putPlatform
import salvo.core.set.addPlatform as addPlatform__core_set
import salvo.core.set.iter as iter__core_set
import salvo.core.set.next as next__core_set
import salvo.core.set.sizePlatform as sizePlatform__core_set
import salvo.core.set.toListPlatform
import salvo.core.set.toStr as toStr__core_set
import salvo.core.string.sizePlatform as sizePlatform__core_string

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
    return (sizePlatform__core_string(a)).compareTo(sizePlatform__core_string(b))
}

fun countUnique(xs: List<Int>): Int {
    return sizePlatform__core_list(xs)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val primes = listOf<Int>(2, 3, 5, 7)
    val vowels = linkedSetOf<String>("a", "e", "i", "o", "u")
    val ages = linkedMapOf<String, Int>(("ada" to 36), ("grace" to 45))
    println(console, "1. list ${toStr__core_list(primes, { __i0 -> (__i0).toString() })}")
    println(console, "1. set ${toStr__core_set(vowels, { __i0 -> __i0 })} of ${sizePlatform__core_set(vowels)}")
    println(console, "1. map ${ages.entries.joinToString(", ", "{", "}") { "${it.key}: ${it.value}" }}")
    val note: Note = Note(text = "still a struct literal")
    println(console, "1. struct ${note.text}")
    val seen: salvo.platform.core.set.MutSet<String> = linkedSetOf<String>()
    addPlatform__core_set(seen, "first")
    println(console, "1. empty then filled ${toStr__core_set(seen, { __i0 -> __i0 })}")
    val tally: salvo.platform.core.map.MutMap<String, Int> = linkedMapOf<String, Int>().also { __m -> __m.putAll(listOf(Pair("pear", 1), Pair("apple", 2))) }
    putPlatform(tally, "fig", 3)
    putPlatform(tally, "pear", 99)
    println(console, "2. insertion order kept ${tally.entries.joinToString(", ", "{", "}") { "${it.key}: ${it.value}" }}")
    val ranked: java.util.SortedSet<String> = java.util.TreeSet<String>(salvo.SalvoCanonicalOrder).also { __s -> __s.addAll(listOf("pear", "apple", "fig")) }
    println(console, "2. key order ${ranked.joinToString(", ", "{", "}")}")
    val smallest = ranked.firstOrNull()
    if (smallest != null) {
        println(console, "2. min is cheap here $smallest")
    }
    val corners: salvo.platform.core.set.MutSet<Point> = linkedSetOf<Point>()
    addPlatform__core_set(corners, Point(x = 0, y = 0))
    val again = addPlatform__core_set(corners, Point(x = 0, y = 0))
    println(console, "3. struct key: size ${sizePlatform__core_set(corners)}, second add $again")
    val labels: salvo.platform.core.map.MutMap<Point, String> = linkedMapOf<Point, String>()
    putPlatform(labels, Point(x = 1, y = 1), "diagonal")
    val found = getPlatform(labels, Point(x = 1, y = 1))
    if (found != null) {
        println(console, "3. looked up by value $found")
    }
    val a = Point(x = 1, y = 2)
    val b = Point(x = 1, y = 2)
    val c = Point(x = 1, y = 9)
    val same = eq__Point_Point(a, b)
    val before = cmp(a, c) < 0
    println(console, "4. equal $same, ordered $before")
    val n1 = Note(text = "same")
    val n2 = Note(text = "same")
    val notesEqual = eq__Note_Note(n1, n2)
    println(console, "4. plain struct equality $notesEqual")
    val squares = listBy(4, { i -> i * i })
    println(console, "5. generated ${toStr__core_list(squares, { __i0 -> (__i0).toString() })}")
    val deduped = linkedSetOf<Int>().also { __s -> __s.addAll(primes) }
    println(console, "5. to_set ${toStr__core_set(deduped, { __i0 -> (__i0).toString() })}")
    val words = listOf<String>("alpha", "be")
    val lengths = linkedMapOf<String, Int>().also { __m -> words.map({ w -> Pair(w, sizePlatform__core_string(w)) }).forEach { __e -> __m.put(__e.first, __e.second) } }
    println(console, "5. to_map with a rule ${lengths.entries.joinToString(", ", "{", "}") { "${it.key}: ${it.value}" }}")
    val filled = listOf<String>("ada", "grace")
    println(console, "6. first is ${first(filled)}, no optional")
    val growing: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    addPlatform__core_list(growing, 7)
    println(console, "6. after add, first is ${first(growing)}")
    val ordered = sort(listOf<Int>(40, 10, 30, 20), { __i0, __i1 -> (__i0).compareTo(__i1) })
    println(console, "6. sorted ${toStr__core_list(ordered, { __i0 -> (__i0).toString() })}")
    var __is1 = binarySearch(ordered, 30, { __i0, __i1 -> (__i0).compareTo(__i1) })
    if (__is1 != null) {
        val at = __is1 as Int
        println(console, "6. found 30 at $at")
    }
    val live: salvo.platform.core.list.MutList<Int> = mutSort(listOf<Int>(10, 30), { __i0, __i1 -> (__i0).compareTo(__i1) })
    addSorted(live, 20, { __i0, __i1 -> (__i0).compareTo(__i1) })
    addSorted(live, 5, { __i0, __i1 -> (__i0).compareTo(__i1) })
    println(console, "6. still sorted ${toStr__core_list(live, { __i0 -> (__i0).toString() })}")
    val bylen = sort(listOf<String>("alpha", "be", "z"), ::byLen)
    println(console, "6. by length ${toStr__core_list(bylen, { __i0 -> __i0 })}")
    var __is2 = binarySearch(bylen, "hi", ::byLen)
    if (__is2 != null) {
        val atLen = __is2 as Int
        println(console, "6. a two-letter word at $atLen")
    }
    val unique = toListPlatform(deduped)
    println(console, "6. distinct ${toStr__core_list(unique, { __i0 -> (__i0).toString() })} of ${countUnique(unique)}")
    var __loop1_pass = iter__core_set(vowels)
    while (true) {
        val __loop1_step = next__core_set(__loop1_pass)
        if (__loop1_step !is Union2.U1<*, *>) { break }
        val v = __loop1_step.value as String
        console.print(v)
    }
    println(console, "")
    var __loop2_pass = iter__core_map(ages)
    while (true) {
        val __loop2_step = next__core_map(__loop2_pass)
        if (__loop2_step !is Union2.U1<*, *>) { break }
        val name = __loop2_step.value as String
        val age = getPlatform(ages, name)
        if (age != null) {
            println(console, "7. $name is $age")
        }
    }
}

fun cmp(a: Point, b: Point): Int {
    val c__c1 = (a.x).compareTo(b.x)
    if (c__c1 != 0) {
        return c__c1
    }
    val c__c2 = (a.y).compareTo(b.y)
    if (c__c2 != 0) {
        return c__c2
    }
    return 0
}

fun hash(value: Point): Long {
    var h = 17L
    h = ((h) * 31L + ((value.x).hashCode().toLong()))
    h = ((h) * 31L + ((value.y).hashCode().toLong()))
    return h
}

fun eq__Point_Point(a: Point, b: Point): Boolean {
    if (!((a.x) == (b.x))) {
        return false
    }
    if (!((a.y) == (b.y))) {
        return false
    }
    return true
}

fun eq__Note_Note(a: Note, b: Note): Boolean {
    if (!((a.text) == (b.text))) {
        return false
    }
    return true
}
