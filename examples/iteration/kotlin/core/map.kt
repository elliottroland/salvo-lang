package salvo.core.map

import salvo.*
import salvo.core.array.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.seq.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

fun<K, V> KeyOf_qualifies(key: K, map: Map<K, V>): Boolean {
    return map.containsKey(key)
}

fun<K, V> get__2(map: Map<K, V>, key: K): V {
    return map[key]!!
}

fun<K, V> iter__4(map: Map<K, V>): MapKeyYield<K> {
    return MapKeyYield(items = map.keys.toMutableList(), at = 0)
}

data class MapKeyYield<K>(
    var items: List<K>,
    var at: Int,
)

class __Codec_MapKeyYield<K>(private val __c_K: salvo.WireCodec<K>) : salvo.WireCodec<MapKeyYield<K>> {
    override fun enc(v: MapKeyYield<K>, out: salvo.WireOut) {
        salvo.ListCodec(__c_K).enc(v.items, out)
        salvo.IntCodec.enc(v.at, out)
    }
    override fun dec(inp: salvo.WireIn): MapKeyYield<K> = MapKeyYield(salvo.ListCodec(__c_K).dec(inp), salvo.IntCodec.dec(inp))
}

fun<K> next__9(p: MapKeyYield<K>): Union2<K, Finished> {
    val key = p.items.getOrNull(p.at)
    if (key == null) {
        return U2_2<K, Finished>(finished())
    }
    p.at = p.at + 1
    return U2_1<K, Finished>(emitted(key))
}

fun<K, V> NonEmpty_qualifies(map: Map<K, V>): Boolean {
    return map.size > 0
}
