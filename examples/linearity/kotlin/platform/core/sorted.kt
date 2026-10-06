// [platform-value-type] [platform-slots] std's sorted pair, for `core.sorted`'s
// `platform type SortedSet<T>(?cmp)` and `SortedMap<K, V>(?cmp)`: a sorted
// list searched by bisection (ROADMAP §0j step 7, user decision 2026-10-06).
// Every operation is handed the `cmp` its call resolved, lent for the call
// [platform-fn-value]; the value keeps only its members, in order. Two
// members are one when `cmp` answers 0 [col-membership].
package salvo.platform.core.sorted

typealias SortedSet<T> = SalvoSortedSet<T>
typealias MutSortedSet<T> = SalvoSortedSet<T>
typealias SortedMap<K, V> = SalvoSortedMap<K, V>
typealias MutSortedMap<K, V> = SalvoSortedMap<K, V>

/** Where [x] is in [items], or `-(insertion point) - 1`, as `binarySearch` answers. */
@PublishedApi internal inline fun <T> bisect(items: List<T>, x: T, cmp: (T, T) -> Int): Int {
    var lo = 0
    var hi = items.size - 1
    while (lo <= hi) {
        val mid = (lo + hi) ushr 1
        val c = cmp(items[mid], x)
        when {
            c < 0 -> lo = mid + 1
            c > 0 -> hi = mid - 1
            else -> return mid
        }
    }
    return -(lo + 1)
}

class SalvoSortedSet<T> @PublishedApi internal constructor(@PublishedApi internal val items: ArrayList<T>) : Iterable<T> {
    override fun iterator(): Iterator<T> = items.iterator()

    fun copy(): SalvoSortedSet<T> = SalvoSortedSet(ArrayList(items))

    // In order on both sides, so equality is the lists' (host equality on the
    // members; ROADMAP §0j step 8).
    override fun equals(other: Any?): Boolean = other is SalvoSortedSet<*> && items == other.items

    override fun hashCode(): Int = items.hashCode()

    override fun toString(): String = items.joinToString(", ", "{", "}")
}

class SalvoSortedMap<K, V> @PublishedApi internal constructor(
    @PublishedApi internal val keys: ArrayList<K>,
    @PublishedApi internal val values: ArrayList<V>,
) {
    fun copy(): SalvoSortedMap<K, V> = SalvoSortedMap(ArrayList(keys), ArrayList(values))

    /** The entries, in key order: what a boundary check walks. */
    operator fun iterator(): Iterator<Pair<K, V>> = keys.indices.map { Pair(keys[it], values[it]) }.iterator()

    override fun equals(other: Any?): Boolean =
        other is SalvoSortedMap<*, *> && keys == other.keys && values == other.values

    override fun hashCode(): Int = keys.hashCode() * 31 + values.hashCode()

    override fun toString(): String = keys.indices.joinToString(", ", "{", "}") { "${keys[it]}: ${values[it]}" }

    /** Stores under [key]; an equal key is replaced, key and value. Answers the displaced value. */
    @PublishedApi internal inline fun store(key: K, value: V, cmp: (K, K) -> Int): V? {
        val i = bisect(keys, key, cmp)
        if (i >= 0) {
            val old = values[i]
            keys[i] = key
            values[i] = value
            return old
        }
        keys.add(-i - 1, key)
        values.add(-i - 1, value)
        return null
    }
}

/**
 * [platform-value-type] Salvo's canonical ordering for an intrinsic type:
 * natural order, except that a `String` compares by **code point** (the JVM's
 * `compareTo` is UTF-16 code-unit order) [kt-ordered].
 */
@Suppress("UNCHECKED_CAST")
fun canonicalCompare(a: Any?, b: Any?): Int {
    if (a is String && b is String) {
        var i = 0
        var j = 0
        while (i < a.length && j < b.length) {
            val ca = a.codePointAt(i)
            val cb = b.codePointAt(j)
            if (ca != cb) return ca.compareTo(cb)
            i += Character.charCount(ca)
            j += Character.charCount(cb)
        }
        return (a.length - i).compareTo(b.length - j)
    }
    return (a as Comparable<Any?>).compareTo(b)
}

