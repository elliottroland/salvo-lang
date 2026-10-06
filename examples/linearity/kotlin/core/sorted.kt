package salvo.core.sorted

import salvo.core.checked.toStr
import salvo.core.deque.get
import salvo.core.deque.iter
import salvo.core.deque.toList
import salvo.core.deque.toStr
import salvo.core.list.contains
import salvo.core.list.get
import salvo.core.list.iter
import salvo.core.list.toStr
import salvo.core.map.MapKeyYield
import salvo.core.map.eq
import salvo.core.map.get
import salvo.core.map.iter
import salvo.core.map.toStr
import salvo.core.set.SetYield
import salvo.core.set.eq
import salvo.core.set.iter
import salvo.core.set.toStr
import salvo.core.string.appendPlatform
import salvo.core.string.iter
import salvo.core.string.mutStr

inline fun<T> sortedSetOfPlatform(elems: Array<T>, cmp: (T, T) -> Int): salvo.platform.core.sorted.SortedSet<T> {
    return salvo.platform.core.sorted.sortedSetOf(elems, cmp)
}

inline fun<T> mutSortedSetOfPlatform(elems: Array<T>, cmp: (T, T) -> Int): salvo.platform.core.sorted.MutSortedSet<T> {
    return salvo.platform.core.sorted.mutSortedSetOf(elems, cmp)
}

inline fun<T> addPlatform(set: salvo.platform.core.sorted.MutSortedSet<T>, elem: T, cmp: (T, T) -> Int): Boolean {
    return salvo.platform.core.sorted.add(set, elem, cmp)
}

fun<T> remove__SortedSet_T(set: salvo.platform.core.sorted.MutSortedSet<T>, elem: T, cmp: (T, T) -> Int): Boolean {
    return removeElemPlatform(set, elem, cmp)
}

inline fun<T> removeElemPlatform(set: salvo.platform.core.sorted.MutSortedSet<T>, elem: T, cmp: (T, T) -> Int): Boolean {
    return salvo.platform.core.sorted.removeElem(set, elem, cmp)
}

fun<T> setSizePlatform(set: salvo.platform.core.sorted.SortedSet<T>): Int {
    return salvo.platform.core.sorted.setSize(set)
}

inline fun<K, V> removeKeyPlatform(map: salvo.platform.core.sorted.MutSortedMap<K, V>, key: K, cmp: (K, K) -> Int): V? {
    return salvo.platform.core.sorted.removeKey(map, key, cmp)
}

fun<K, V> mapSizePlatform(map: salvo.platform.core.sorted.SortedMap<K, V>): Int {
    return salvo.platform.core.sorted.mapSize(map)
}

inline fun<T> containsPlatform(set: salvo.platform.core.sorted.SortedSet<T>, elem: T, cmp: (T, T) -> Int): Boolean {
    return salvo.platform.core.sorted.contains(set, elem, cmp)
}

fun<T> size__SortedSet(set: salvo.platform.core.sorted.SortedSet<T>): Int {
    return setSizePlatform(set)
}

fun<T> minPlatform(set: salvo.platform.core.sorted.SortedSet<T>): T? {
    return salvo.platform.core.sorted.min(set)
}

fun<T> maxPlatform(set: salvo.platform.core.sorted.SortedSet<T>): T? {
    return salvo.platform.core.sorted.max(set)
}

fun<T> toListPlatform(set: salvo.platform.core.sorted.SortedSet<T>): List<T> {
    return salvo.platform.core.sorted.toList(set)
}

