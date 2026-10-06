// [platform-value-type] [platform-slots] std's map, for `core.map`'s `platform
// type Map<K, V>(?hash, ?eq) canbe Mut`: one insertion-ordered slab for every
// identity (ROADMAP §0j step 7, user decision 2026-10-06). Every operation is
// handed the `hash` and `eq` its call resolved, lent for the call
// [platform-fn-value]; the map keeps the entries and the digest each key was
// filed under, nothing else. The operations are `inline`, as the set's are,
// so the canonical identity compiles to the direct `hashCode`/`==` [kt-keyed].
package salvo.platform.core.map

typealias Map<K, V> = SalvoMap<K, V>
typealias MutMap<K, V> = SalvoMap<K, V>

/**
 * Entries in first-insertion order ([SalvoMap.TOMB] as the key where one was
 * removed), each key's digest, and an open-addressing index over a
 * power-of-two table holding `slot + 1` (0 is empty). An overwrite keeps the
 * key's position [col-insertion-order].
 */
class SalvoMap<K, V> @PublishedApi internal constructor() : Iterable<Pair<K, V>> {
    @PublishedApi internal var keys: Array<Any?> = arrayOfNulls(8)
    @PublishedApi internal var values: Array<Any?> = arrayOfNulls(8)
    @PublishedApi internal var digests: LongArray = LongArray(8)
    @PublishedApi internal var count = 0
    @PublishedApi internal var live = 0
    @PublishedApi internal var index: IntArray = IntArray(16)

    @PublishedApi internal fun home(digest: Long): Int {
        val h = digest xor (digest ushr 29) xor (digest ushr 47)
        return h.toInt() and (index.size - 1)
    }

    /** The slot holding a key equal to [key], or -1. */
    @Suppress("UNCHECKED_CAST")
    @PublishedApi internal inline fun find(key: K, digest: Long, eq: (K, K) -> Boolean): Int {
        val mask = index.size - 1
        var i = home(digest)
        while (true) {
            val at = index[i]
            if (at == 0) return -1
            val s = at - 1
            if (digests[s] == digest) {
                val k = keys[s]
                if (k !== TOMB && eq(k as K, key)) return s
            }
            i = (i + 1) and mask
        }
    }

    @PublishedApi internal fun append(key: K, value: V, digest: Long) {
        if ((count + 1) * 4 > index.size * 3) rebuild()
        if (count == keys.size) {
            keys = keys.copyOf(count * 2)
            values = values.copyOf(count * 2)
            digests = digests.copyOf(count * 2)
        }
        keys[count] = key
        values[count] = value
        digests[count] = digest
        count++
        live++
        place(count - 1)
    }

    private fun place(s: Int) {
        val mask = index.size - 1
        var i = home(digests[s])
        while (index[i] != 0) i = (i + 1) and mask
        index[i] = s + 1
    }

    private fun rebuild() {
        var w = 0
        for (r in 0 until count) {
            if (keys[r] !== TOMB) {
                keys[w] = keys[r]
                values[w] = values[r]
                digests[w] = digests[r]
                w++
            }
        }
        for (r in w until count) {
            keys[r] = null
            values[r] = null
        }
        count = w
        var cap = 16
        while (cap * 3 < (count + 1) * 8) cap *= 2
        index = IntArray(cap)
        for (s in 0 until count) place(s)
    }

    @Suppress("UNCHECKED_CAST")
    @PublishedApi internal fun removeAt(s: Int): V {
        val old = values[s] as V
        keys[s] = TOMB
        values[s] = null
        live--
        if (count > 2 * live + 8) rebuild()
        return old
    }

    /** Stores [value] under [key]: an existing key keeps its place and takes the new key and value. */
    @Suppress("UNCHECKED_CAST")
    @PublishedApi internal inline fun store(key: K, value: V, hash: (K) -> Long, eq: (K, K) -> Boolean): Pair<Boolean, V?> {
        val d = hash(key)
        val i = find(key, d, eq)
        if (i < 0) {
            append(key, value, d)
            return Pair(false, null)
        }
        val old = values[i] as V
        keys[i] = key
        values[i] = value
        return Pair(true, old)
    }

    /** The live entries' slot numbers, in insertion order. */
    @PublishedApi internal fun liveSlots(): Sequence<Int> = (0 until count).asSequence().filter { keys[it] !== TOMB }

    /** The entries, in insertion order: what a boundary check and host code walk. */
    @Suppress("UNCHECKED_CAST")
    override fun iterator(): Iterator<Pair<K, V>> = liveSlots().map { Pair(keys[it] as K, values[it] as V) }.iterator()

    /** A fresh map with the same entries, filed the same way; the values are shared. */
    @Suppress("UNCHECKED_CAST")
    fun copy(): SalvoMap<K, V> {
        val out = SalvoMap<K, V>()
        for (i in liveSlots()) out.append(keys[i] as K, values[i] as V, digests[i])
        return out
    }

    /** Order-blind, on host equality (what a data class holding a map compares by; ROADMAP §0j step 8). */
    @Suppress("UNCHECKED_CAST")
    override fun equals(other: Any?): Boolean {
        if (other !is SalvoMap<*, *>) return false
        if (live != other.live) return false
        val o = other as SalvoMap<Any?, Any?>
        for (i in liveSlots()) {
            val j = o.find(keys[i], digests[i]) { a, b -> a == b }
            if (j < 0 || o.values[j] != values[i]) return false
        }
        return true
    }

