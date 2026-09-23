package salvo.core.sorted

import salvo.core.bytes.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.seq.*
import salvo.core.set.*
import salvo.core.string.*

fun<T> iter__7(set: java.util.SortedSet<T>): SetYield<T> {
    return SetYield(items = set.toMutableList(), at = 0)
}

fun<K, V> iter__8(map: java.util.SortedMap<K, V>): MapKeyYield<K> {
    return MapKeyYield(items = map.keys.toMutableList(), at = 0)
}
