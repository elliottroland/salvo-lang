// [platform-value-type] std's set, for `core.set`'s `platform type Set<T>(?hash,
// ?eq) canbe Mut` (ROADMAP 0.7): Kotlin's insertion-ordered sets — a
// `LinkedHashSet`, or the runtime's `SalvoHashSet` when the type names its
// identities [platform-slots].
package salvo.platform.core.set

typealias Set<T> = kotlin.collections.Set<T>
typealias MutSet<T> = kotlin.collections.MutableSet<T>

// [platform-iterable] The host's loop over a set: the set itself.
fun <T> each(set: Set<T>): Iterable<T> = set

fun <T> add(set: MutSet<T>, elem: T): Boolean = set.add(elem)

fun <T> remove(set: MutSet<T>, elem: T): Boolean = set.remove(elem)

fun <T> contains(set: Set<T>, elem: T): Boolean = set.contains(elem)

fun <T> size(set: Set<T>): Int = set.size

// In insertion order [col-insertion-order].
fun <T> toList(set: Set<T>): MutableList<T> = set.toMutableList()
