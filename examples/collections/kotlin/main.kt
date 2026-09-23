package salvo.main

import salvo.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.console.*
import salvo.core.fs.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.nonempty.*
import salvo.core.range.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

data class Point(
    val x: Int,
    val y: Int,
) : Comparable<Point> {
    override fun compareTo(other: Point): Int {
        run { val __c = salvo.__salvoCompare(x, other.x); if (__c != 0) return __c }
        run { val __c = salvo.__salvoCompare(y, other.y); if (__c != 0) return __c }
        return 0
    }
}

data class Note(
    val text: String,
)

fun by_len(a: String, b: String): Int {
    return (a.length).compareTo(b.length)
}

fun count_unique(xs: List<Int>): Int {
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
    val tally: MutableMap<String, Int> = linkedMapOf<String, Int>(Pair("pear", 1), Pair("apple", 2))
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
    val same = eq__4(a, b)
    val before = cmp__4(a, c) < 0
    println(console, "4. equal $same, ordered $before")
    val n1 = Note(text = "same")
    val n2 = Note(text = "same")
    val notes_equal = eq__5(n1, n2)
    println(console, "4. plain struct equality $notes_equal")
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
    var __is1 = binary_search(ordered, 30, { __i0, __i1 -> (__i0).compareTo(__i1) })
    if (__is1 != null) {
        val at = __is1 as Int
        println(console, "6. found 30 at $at")
    }
    val live: MutableList<Int> = mut_sort(listOf<Int>(10, 30), { __i0, __i1 -> (__i0).compareTo(__i1) })
    add_sorted(live, 20, { __i0, __i1 -> (__i0).compareTo(__i1) })
    add_sorted(live, 5, { __i0, __i1 -> (__i0).compareTo(__i1) })
    println(console, "6. still sorted ${live.joinToString(", ", "[", "]")}")
    val bylen = sort(listOf<String>("alpha", "be", "z"), ::by_len)
    println(console, "6. by length ${bylen.joinToString(", ", "[", "]")}")
    var __is2 = binary_search(bylen, "hi", ::by_len)
    if (__is2 != null) {
        val at_len = __is2 as Int
        println(console, "6. a two-letter word at $at_len")
    }
    val unique = deduped.toMutableList()
    println(console, "6. distinct ${unique.joinToString(", ", "[", "]")} of ${count_unique(unique)}")
    var __loop1_pass = iter__6(vowels)
    while (true) {
        val __loop1_step = next__8(__loop1_pass)
        if (__loop1_step !is U2_1<*, *>) { break }
        val v = __loop1_step.value as String
        console.print(v)
    }
    println(console, "")
    var __loop2_pass = iter__4(ages)
    while (true) {
        val __loop2_step = next__6(__loop2_pass)
        if (__loop2_step !is U2_1<*, *>) { break }
        val name = __loop2_step.value as String
        val age = ages[name]
        if (age != null) {
            println(console, "7. $name is $age")
        }
    }
}

fun cmp__4(a: Point, b: Point): Int {
    return salvo.__salvoCompare(a, b)
}

fun hash__4(value: Point): Long {
    return value.hashCode().toLong()
}

fun eq__4(a: Point, b: Point): Boolean {
    return a == b
}

fun eq__5(a: Note, b: Note): Boolean {
    return a == b
}
