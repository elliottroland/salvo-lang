// [platform-value-type] [platform-slots] std's map, for `core.map`'s `platform
// type Map<K, V>(?hash, ?eq) canbe Mut`: one insertion-ordered slab for every
// identity (ROADMAP §0j step 7, user decision 2026-10-06). Every operation is
// handed the `hash` and `eq` its call resolved, lent for the call
// [platform-fn-value]; the map keeps the entries and the digest each key was
// filed under, nothing else.
package salvo.platform.core.map

typealias Map<K, V> = SalvoMap<K, V>
typealias MutMap<K, V> = SalvoMap<K, V>

/**
 * Entries in first-insertion order, `null` where one was removed, with each
 * key's digest; [buckets] maps a digest to its slots. An overwrite keeps the
 * key's position [col-insertion-order].
 */
class SalvoMap<K, V> internal constructor() : Iterable<Pair<K, V>> {
    internal val keys = ArrayList<Any?>()
    internal val values = ArrayList<Any?>()
    internal val digests = ArrayList<Long>()
    internal val buckets = HashMap<Long, MutableList<Int>>()
    internal var live = 0

    @Suppress("UNCHECKED_CAST")
    internal fun find(key: K, hash: (K) -> Long, eq: (K, K) -> Boolean): Int {
        val bucket = buckets[hash(key)] ?: return -1
        for (i in bucket) {
            val k = keys[i]
            if (k !== TOMB && eq(k as K, key)) return i
        }
        return -1
    }

    internal fun append(key: K, value: V, digest: Long) {
        keys.add(key)
        values.add(value)
        digests.add(digest)
        buckets.getOrPut(digest) { ArrayList(1) }.add(keys.size - 1)
        live++
    }

    /** Stores [value] under [key]: an existing key keeps its place and takes the new key and value. */
    @Suppress("UNCHECKED_CAST")
    internal fun store(key: K, value: V, hash: (K) -> Long, eq: (K, K) -> Boolean): Pair<Boolean, V?> {
        val i = find(key, hash, eq)
        if (i < 0) {
            append(key, value, hash(key))
            return Pair(false, null)
        }
        val old = values[i] as V
        keys[i] = key
        values[i] = value
        return Pair(true, old)
    }

    @Suppress("UNCHECKED_CAST")
    internal fun removeAt(i: Int): V {
        val d = digests[i]
        val bucket = buckets[d]!!
        bucket.remove(i)
        if (bucket.isEmpty()) buckets.remove(d)
        val old = values[i] as V
        keys[i] = TOMB
        values[i] = null
        live--
        if (keys.size > 2 * live + 8) compact()
        return old
    }

    @Suppress("UNCHECKED_CAST")
    private fun compact() {
        val ks = ArrayList<Any?>(live)
        val vs = ArrayList<Any?>(live)
        val ds = ArrayList<Long>(live)
        for (i in keys.indices) {
            if (keys[i] !== TOMB) {
                ks.add(keys[i]); vs.add(values[i]); ds.add(digests[i])
            }
        }
        keys.clear(); values.clear(); digests.clear(); buckets.clear()
        live = 0
        for (i in ks.indices) append(ks[i] as K, vs[i] as V, ds[i])
    }

    /** The entries, in insertion order: what a boundary check and host code walk. */
    @Suppress("UNCHECKED_CAST")
    override fun iterator(): Iterator<Pair<K, V>> = liveSlots().map { Pair(keys[it] as K, values[it] as V) }.iterator()

    /** The live entries' slot numbers, in insertion order. */
    internal fun liveSlots(): Sequence<Int> = keys.indices.asSequence().filter { keys[it] !== TOMB }

    /** A fresh map with the same entries, filed the same way; the values are shared. */
    @Suppress("UNCHECKED_CAST")
    fun copy(): SalvoMap<K, V> {
        val out = SalvoMap<K, V>()
        for (i in liveSlots()) out.append(keys[i] as K, values[i] as V, digests[i])
        return out
    }