    override fun hashCode(): Int {
        var h = 0L
        for (i in liveSlots()) h += digests[i] * 31 + (values[i]?.hashCode() ?: 0)
        return (h xor (h ushr 32)).toInt()
    }

    /** `{a: 1, b: 2}`, in insertion order — the map literal that would build it [col-to-str]. */
    override fun toString(): String = liveSlots().joinToString(", ", "{", "}") { "${keys[it]}: ${values[it]}" }

    @PublishedApi internal companion object {
        @PublishedApi internal val TOMB = Any()
    }
}

/**
 * [platform-value-type] For host code: a map keyed **canonically** — the
 * identity Salvo resolves for an intrinsic key type (`Str`, `Int`, `Long`,
 * `Char`, `Bool`, `Byte`), whose `hash` is the JVM's `hashCode` and whose
 * `eq` is `==`. In the order given; a repeated key keeps its first place and
 * takes its last value [col-duplicate-keys].
 */
fun <K, V> canonicalMap(entries: Iterable<Pair<K, V>>): SalvoMap<K, V> {
    val m = SalvoMap<K, V>()
    for ((k, v) in entries) m.store(k, v, { it.hashCode().toLong() }, { a, b -> a == b })
    return m
}

// [platform-iterable] [col-map-iter] The host's loop over a map: its keys.
@Suppress("UNCHECKED_CAST")
fun <K, V> each(map: Map<K, V>): Iterable<K> = map.liveSlots().map { map.keys[it] as K }.asIterable()

inline fun <K, V> mapOf(entries: Array<Pair<K, V>>, hash: (K) -> Long, eq: (K, K) -> Boolean): Map<K, V> =
    mutMapOf(entries, hash, eq)

// A repeated key keeps the position of its first appearance and takes the
// value of its last [col-duplicate-keys].
inline fun <K, V> mutMapOf(entries: Array<Pair<K, V>>, hash: (K) -> Long, eq: (K, K) -> Boolean): MutMap<K, V> {
    val m = SalvoMap<K, V>()
    for ((k, v) in entries) m.store(k, v, hash, eq)
    return m
}

inline fun <K, V> mapBy(size: Int, init: (Int) -> Pair<K, V>, hash: (K) -> Long, eq: (K, K) -> Boolean): Map<K, V> =
    mutMapBy(size, init, hash, eq)

inline fun <K, V> mutMapBy(size: Int, init: (Int) -> Pair<K, V>, hash: (K) -> Long, eq: (K, K) -> Boolean): MutMap<K, V> {
    val m = SalvoMap<K, V>()
    for (i in 0 until size) {
        val (k, v) = init(i)
        m.store(k, v, hash, eq)
    }
    return m
}

inline fun <K, V> toMap(pairs: List<Pair<K, V>>, hash: (K) -> Long, eq: (K, K) -> Boolean): Map<K, V> {
    val m = SalvoMap<K, V>()
    for ((k, v) in pairs) m.store(k, v, hash, eq)
    return m
}

@Suppress("UNCHECKED_CAST")
inline fun <K, V> get(map: Map<K, V>, key: K, hash: (K) -> Long, eq: (K, K) -> Boolean): V? {
    val i = map.find(key, hash(key), eq)
    return if (i < 0) null else map.values[i] as V
}

// The claim proved the key present [qual-depend].
@Suppress("UNCHECKED_CAST")
inline fun <K, V> getPresent(map: Map<K, V>, key: K, hash: (K) -> Long, eq: (K, K) -> Boolean): V =
    map.values[map.find(key, hash(key), eq)] as V

inline fun <K, V> put(map: MutMap<K, V>, key: K, value: V, hash: (K) -> Long, eq: (K, K) -> Boolean) {
    map.store(key, value, hash, eq)
}

inline fun <K, V> replace(map: MutMap<K, V>, key: K, value: V, hash: (K) -> Long, eq: (K, K) -> Boolean): V? =
    map.store(key, value, hash, eq).second

inline fun <K, V> remove(map: MutMap<K, V>, key: K, hash: (K) -> Long, eq: (K, K) -> Boolean): V? {
    val i = map.find(key, hash(key), eq)
    return if (i < 0) null else map.removeAt(i)
}

inline fun <K, V> containsKey(map: Map<K, V>, key: K, hash: (K) -> Long, eq: (K, K) -> Boolean): Boolean =
    map.find(key, hash(key), eq) >= 0

fun <K, V> size(map: Map<K, V>): Int = map.live

@Suppress("UNCHECKED_CAST")
fun <K, V> keys(map: Map<K, V>): MutableList<K> = map.liveSlots().map { map.keys[it] as K }.toMutableList()

// [linear-container] The values, in insertion order.
@Suppress("UNCHECKED_CAST")
fun <K, V> intoValues(map: Map<K, V>): MutableList<V> = map.liveSlots().map { map.values[it] as V }.toMutableList()

// [col-map-entries] The slots, live or removed: what `entries` and `values` walk.
fun <K, V> slotCount(map: Map<K, V>): Int = map.count

@Suppress("UNCHECKED_CAST")
fun <K, V> keyAt(map: Map<K, V>, at: Int): K? {
    val k = map.keys[at]
    return if (k === SalvoMap.TOMB) null else k as K
}

@Suppress("UNCHECKED_CAST")
fun <K, V> valueAt(map: Map<K, V>, at: Int): V = map.values[at] as V
