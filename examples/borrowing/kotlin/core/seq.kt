package salvo.core.seq

import salvo.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, U> map(it: It, f: (T) -> U, next: (It) -> Union2<T, Finished>): MutableList<U> {
    val out = mutableListOf<U>()
    while (true) {
        val __loop1_step = next(it)
        if (__loop1_step !is Union2.U1<*, *>) { break }
        val x = __loop1_step.value as T
        out.add(f(x))
    }
    return out
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> filter(it: It, keep: (T) -> Boolean, next: (It) -> Union2<T, Finished>): MutableList<T> {
    val out = mutableListOf<T>()
    while (true) {
        val __loop2_step = next(it)
        if (__loop2_step !is Union2.U1<*, *>) { break }
        val x = __loop2_step.value as T
        if (keep(x)) {
            out.add(x)
        }
    }
    return out
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, A> reduce(it: It, init: A, f: (A, T) -> A, next: (It) -> Union2<T, Finished>): A {
    var acc = init
    while (true) {
        val __loop3_step = next(it)
        if (__loop3_step !is Union2.U1<*, *>) { break }
        val x = __loop3_step.value as T
        acc = f(acc, x)
    }
    return acc
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<D, It, T, U> mapTo(dest: D, it: It, f: (T) -> U, add: (D, U) -> Unit, next: (It) -> Union2<T, Finished>): D {
    while (true) {
        val __loop4_step = next(it)
        if (__loop4_step !is Union2.U1<*, *>) { break }
        val x = __loop4_step.value as T
        add(dest, f(x))
    }
    return dest
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<D, It, T> filterTo(dest: D, it: It, keep: (T) -> Boolean, add: (D, T) -> Unit, copy: (T) -> T, next: (It) -> Union2<T, Finished>): D {
    while (true) {
        val __loop5_step = next(it)
        if (__loop5_step !is Union2.U1<*, *>) { break }
        val x = __loop5_step.value as T
        if (keep(x)) {
            add(dest, copy(x))
        }
    }
    return dest
}

fun<T> drop(value: T) {
}

data class Mapping<It, T, U>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var f: (T) -> U,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, U> next__13(m: Mapping<It, T, U>): Union2<U, Finished> {
    val step = m.step
    val x = step(m.src)
    if (x is Union2.U2<*, *>) {
        return Union2.U2<U, Finished>(finished())
    }
    val f = m.f
    val y: U = f((x.value as T))
    return Union2.U1<U, Finished>(emitted(y))
}

fun<It, T, U> mapping(it: It, f: (T) -> U, next: (It) -> Union2<T, Finished>): Mapping<It, T, U> {
    return Mapping(src = it, step = next, f = f)
}

data class Filtering<It, T>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var keep: (T) -> Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> next__14(t: Filtering<It, T>): Union2<T, Finished> {
    val step = t.step
    val keep = t.keep
    while (true) {
        val x = step(t.src)
        if (x is Union2.U2<*, *>) {
            return Union2.U2<T, Finished>(finished())
        }
        if (keep((x.value as T))) {
            return Union2.U1<T, Finished>(emitted((x.value as T)))
        }
    }
    return Union2.U2<T, Finished>(finished())
}

fun<It, T> filtering(it: It, keep: (T) -> Boolean, next: (It) -> Union2<T, Finished>): Filtering<It, T> {
    return Filtering(src = it, step = next, keep = keep)
}

data class Taking<It, T>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var left: Int,
)

fun<It, T> next__15(t: Taking<It, T>): Union2<T, Finished> {
    if (t.left <= 0) {
        return Union2.U2<T, Finished>(finished())
    }
    t.left = t.left - 1
    val step = t.step
    return step(t.src)
}

fun<It, T> taking(it: It, n: Int, next: (It) -> Union2<T, Finished>): Taking<It, T> {
    return Taking(src = it, step = next, left = n)
}

data class TakingWhile<It, T>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var keep: (T) -> Boolean,
    var done: Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> next__16(t: TakingWhile<It, T>): Union2<T, Finished> {
    if (t.done) {
        return Union2.U2<T, Finished>(finished())
    }
    val step = t.step
    val x = step(t.src)
    if (x is Union2.U2<*, *>) {
        t.done = true
        return Union2.U2<T, Finished>(finished())
    }
    val keep = t.keep
    if (keep((x.value as T))) {
        return Union2.U1<T, Finished>(emitted((x.value as T)))
    }
    t.done = true
    return Union2.U2<T, Finished>(finished())
}

fun<It, T> takingWhile(it: It, keep: (T) -> Boolean, next: (It) -> Union2<T, Finished>): TakingWhile<It, T> {
    return TakingWhile(src = it, step = next, keep = keep, done = false)
}

data class Skipping<It, T>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var left: Int,
)

fun<It, T> next__17(t: Skipping<It, T>): Union2<T, Finished> {
    val step = t.step
    while (t.left > 0) {
        t.left = t.left - 1
        val x = step(t.src)
        if (x is Union2.U2<*, *>) {
            return Union2.U2<T, Finished>(finished())
        }
    }
    return step(t.src)
}

fun<It, T> skipping(it: It, n: Int, next: (It) -> Union2<T, Finished>): Skipping<It, T> {
    return Skipping(src = it, step = next, left = n)
}

data class SkippingWhile<It, T>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var skip: (T) -> Boolean,
    var started: Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> next__18(t: SkippingWhile<It, T>): Union2<T, Finished> {
    val step = t.step
    if (t.started) {
        return step(t.src)
    }
    val skip = t.skip
    while (true) {
        val x = step(t.src)
        if (x is Union2.U2<*, *>) {
            return Union2.U2<T, Finished>(finished())
        }
        if (!skip((x.value as T))) {
            t.started = true
            return Union2.U1<T, Finished>(emitted((x.value as T)))
        }
    }
    return Union2.U2<T, Finished>(finished())
}

fun<It, T> skippingWhile(it: It, skip: (T) -> Boolean, next: (It) -> Union2<T, Finished>): SkippingWhile<It, T> {
    return SkippingWhile(src = it, step = next, skip = skip, started = false)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> toList(it: It, next: (It) -> Union2<T, Finished>): MutableList<T> {
    val out = mutableListOf<T>()
    while (true) {
        val __loop6_step = next(it)
        if (__loop6_step !is Union2.U1<*, *>) { break }
        val x = __loop6_step.value as T
        out.add(x)
    }
    return out
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> count__2(it: It, next: (It) -> Union2<T, Finished>): Int {
    var n = 0
    while (true) {
        val __loop7_step = next(it)
        if (__loop7_step !is Union2.U1<*, *>) { break }
        val _x = __loop7_step.value as T
        n = n + 1
    }
    return n
}
