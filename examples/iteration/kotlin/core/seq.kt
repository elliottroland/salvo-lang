package salvo.core.seq

import salvo.*
import salvo.core.array.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.string.*

fun<It, T, U> map(it: It, f: (T) -> U, next: (It) -> Union2<T, Finished>): MutableList<U> {
    val out = mutableListOf<U>()
    while (true) {
        val __loop1_step = next(it)
        if (__loop1_step !is U2_1<*, *>) { break }
        val x = __loop1_step.value as T
        out.add(f(x))
    }
    return out
}

fun<It, T> filter(it: It, keep: (T) -> Boolean, next: (It) -> Union2<T, Finished>): MutableList<T> {
    val out = mutableListOf<T>()
    while (true) {
        val __loop2_step = next(it)
        if (__loop2_step !is U2_1<*, *>) { break }
        val x = __loop2_step.value as T
        if (keep(x)) {
            out.add(x)
        }
    }
    return out
}

fun<It, T, A> reduce(it: It, init: A, f: (A, T) -> A, next: (It) -> Union2<T, Finished>): A {
    var acc = init
    while (true) {
        val __loop3_step = next(it)
        if (__loop3_step !is U2_1<*, *>) { break }
        val x = __loop3_step.value as T
        acc = f(acc, x)
    }
    return acc
}

data class MapYield<It, T, U>(
    var source: It,
    var f: (T) -> U,
    var step: (It) -> Union2<T, Finished>,
)

fun<It, T, U> map_lazy(it: It, f: (T) -> U, next: (It) -> Union2<T, Finished>): MapYield<It, T, U> {
    return MapYield(source = it, f = f, step = next)
}

fun<It, T, U> next__3(p: MapYield<It, T, U>): Union2<U, Finished> {
    val advance = p.step
    val step = advance(p.source)
    when (step) {
        is U2_1<*, *> -> {
            val f = p.f
            return U2_1<U, Finished>(emitted(f((step.value as T))))
        }
        is U2_2<*, *> -> {
            return U2_2<U, Finished>(finished())
        }
    }
}

data class FilterYield<It, T>(
    var source: It,
    var keep: (T) -> Boolean,
    var step: (It) -> Union2<T, Finished>,
)

fun<It, T> filter_lazy(it: It, keep: (T) -> Boolean, next: (It) -> Union2<T, Finished>): FilterYield<It, T> {
    return FilterYield(source = it, keep = keep, step = next)
}

fun<It, T> next__4(p: FilterYield<It, T>): Union2<T, Finished> {
    val advance = p.step
    val keep = p.keep
    var going = true
    while (going) {
        val step = advance(p.source)
        when (step) {
            is U2_1<*, *> -> {
                if (keep((step.value as T))) {
                    return U2_1<T, Finished>(emitted((step.value as T)))
                }
            }
            is U2_2<*, *> -> {
                going = false
            }
        }
    }
    return U2_2<T, Finished>(finished())
}

fun<D, It, T, U> map_to(dest: D, it: It, f: (T) -> U, add: (D, U) -> Unit, next: (It) -> Union2<T, Finished>): D {
    while (true) {
        val __loop4_step = next(it)
        if (__loop4_step !is U2_1<*, *>) { break }
        val x = __loop4_step.value as T
        add(dest, f(x))
    }
    return dest
}

fun<D, It, T> filter_to(dest: D, it: It, keep: (T) -> Boolean, add: (D, T) -> Unit, next: (It) -> Union2<T, Finished>): D {
    while (true) {
        val __loop5_step = next(it)
        if (__loop5_step !is U2_1<*, *>) { break }
        val x = __loop5_step.value as T
        if (keep(x)) {
            add(dest, x)
        }
    }
    return dest
}