    /** Order-blind, on host equality (what a data class holding a map compares by; ROADMAP §0j step 8). */
    override fun equals(other: Any?): Boolean {
        if (other !is SalvoMap<*, *>) return false
        if (live != other.live) return false
        for (i in liveSlots()) {
            val bucket = other.buckets[digests[i]] ?: return false
            if (bucket.none { other.keys[it] == keys[i] && other.values[it] == values[i] }) return false
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

    internal companion object {
        val TOMB = Any()
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

fun <K, V> mapOf(entries: Array<Pair<K, V>>, hash: (K) -> Long, eq: (K, K) -> Boolean): Map<K, V> =
    mutMapOf(entries, hash, eq)

// A repeated key keeps the position of its first appearance and takes the
// value of its last [col-duplicate-keys].
fun <K, V> mutMapOf(entries: Array<Pair<K, V>>, hash: (K) -> Long, eq: (K, K) -> Boolean): MutMap<K, V> {
    val m = SalvoMap<K, V>()
    for ((k, v) in entries) m.store(k, v, hash, eq)
    return m
}

fun <K, V> mapBy(size: Int, init: (Int) -> Pair<K, V>, hash: (K) -> Long, eq: (K, K) -> Boolean): Map<K, V> =
    mutMapBy(size, init, hash, eq)

fun <K, V> mutMapBy(size: Int, init: (Int) -> Pair<K, V>, hash: (K) -> Long, eq: (K, K) -> Boolean): MutMap<K, V> {
    val m = SalvoMap<K, V>()
    for (i in 0 until size) {
        val (k, v) = init(i)
        m.store(k, v, hash, eq)
    }
    return m
}

fun <K, V> toMap(pairs: List<Pair<K, V>>, hash: (K) -> Long, eq: (K, K) -> Boolean): Map<K, V> {
    val m = SalvoMap<K, V>()
    for ((k, v) in pairs) m.store(k, v, hash, eq)
    return m
}

@Suppress("UNCHECKED_CAST")
fun <K, V> get(map: Map<K, V>, key: K, hash: (K) -> Long, eq: (K, K) -> Boolean): V? {
    val i = map.find(key, hash, eq)
    return if (i < 0) null else map.values[i] as V
}

// The claim proved the key present [qual-depend].
@Suppress("UNCHECKED_CAST")
fun <K, V> getPresent(map: Map<K, V>, key: K, hash: (K) -> Long, eq: (K, K) -> Boolean): V =
    map.values[map.find(key, hash, eq)] as V

fun <K, V> put(map: MutMap<K, V>, key: K, value: V, hash: (K) -> Long, eq: (K, K) -> Boolean) {
    map.store(key, value, hash, eq)
}

fun <K, V> replace(map: MutMap<K, V>, key: K, value: V, hash: (K) -> Long, eq: (K, K) -> Boolean): V? =
    map.store(key, value, hash, eq).second

fun <K, V> remove(map: MutMap<K, V>, key: K, hash: (K) -> Long, eq: (K, K) -> Boolean): V? {
    val i = map.find(key, hash, eq)
    return if (i < 0) null else map.removeAt(i)
}

fun <K, V> containsKey(map: Map<K, V>, key: K, hash: (K) -> Long, eq: (K, K) -> Boolean): Boolean =
    map.find(key, hash, eq) >= 0

fun <K, V> size(map: Map<K, V>): Int = map.live

@Suppress("UNCHECKED_CAST")
fun <K, V> keys(map: Map<K, V>): MutableList<K> = map.liveSlots().map { map.keys[it] as K }.toMutableList()

// [linear-container] The values, in insertion order.
@Suppress("UNCHECKED_CAST")
fun <K, V> intoValues(map: Map<K, V>): MutableList<V> = map.liveSlots().map { map.values[it] as V }.toMutableList()
