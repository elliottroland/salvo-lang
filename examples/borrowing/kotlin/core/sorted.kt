package salvo.core.sorted

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

fun<T> NonEmpty_qualifies(set: java.util.SortedSet<T>): Boolean {
    return set.size > 0
}

fun<K, V> NonEmpty_qualifies(map: java.util.SortedMap<K, V>): Boolean {
    return map.size > 0
}

fun<T> min(set: java.util.SortedSet<T>): T {
    return (set.firstOrNull() ?: throw AssertionError("salvo: value is absent at core.sorted:160:12"))
}

fun<T> max(set: java.util.SortedSet<T>): T {
    return (set.lastOrNull() ?: throw AssertionError("salvo: value is absent at core.sorted:164:12"))
}

fun<K, V> first_key(map: java.util.SortedMap<K, V>): K {
    return (map.keys.firstOrNull() ?: throw AssertionError("salvo: value is absent at core.sorted:168:12"))
}

fun<K, V> last_key(map: java.util.SortedMap<K, V>): K {
    return (map.keys.lastOrNull() ?: throw AssertionError("salvo: value is absent at core.sorted:172:12"))
}
