package salvo.core.seq

import salvo.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.set.*
import salvo.core.sorted.*

@Suppress("UNCHECKED_CAST")
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

@Suppress("UNCHECKED_CAST")
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

@Suppress("UNCHECKED_CAST")
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

@Suppress("UNCHECKED_CAST")
fun<D, It, T, U> map_to(dest: D, it: It, f: (T) -> U, add: (D, U) -> Unit, next: (It) -> Union2<T, Finished>): D {
    while (true) {
        val __loop4_step = next(it)
        if (__loop4_step !is U2_1<*, *>) { break }
        val x = __loop4_step.value as T
        add(dest, f(x))
    }
    return dest
}

@Suppress("UNCHECKED_CAST")
fun<D, It, T> filter_to(dest: D, it: It, keep: (T) -> Boolean, add: (D, T) -> Unit, copy: (T) -> T, next: (It) -> Union2<T, Finished>): D {
    while (true) {
        val __loop5_step = next(it)
        if (__loop5_step !is U2_1<*, *>) { break }
        val x = __loop5_step.value as T
        if (keep(x)) {
            add(dest, copy(x))
        }
    }
    return dest
}

fun<T> drop(value: T) {
}
