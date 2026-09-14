package salvo.core.nonempty

import salvo.core.array.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.seq.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

fun<T> NonEmpty_qualifies(set: Set<T>): Boolean {
    return set.size > 0
}

fun<K, V> NonEmpty_qualifies(map: Map<K, V>): Boolean {
    return map.size > 0
}

fun<T> NonEmpty_qualifies(set: java.util.SortedSet<T>): Boolean {
    return set.size > 0
}

fun<K, V> NonEmpty_qualifies(map: java.util.SortedMap<K, V>): Boolean {
    return map.size > 0
}

fun<T> min(set: java.util.SortedSet<T>): T {
    return set.firstOrNull()!!
}

fun<T> max(set: java.util.SortedSet<T>): T {
    return set.lastOrNull()!!
}

fun<K, V> first_key(map: java.util.SortedMap<K, V>): K {
    return map.keys.firstOrNull()!!
}

fun<K, V> last_key(map: java.util.SortedMap<K, V>): K {
    return map.keys.lastOrNull()!!
}
