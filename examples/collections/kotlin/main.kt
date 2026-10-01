package salvo.main

import salvo.*
import salvo.core.console.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

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
    return (a.length).compareTo(b.length)
}

fun countUnique(xs: List<Int>): Int {
    return xs.size
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun main() {
    val console: Console = StdOutConsole()
    val primes = listOf<Int>(2, 3, 5, 7)
    val vowels = linkedSetOf<String>("a", "e", "i", "o", "u")
    val ages = linkedMapOf<String, Int>(("ada" to 36), ("grace" to 45))
    println(console, "1. list ${primes.joinToString(", ", "[", "]")}")
    println(console, "1. set ${vowels.joinToString(", ", "{", "}")} of ${vowels.size}")
    println(console, "1. map ${ages.entries.joinToString(", ", "{", "}") { "${it.key}: ${it.value}" }}")
    val note: Note = Note(text = "still a struct literal")
    println(console, "1. struct ${note.text}")
    val seen: MutableSet<String> = linkedSetOf<String>()
    seen.add("first")
    println(console, "1. empty then filled ${seen.joinToString(", ", "{", "}")}")
    val tally: MutableMap<String, Int> = linkedMapOf<String, Int>().also { __m -> __m.putAll(listOf(Pair("pear", 1), Pair("apple", 2))) }
    tally.put("fig", 3)
    tally.put("pear", 99)
    println(console, "2. insertion order kept ${tally.entries.joinToString(", ", "{", "}") { "${it.key}: ${it.value}" }}")
    val ranked: java.util.SortedSet<String> = java.util.TreeSet<String>(java.util.Comparator { __a, __b -> salvo.__salvoCompare(__a, __b) }).also { __s -> __s.addAll(listOf("pear", "apple", "fig")) }
    println(console, "2. key order ${ranked.joinToString(", ", "{", "}")}")
    val smallest = ranked.firstOrNull()
    if (smallest != null) {
        println(console, "2. min is cheap here $smallest")
    }
    val corners: MutableSet<Point> = linkedSetOf<Point>()
    corners.add(Point(x = 0, y = 0))
    val again = corners.add(Point(x = 0, y = 0))
    println(console, "3. struct key: size ${corners.size}, second add $again")
    val labels: MutableMap<Point, String> = linkedMapOf<Point, String>()
    labels.put(Point(x = 1, y = 1), "diagonal")
    val found = labels[Point(x = 1, y = 1)]
    if (found != null) {
        println(console, "3. looked up by value $found")
    }
    val a = Point(x = 1, y = 2)
    val b = Point(x = 1, y = 2)
    val c = Point(x = 1, y = 9)
    val same = eq__7(a, b)
    val before = cmp__5(a, c) < 0
    println(console, "4. equal $same, ordered $before")
    val n1 = Note(text = "same")
    val n2 = Note(text = "same")
    val notesEqual = eq__8(n1, n2)
    println(console, "4. plain struct equality $notesEqual")
    val squares = MutableList<Int>(4, { i -> i * i })
    println(console, "5. generated ${squares.joinToString(", ", "[", "]")}")
    val deduped = linkedSetOf<Int>().also { __s -> __s.addAll(primes) }
    println(console, "5. to_set ${deduped.joinToString(", ", "{", "}")}")
    val words = listOf<String>("alpha", "be")
    val lengths = linkedMapOf<String, Int>().also { __m -> words.map({ w -> Pair(w, w.length) }).forEach { __e -> __m.put(__e.first, __e.second) } }
    println(console, "5. to_map with a rule ${lengths.entries.joinToString(", ", "{", "}") { "${it.key}: ${it.value}" }}")
    val filled = listOf<String>("ada", "grace")
    println(console, "6. first is ${first(filled)}, no optional")
    val growing: MutableList<Int> = mutableListOf<Int>()
    growing.add(7)
    println(console, "6. after add, first is ${first(growing)}")
    val ordered = sort(listOf<Int>(40, 10, 30, 20), { __i0, __i1 -> (__i0).compareTo(__i1) })
    println(console, "6. sorted ${ordered.joinToString(", ", "[", "]")}")
    var __is1 = binarySearch(ordered, 30, { __i0, __i1 -> (__i0).compareTo(__i1) })
    if (__is1 != null) {
        val at = __is1 as Int
        println(console, "6. found 30 at $at")
    }
    val live: MutableList<Int> = mutSort(listOf<Int>(10, 30), { __i0, __i1 -> (__i0).compareTo(__i1) })
    addSorted(live, 20, { __i0, __i1 -> (__i0).compareTo(__i1) })
    addSorted(live, 5, { __i0, __i1 -> (__i0).compareTo(__i1) })
    println(console, "6. still sorted ${live.joinToString(", ", "[", "]")}")
    val bylen = sort(listOf<String>("alpha", "be", "z"), ::byLen)
    println(console, "6. by length ${bylen.joinToString(", ", "[", "]")}")
    var __is2 = binarySearch(bylen, "hi", ::byLen)
    if (__is2 != null) {
        val atLen = __is2 as Int
        println(console, "6. a two-letter word at $atLen")
    }
    val unique = deduped.toMutableList()
    println(console, "6. distinct ${unique.joinToString(", ", "[", "]")} of ${countUnique(unique)}")
    var __loop1_pass = iter__5(vowels)
    while (true) {
        val __loop1_step = next__11(__loop1_pass)
        if (__loop1_step !is U2_1<*, *>) { break }
        val v = __loop1_step.value as String
        console.print(v)
    }
    println(console, "")
    var __loop2_pass = iter__4(ages)
    while (true) {
        val __loop2_step = next__9(__loop2_pass)
        if (__loop2_step !is U2_1<*, *>) { break }
        val name = __loop2_step.value as String
        val age = ages[name]
        if (age != null) {
            println(console, "7. $name is $age")
        }
    }
}

fun cmp__5(a: Point, b: Point): Int {
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

fun hash__7(value: Point): Long {
    var h = 17L
    h = ((h) * 31L + ((value.x).hashCode().toLong()))
    h = ((h) * 31L + ((value.y).hashCode().toLong()))
    return h
}

fun eq__7(a: Point, b: Point): Boolean {
    if (!((a.x) == (b.x))) {
        return false
    }
    if (!((a.y) == (b.y))) {
        return false
    }
    return true
}

fun eq__8(a: Note, b: Note): Boolean {
    if (!((a.text) == (b.text))) {
        return false
    }
    return true
}
