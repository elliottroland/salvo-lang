package salvo.std.core.seq

import salvo.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, U, E> map__It_Fn(e: E, it: It, f: (E, T) -> U, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): salvo.platform.std.core.list.MutList<U> {
    val out: salvo.platform.std.core.list.MutList<U> = mutableListOf<U>()
    while (true) {
        val __step_2: Union2<T, salvo.std.core.iterator.Finished> = next(e, it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                salvo.std.core.list.addPlatform(out, f(e, x))
            }
            else -> {
                break
            }
        }
    }
    return out
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, E> filter(e: E, it: It, keep: (E, T) -> Boolean, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): salvo.platform.std.core.list.MutList<T> {
    val out: salvo.platform.std.core.list.MutList<T> = mutableListOf<T>()
    while (true) {
        val __step_2: Union2<T, salvo.std.core.iterator.Finished> = next(e, it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                if (keep(e, x)) {
                    salvo.std.core.list.addPlatform(out, x)
                }
            }
            else -> {
                break
            }
        }
    }
    return out
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, A, E> reduce__It_A_Fn(e: E, it: It, init: A, f: (E, A, T) -> A, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): A {
    var acc: A = init
    while (true) {
        val __step_2: Union2<T, salvo.std.core.iterator.Finished> = next(e, it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                acc = f(e, acc, x)
            }
            else -> {
                break
            }
        }
    }
    return acc
}

fun<T, U, E> map__List_Fn(e: E, list: List<T>, f: (E, T) -> U): salvo.platform.std.core.list.MutList<U> {
    val out: salvo.platform.std.core.list.MutList<U> = mutableListOf<U>()
    for (x in salvo.platform.std.core.list.each(list)) {
        salvo.std.core.list.addPlatform(out, f(e, x))
    }
    return out
}

fun<T, E> filterPlatform(e: E, list: List<T>, keep: (E, T) -> Boolean): salvo.platform.std.core.list.MutList<T> = salvo.platform.std.core.seq.filter(list, { __x0 -> keep(e, __x0) })

fun<T, A, E> reduce__List_A_Fn(e: E, list: List<T>, init: A, f: (E, A, T) -> A): A {
    var acc: A = init
    for (x in salvo.platform.std.core.list.each(list)) {
        acc = f(e, acc, x)
    }
    return acc
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<D, It, T, U> mapTo(dest: D, it: It, f: (T) -> U, add: (D, U) -> Unit, next: (It) -> Union2<T, salvo.std.core.iterator.Finished>): D {
    while (true) {
        val __step_2: Union2<T, salvo.std.core.iterator.Finished> = next(it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                add(dest, f(x))
            }
            else -> {
                break
            }
        }
    }
    return dest
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<D, It, T> filterTo(dest: D, it: It, keep: (T) -> Boolean, add: (D, T) -> Unit, copy: (T) -> T, next: (It) -> Union2<T, salvo.std.core.iterator.Finished>): D {
    while (true) {
        val __step_2: Union2<T, salvo.std.core.iterator.Finished> = next(it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                if (keep(x)) {
                    add(dest, copy(x))
                }
            }
            else -> {
                break
            }
        }
    }
    return dest
}

fun<T> drop(value: T) {
}

data class Mapping<It, T, U, E>(
    var src: It,
    var step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>,
    var f: (E, T) -> U,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, U, E> next__Mapping(e: E, m: Mapping<It, T, U, E>): Union2<U, salvo.std.core.iterator.Finished> {
    val step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished> = m.step
    val x: Union2<T, salvo.std.core.iterator.Finished> = step(e, m.src)
    if ((x is Union2.U2<*, *>)) {
        val x_1: salvo.std.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.std.core.iterator.Finished)
        return Union2.U2<U, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val f: (E, T) -> U = m.f
    val x_2: T = ((x as Union2.U1<*, *>).value as T)
    val y: U = f(e, x_2)
    return Union2.U1<U, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(y))
}

fun<It, T, U, E> mapping(it: It, f: (E, T) -> U, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): Mapping<It, T, U, E> {
    return Mapping<It, T, U, E>(src = it, step = next, f = f)
}

fun<It, T, U, E> detach__Mapping(it: Mapping<It, T, U, E>): It {
    return it.src
}

data class Filtering<It, T, E>(
    var src: It,
    var step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>,
    var keep: (E, T) -> Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, E> next__Filtering(e: E, t: Filtering<It, T, E>): Union2<T, salvo.std.core.iterator.Finished> {
    val step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished> = t.step
    val keep: (E, T) -> Boolean = t.keep
    while (true) {
        val x: Union2<T, salvo.std.core.iterator.Finished> = step(e, t.src)
        if ((x is Union2.U2<*, *>)) {
            val x_1: salvo.std.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.std.core.iterator.Finished)
            return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
        }
        val x_2: T = ((x as Union2.U1<*, *>).value as T)
        if (keep(e, x_2)) {
            return Union2.U1<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(x_2))
        }
    }
    throw AssertionError("salvo: unreachable at std.core.seq:323:5")
    return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
}

