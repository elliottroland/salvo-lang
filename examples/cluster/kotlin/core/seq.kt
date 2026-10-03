package salvo.core.seq

import salvo.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.deque.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.range.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
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

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
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

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
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

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<D, It, T, U> mapTo(dest: D, it: It, f: (T) -> U, add: (D, U) -> Unit, next: (It) -> Union2<T, Finished>): D {
    while (true) {
        val __loop4_step = next(it)
        if (__loop4_step !is Union2.U1<*, *>) { break }
        val x = __loop4_step.value as T
        add(dest, f(x))
    }
    return dest
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
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

data class Take<It, T>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var left: Int,
)

fun<It, T> next__13(t: Take<It, T>): Union2<T, Finished> {
    if (t.left <= 0) {
        return Union2.U2<T, Finished>(finished())
    }
    t.left = t.left - 1
    val step = t.step
    return step(t.src)
}

fun<It, T> take(it: It, n: Int, next: (It) -> Union2<T, Finished>): Take<It, T> {
    return Take(src = it, step = next, left = n)
}

data class TakeWhile<It, T>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var keep: (T) -> Boolean,
    var done: Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<It, T> next__14(t: TakeWhile<It, T>): Union2<T, Finished> {
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

fun<It, T> takeWhile(it: It, keep: (T) -> Boolean, next: (It) -> Union2<T, Finished>): TakeWhile<It, T> {
    return TakeWhile(src = it, step = next, keep = keep, done = false)
}

data class Skip<It, T>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var left: Int,
)

fun<It, T> next__15(t: Skip<It, T>): Union2<T, Finished> {
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

fun<It, T> skip(it: It, n: Int, next: (It) -> Union2<T, Finished>): Skip<It, T> {
    return Skip(src = it, step = next, left = n)
}

data class SkipWhile<It, T>(
    var src: It,
    var step: (It) -> Union2<T, Finished>,
    var skip: (T) -> Boolean,
    var started: Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<It, T> next__16(t: SkipWhile<It, T>): Union2<T, Finished> {
    val step = t.step
    if (t.started) {
        return step(t.src)
    }
    val passing = t.skip
    while (true) {
        val x = step(t.src)
        if (x is Union2.U2<*, *>) {
            return Union2.U2<T, Finished>(finished())
        }
        if (!passing((x.value as T))) {
            t.started = true
            return Union2.U1<T, Finished>(emitted((x.value as T)))
        }
    }
    return Union2.U2<T, Finished>(finished())
}

fun<It, T> skipWhile(it: It, skip: (T) -> Boolean, next: (It) -> Union2<T, Finished>): SkipWhile<It, T> {
    return SkipWhile(src = it, step = next, skip = skip, started = false)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun<It, T> collect(it: It, next: (It) -> Union2<T, Finished>): MutableList<T> {
    val out = mutableListOf<T>()
    while (true) {
        val __loop6_step = next(it)
        if (__loop6_step !is Union2.U1<*, *>) { break }
        val x = __loop6_step.value as T
        out.add(x)
    }
    return out
}