fun<T> toStr__SortedSet(set: salvo.platform.core.sorted.SortedSet<T>, toStr: (T) -> String): String {
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

fun<T> iter__SortedSet(set: salvo.platform.core.sorted.SortedSet<T>): SetYield<T> {
    return SetYield(items = toListPlatform(set), at = 0)
}

inline fun<K, V> sortedMapOfPlatform(entries: Array<Pair<K, V>>, cmp: (K, K) -> Int): salvo.platform.core.sorted.SortedMap<K, V> {
    return salvo.platform.core.sorted.sortedMapOf(entries, cmp)
}

inline fun<K, V> mutSortedMapOfPlatform(entries: Array<Pair<K, V>>, cmp: (K, K) -> Int): salvo.platform.core.sorted.MutSortedMap<K, V> {
    return salvo.platform.core.sorted.mutSortedMapOf(entries, cmp)
}

inline fun<K, V> getPlatform(map: salvo.platform.core.sorted.SortedMap<K, V>, key: K, cmp: (K, K) -> Int): V? {
    return salvo.platform.core.sorted.get(map, key, cmp)
}

inline fun<K, V> putPlatform(map: salvo.platform.core.sorted.MutSortedMap<K, V>, key: K, value: V, cmp: (K, K) -> Int) {
    return salvo.platform.core.sorted.put(map, key, value, cmp)
}

fun<K, V> remove__SortedMap_K(map: salvo.platform.core.sorted.MutSortedMap<K, V>, key: K, cmp: (K, K) -> Int): V? {
    return removeKeyPlatform(map, key, cmp)
}

inline fun<K, V> containsKeyPlatform(map: salvo.platform.core.sorted.SortedMap<K, V>, key: K, cmp: (K, K) -> Int): Boolean {
    return salvo.platform.core.sorted.containsKey(map, key, cmp)
}

fun<K, V> size__SortedMap(map: salvo.platform.core.sorted.SortedMap<K, V>): Int {
    return mapSizePlatform(map)
}

fun<K, V> firstKeyPlatform(map: salvo.platform.core.sorted.SortedMap<K, V>): K? {
    return salvo.platform.core.sorted.firstKey(map)
}

fun<K, V> lastKeyPlatform(map: salvo.platform.core.sorted.SortedMap<K, V>): K? {
    return salvo.platform.core.sorted.lastKey(map)
}

fun<K, V> keysPlatform(map: salvo.platform.core.sorted.SortedMap<K, V>): List<K> {
    return salvo.platform.core.sorted.keys(map)
}

fun<K, V> toStr__SortedMap(map: salvo.platform.core.sorted.SortedMap<K, V>, toStr: (K) -> String, toStr__1: (V) -> String, cmp: (K, K) -> Int): String {
    val out = mutStr(arrayOf("{"))
    var i = 0
    for (k in salvo.platform.core.sorted.eachSortedMap(map)) {
        if (i > 0) {
            appendPlatform(out, ", ")
        }
        appendPlatform(out, toStr(k))
        appendPlatform(out, ": ")
        appendPlatform(out, toStr__1((getPlatform(map, k, cmp) ?: throw AssertionError("salvo: value is absent at core.sorted:148:28"))))
        i = i + 1
    }
    appendPlatform(out, "}")
    return out.toString()
}

fun<T> eq__SortedSet_SortedSet(a: salvo.platform.core.sorted.SortedSet<T>, b: salvo.platform.core.sorted.SortedSet<T>, cmp: (T, T) -> Int): Boolean {
    if (size__SortedSet(a) != size__SortedSet(b)) {
        return false
    }
    for (x in salvo.platform.core.sorted.eachSortedSet(a)) {
        if (!containsPlatform(b, x, cmp)) {
            return false
        }
    }
    return true
}

fun<K, V> eq__SortedMap_SortedMap(a: salvo.platform.core.sorted.SortedMap<K, V>, b: salvo.platform.core.sorted.SortedMap<K, V>, eq: (V, V) -> Boolean, cmp: (K, K) -> Int): Boolean {
    if (size__SortedMap(a) != size__SortedMap(b)) {
        return false
    }
    for (k in salvo.platform.core.sorted.eachSortedMap(a)) {
        val theirs = getPlatform(b, k, cmp)
        if (theirs == null) {
            return false
        }
        if (!eq((getPlatform(a, k, cmp) ?: throw AssertionError("salvo: value is absent at core.sorted:181:16")), theirs)) {
            return false
        }
    }
    return true
}

fun<K, V> iter__SortedMap(map: salvo.platform.core.sorted.SortedMap<K, V>): MapKeyYield<K> {
    return MapKeyYield(items = keysPlatform(map), at = 0)
}

fun<T> NonEmpty_qualifies(set: salvo.platform.core.sorted.SortedSet<T>): Boolean {
    return size__SortedSet(set) > 0
}

fun<K, V> NonEmpty_qualifies(map: salvo.platform.core.sorted.SortedMap<K, V>): Boolean {
    return size__SortedMap(map) > 0
}

fun<T> min(set: salvo.platform.core.sorted.SortedSet<T>): T {
    return (minPlatform(set) ?: throw AssertionError("salvo: value is absent at core.sorted:236:12"))
}

fun<T> max(set: salvo.platform.core.sorted.SortedSet<T>): T {
    return (maxPlatform(set) ?: throw AssertionError("salvo: value is absent at core.sorted:240:12"))
}

fun<K, V> firstKey(map: salvo.platform.core.sorted.SortedMap<K, V>): K {
    return (firstKeyPlatform(map) ?: throw AssertionError("salvo: value is absent at core.sorted:244:12"))
}

fun<K, V> lastKey(map: salvo.platform.core.sorted.SortedMap<K, V>): K {
    return (lastKeyPlatform(map) ?: throw AssertionError("salvo: value is absent at core.sorted:248:12"))
}
