package salvo.core.sorted

import salvo.core.list.at
import salvo.core.map.MapKeyYield
import salvo.core.set.SetYield
import salvo.core.string.appendPlatform
import salvo.core.string.mutStr

fun<T> addPlatform(set: salvo.platform.core.sorted.MutSortedSet<T>, elem: T): Boolean {
    return salvo.platform.core.sorted.add(set, elem)
}

fun<T> remove__SortedSet_T(set: salvo.platform.core.sorted.MutSortedSet<T>, elem: T): Boolean {
    return removeElemPlatform(set, elem)
}

fun<T> removeElemPlatform(set: salvo.platform.core.sorted.MutSortedSet<T>, elem: T): Boolean {
    return salvo.platform.core.sorted.removeElem(set, elem)
}

fun<T> setSizePlatform(set: java.util.SortedSet<T>): Int {
    return salvo.platform.core.sorted.setSize(set)
}

fun<K, V> removeKeyPlatform(map: salvo.platform.core.sorted.MutSortedMap<K, V>, key: K): V? {
    return salvo.platform.core.sorted.removeKey(map, key)
}

fun<K, V> mapSizePlatform(map: java.util.SortedMap<K, V>): Int {
    return salvo.platform.core.sorted.mapSize(map)
}

fun<T> containsPlatform(set: java.util.SortedSet<T>, elem: T): Boolean {
    return salvo.platform.core.sorted.contains(set, elem)
}

fun<T> size__SortedSet(set: java.util.SortedSet<T>): Int {
    return setSizePlatform(set)
}

fun<T> minPlatform(set: java.util.SortedSet<T>): T? {
    return salvo.platform.core.sorted.min(set)
}

fun<T> maxPlatform(set: java.util.SortedSet<T>): T? {
    return salvo.platform.core.sorted.max(set)
}

fun<T> toListPlatform(set: java.util.SortedSet<T>): List<T> {
    return salvo.platform.core.sorted.toList(set)
}

fun<T> toStr(set: java.util.SortedSet<T>, toStr: (T) -> String): String {
    val out = mutStr(arrayOf("{"))
    var i = 0
    for (x in salvo.platform.core.sorted.eachSortedSet(set)) {
        if (i > 0) {
            appendPlatform(out, ", ")
        }
        appendPlatform(out, toStr(x))
        i = i + 1
    }
    appendPlatform(out, "}")
    return out.toString()
}

fun<T> iter__SortedSet(set: java.util.SortedSet<T>): SetYield<T> {
    return SetYield(items = toListPlatform(set), at = 0)
}

fun<K, V> getPlatform(map: java.util.SortedMap<K, V>, key: K): V? {
    return salvo.platform.core.sorted.get(map, key)
}

fun<K, V> putPlatform(map: salvo.platform.core.sorted.MutSortedMap<K, V>, key: K, value: V) {
    return salvo.platform.core.sorted.put(map, key, value)
}

fun<K, V> remove__SortedMap_K(map: salvo.platform.core.sorted.MutSortedMap<K, V>, key: K): V? {
    return removeKeyPlatform(map, key)
}

fun<K, V> containsKeyPlatform(map: java.util.SortedMap<K, V>, key: K): Boolean {
    return salvo.platform.core.sorted.containsKey(map, key)
}

fun<K, V> size__SortedMap(map: java.util.SortedMap<K, V>): Int {
    return mapSizePlatform(map)
}

fun<K, V> firstKeyPlatform(map: java.util.SortedMap<K, V>): K? {
    return salvo.platform.core.sorted.firstKey(map)
}

fun<K, V> lastKeyPlatform(map: java.util.SortedMap<K, V>): K? {
    return salvo.platform.core.sorted.lastKey(map)
}

fun<K, V> keysPlatform(map: java.util.SortedMap<K, V>): List<K> {
    return salvo.platform.core.sorted.keys(map)
}

fun<K, V> iter__SortedMap(map: java.util.SortedMap<K, V>): MapKeyYield<K> {
    return MapKeyYield(items = keysPlatform(map), at = 0)
}

fun<T> NonEmpty_qualifies(set: java.util.SortedSet<T>): Boolean {
    return size__SortedSet(set) > 0
}

fun<K, V> NonEmpty_qualifies(map: java.util.SortedMap<K, V>): Boolean {
    return size__SortedMap(map) > 0
}

fun<T> min(set: java.util.SortedSet<T>): T {
    return (minPlatform(set) ?: throw AssertionError("salvo: value is absent at core.sorted:188:12"))
}

fun<T> max(set: java.util.SortedSet<T>): T {
    return (maxPlatform(set) ?: throw AssertionError("salvo: value is absent at core.sorted:192:12"))
}

fun<K, V> firstKey(map: java.util.SortedMap<K, V>): K {
    return (firstKeyPlatform(map) ?: throw AssertionError("salvo: value is absent at core.sorted:196:12"))
}

fun<K, V> lastKey(map: java.util.SortedMap<K, V>): K {
    return (lastKeyPlatform(map) ?: throw AssertionError("salvo: value is absent at core.sorted:200:12"))
}