fun<It, T, E> filtering(it: It, keep: (E, T) -> Boolean, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): Filtering<It, T, E> {
    return Filtering<It, T, E>(src = it, step = next, keep = keep)
}

fun<It, T, E> detach__Filtering(it: Filtering<It, T, E>): It {
    return it.src
}

data class Taking<It, T, E>(
    var src: It,
    var step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>,
    var left: Int,
)

fun<It, T, E> next__Taking(e: E, t: Taking<It, T, E>): Union2<T, salvo.std.core.iterator.Finished> {
    if ((t.left <= 0)) {
        return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    t.left = (t.left - 1)
    val step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished> = t.step
    return step(e, t.src)
}

fun<It, T, E> taking(it: It, n: Int, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): Taking<It, T, E> {
    return Taking<It, T, E>(src = it, step = next, left = n)
}

fun<It, T, E> detach__Taking(it: Taking<It, T, E>): It {
    return it.src
}

data class TakingWhile<It, T, E>(
    var src: It,
    var step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>,
    var keep: (E, T) -> Boolean,
    var done: Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, E> next__TakingWhile(e: E, t: TakingWhile<It, T, E>): Union2<T, salvo.std.core.iterator.Finished> {
    if (t.done) {
        return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished> = t.step
    val x: Union2<T, salvo.std.core.iterator.Finished> = step(e, t.src)
    if ((x is Union2.U2<*, *>)) {
        val x_1: salvo.std.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.std.core.iterator.Finished)
        t.done = true
        return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    val keep: (E, T) -> Boolean = t.keep
    val x_2: T = ((x as Union2.U1<*, *>).value as T)
    if (keep(e, x_2)) {
        return Union2.U1<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(x_2))
    }
    t.done = true
    return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
}

fun<It, T, E> takingWhile(it: It, keep: (E, T) -> Boolean, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): TakingWhile<It, T, E> {
    return TakingWhile<It, T, E>(src = it, step = next, keep = keep, done = false)
}

fun<It, T, E> detach__TakingWhile(it: TakingWhile<It, T, E>): It {
    return it.src
}

data class Skipping<It, T, E>(
    var src: It,
    var step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>,
    var left: Int,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, E> next__Skipping(e: E, t: Skipping<It, T, E>): Union2<T, salvo.std.core.iterator.Finished> {
    val step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished> = t.step
    while (true) {
        if (!((t.left > 0))) {
            break
        }
        t.left = (t.left - 1)
        val x: Union2<T, salvo.std.core.iterator.Finished> = step(e, t.src)
        if ((x is Union2.U2<*, *>)) {
            val x_1: salvo.std.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.std.core.iterator.Finished)
            return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
        }
    }
    return step(e, t.src)
}

fun<It, T, E> skipping(it: It, n: Int, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): Skipping<It, T, E> {
    return Skipping<It, T, E>(src = it, step = next, left = n)
}

fun<It, T, E> detach__Skipping(it: Skipping<It, T, E>): It {
    return it.src
}

data class SkippingWhile<It, T, E>(
    var src: It,
    var step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>,
    var skip: (E, T) -> Boolean,
    var started: Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, E> next__SkippingWhile(e: E, t: SkippingWhile<It, T, E>): Union2<T, salvo.std.core.iterator.Finished> {
    val step: (E, It) -> Union2<T, salvo.std.core.iterator.Finished> = t.step
    if (t.started) {
        return step(e, t.src)
    }
    val skip: (E, T) -> Boolean = t.skip
    while (true) {
        val x: Union2<T, salvo.std.core.iterator.Finished> = step(e, t.src)
        if ((x is Union2.U2<*, *>)) {
            val x_1: salvo.std.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.std.core.iterator.Finished)
            return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
        }
        val x_2: T = ((x as Union2.U1<*, *>).value as T)
        if (!(skip(e, x_2))) {
            t.started = true
            val x_3: T = ((x as Union2.U1<*, *>).value as T)
            return Union2.U1<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(x_3))
        }
    }
    throw AssertionError("salvo: unreachable at std.core.seq:546:5")
    return Union2.U2<T, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
}

fun<It, T, E> skippingWhile(it: It, skip: (E, T) -> Boolean, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): SkippingWhile<It, T, E> {
    return SkippingWhile<It, T, E>(src = it, step = next, skip = skip, started = false)
}

fun<It, T, E> detach__SkippingWhile(it: SkippingWhile<It, T, E>): It {
    return it.src
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, E> toList(e: E, it: It, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): salvo.platform.std.core.list.MutList<T> {
    val out: salvo.platform.std.core.list.MutList<T> = mutableListOf<T>()
    while (true) {
        val __step_2: Union2<T, salvo.std.core.iterator.Finished> = next(e, it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                salvo.std.core.list.addPlatform(out, x)
            }
            else -> {
                break
            }
        }
    }
    return out
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, E> count(e: E, it: It, next: (E, It) -> Union2<T, salvo.std.core.iterator.Finished>): Int {
    var n: Int = 0
    while (true) {
        val __step_2: Union2<T, salvo.std.core.iterator.Finished> = next(e, it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val _x: T = __emitted_3
                n = (n + 1)
            }
            else -> {
                break
            }
        }
    }
    return n
}

