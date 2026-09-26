package salvo.core.map

import salvo.*
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

fun<K> next__7(p: MapKeyYield<K>): Union2<K, Finished> {
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
