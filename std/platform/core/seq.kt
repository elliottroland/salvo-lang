// [platform-value-type] `core.seq`'s list fast path: `filter` over a `List`
// keeps the elements that pass (ROADMAP 0.7).
package salvo.platform.core.seq

fun <T> filter(list: List<T>, keep: (T) -> Boolean): MutableList<T> =
    list.filter(keep).toMutableList()