/** [platform-value-type] For host code: a sorted set under the canonical ordering. */
fun <T> canonicalSortedSet(elems: Iterable<T>): SalvoSortedSet<T> =
    mutSortedSetOf((elems.toList() as List<Any?>).toTypedArray() as Array<T>, ::canonicalCompare)

/** [platform-value-type] For host code: a sorted map under the canonical ordering. */
fun <K, V> canonicalSortedMap(entries: Iterable<Pair<K, V>>): SalvoSortedMap<K, V> {
    val m = SalvoSortedMap(ArrayList<K>(), ArrayList<V>())
    for ((k, v) in entries) m.store(k, v, ::canonicalCompare)
    return m
}

// [platform-iterable] The host's loops: a set's elements and a map's keys.
fun <T> eachSortedSet(set: SortedSet<T>): Iterable<T> = set

fun <K, V> eachSortedMap(map: SortedMap<K, V>): Iterable<K> = map.keys

inline fun <T> sortedSetOf(elems: Array<T>, cmp: (T, T) -> Int): SortedSet<T> = mutSortedSetOf(elems, cmp)

// A repeated member: the last wins, in the place they share.
inline fun <T> mutSortedSetOf(elems: Array<T>, cmp: (T, T) -> Int): MutSortedSet<T> {
    val s = SalvoSortedSet(ArrayList<T>(elems.size))
    for (e in elems) {
        val i = bisect(s.items, e, cmp)
        if (i >= 0) s.items[i] = e else s.items.add(-i - 1, e)
    }
    return s
}

inline fun <T> add(set: MutSortedSet<T>, elem: T, cmp: (T, T) -> Int): Boolean {
    val i = bisect(set.items, elem, cmp)
    if (i >= 0) return false
    set.items.add(-i - 1, elem)
    return true
}

inline fun <T> removeElem(set: MutSortedSet<T>, elem: T, cmp: (T, T) -> Int): Boolean {
    val i = bisect(set.items, elem, cmp)
    if (i < 0) return false
    set.items.removeAt(i)
    return true
}

inline fun <T> contains(set: SortedSet<T>, elem: T, cmp: (T, T) -> Int): Boolean = bisect(set.items, elem, cmp) >= 0

fun <T> setSize(set: SortedSet<T>): Int = set.items.size

fun <T> min(set: SortedSet<T>): T? = set.items.firstOrNull()

fun <T> max(set: SortedSet<T>): T? = set.items.lastOrNull()

fun <T> toList(set: SortedSet<T>): MutableList<T> = ArrayList(set.items)

inline fun <K, V> sortedMapOf(entries: Array<Pair<K, V>>, cmp: (K, K) -> Int): SortedMap<K, V> = mutSortedMapOf(entries, cmp)

inline fun <K, V> mutSortedMapOf(entries: Array<Pair<K, V>>, cmp: (K, K) -> Int): MutSortedMap<K, V> {
    val m = SalvoSortedMap(ArrayList<K>(entries.size), ArrayList<V>(entries.size))
    for ((k, v) in entries) m.store(k, v, cmp)
    return m
}

inline fun <K, V> get(map: SortedMap<K, V>, key: K, cmp: (K, K) -> Int): V? {
    val i = bisect(map.keys, key, cmp)
    return if (i >= 0) map.values[i] else null
}

inline fun <K, V> put(map: MutSortedMap<K, V>, key: K, value: V, cmp: (K, K) -> Int) {
    map.store(key, value, cmp)
}

inline fun <K, V> removeKey(map: MutSortedMap<K, V>, key: K, cmp: (K, K) -> Int): V? {
    val i = bisect(map.keys, key, cmp)
    if (i < 0) return null
    map.keys.removeAt(i)
    return map.values.removeAt(i)
}

inline fun <K, V> containsKey(map: SortedMap<K, V>, key: K, cmp: (K, K) -> Int): Boolean = bisect(map.keys, key, cmp) >= 0

fun <K, V> mapSize(map: SortedMap<K, V>): Int = map.keys.size

fun <K, V> firstKey(map: SortedMap<K, V>): K? = map.keys.firstOrNull()

fun <K, V> lastKey(map: SortedMap<K, V>): K? = map.keys.lastOrNull()

fun <K, V> keys(map: SortedMap<K, V>): MutableList<K> = ArrayList(map.keys)
