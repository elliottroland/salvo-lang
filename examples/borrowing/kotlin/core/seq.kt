package salvo.core.seq

import salvo.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, U> map__It_Fn(it: It, f: (T) -> U, next: (It) -> Union2<T, salvo.core.iterator.Finished>): salvo.platform.core.list.MutList<U> {
    val out: salvo.platform.core.list.MutList<U> = mutableListOf<U>()
    while (true) {
        val __step_2: Union2<T, salvo.core.iterator.Finished> = next(it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                salvo.core.list.addPlatform(out, f(x))
            }
            else -> {
                break
            }
        }
    }
    return out
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> filter(it: It, keep: (T) -> Boolean, next: (It) -> Union2<T, salvo.core.iterator.Finished>): salvo.platform.core.list.MutList<T> {
    val out: salvo.platform.core.list.MutList<T> = mutableListOf<T>()
    while (true) {
        val __step_2: Union2<T, salvo.core.iterator.Finished> = next(it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                if (keep(x)) {
                    salvo.core.list.addPlatform(out, x)
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
fun<It, T, A> reduce__It_A_Fn(it: It, init: A, f: (A, T) -> A, next: (It) -> Union2<T, salvo.core.iterator.Finished>): A {
    var acc: A = init
    while (true) {
        val __step_2: Union2<T, salvo.core.iterator.Finished> = next(it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                acc = f(acc, x)
            }
            else -> {
                break
            }
        }
    }
    return acc
}

fun<T, U> map__List_Fn(list: List<T>, f: (T) -> U): salvo.platform.core.list.MutList<U> {
    val out: salvo.platform.core.list.MutList<U> = mutableListOf<U>()
    for (x in salvo.platform.core.list.each(list)) {
        salvo.core.list.addPlatform(out, f(x))
    }
    return out
}

fun<T> filterPlatform(list: List<T>, keep: (T) -> Boolean): salvo.platform.core.list.MutList<T> = salvo.platform.core.seq.filter(list, keep)

fun<T, A> reduce__List_A_Fn(list: List<T>, init: A, f: (A, T) -> A): A {
    var acc: A = init
    for (x in salvo.platform.core.list.each(list)) {
        acc = f(acc, x)
    }
    return acc
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<D, It, T, U> mapTo(dest: D, it: It, f: (T) -> U, add: (D, U) -> Unit, next: (It) -> Union2<T, salvo.core.iterator.Finished>): D {
    while (true) {
        val __step_2: Union2<T, salvo.core.iterator.Finished> = next(it)
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
fun<D, It, T> filterTo(dest: D, it: It, keep: (T) -> Boolean, add: (D, T) -> Unit, copy: (T) -> T, next: (It) -> Union2<T, salvo.core.iterator.Finished>): D {
    while (true) {
        val __step_2: Union2<T, salvo.core.iterator.Finished> = next(it)
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

data class Mapping<It, T, U>(
    var src: It,
    var step: (It) -> Union2<T, salvo.core.iterator.Finished>,
    var f: (T) -> U,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T, U> next__Mapping(m: Mapping<It, T, U>): Union2<U, salvo.core.iterator.Finished> {
    val step: (It) -> Union2<T, salvo.core.iterator.Finished> = m.step
    val x: Union2<T, salvo.core.iterator.Finished> = step(m.src)
    if ((x is Union2.U2<*, *>)) {
        val x_1: salvo.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.core.iterator.Finished)
        return Union2.U2<U, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val f: (T) -> U = m.f
    val x_2: T = ((x as Union2.U1<*, *>).value as T)
    val y: U = f(x_2)
    return Union2.U1<U, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(y))
}

fun<It, T, U> mapping(it: It, f: (T) -> U, next: (It) -> Union2<T, salvo.core.iterator.Finished>): Mapping<It, T, U> {
    return Mapping<It, T, U>(src = it, step = next, f = f)
}

data class Filtering<It, T>(
    var src: It,
    var step: (It) -> Union2<T, salvo.core.iterator.Finished>,
    var keep: (T) -> Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> next__Filtering(t: Filtering<It, T>): Union2<T, salvo.core.iterator.Finished> {
    val step: (It) -> Union2<T, salvo.core.iterator.Finished> = t.step
    val keep: (T) -> Boolean = t.keep
    while (true) {
        if (!(true)) {
            break
        }
        val x: Union2<T, salvo.core.iterator.Finished> = step(t.src)
        if ((x is Union2.U2<*, *>)) {
            val x_1: salvo.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.core.iterator.Finished)
            return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
        }
        val x_2: T = ((x as Union2.U1<*, *>).value as T)
        if (keep(x_2)) {
            return Union2.U1<T, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(x_2))
        }
    }
    return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
}

fun<It, T> filtering(it: It, keep: (T) -> Boolean, next: (It) -> Union2<T, salvo.core.iterator.Finished>): Filtering<It, T> {
    return Filtering<It, T>(src = it, step = next, keep = keep)
}

data class Taking<It, T>(
    var src: It,
    var step: (It) -> Union2<T, salvo.core.iterator.Finished>,
    var left: Int,
)

fun<It, T> next__Taking(t: Taking<It, T>): Union2<T, salvo.core.iterator.Finished> {
    if ((t.left <= 0)) {
        return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    t.left = (t.left - 1)
    val step: (It) -> Union2<T, salvo.core.iterator.Finished> = t.step
    return step(t.src)
}

fun<It, T> taking(it: It, n: Int, next: (It) -> Union2<T, salvo.core.iterator.Finished>): Taking<It, T> {
    return Taking<It, T>(src = it, step = next, left = n)
}

data class TakingWhile<It, T>(
    var src: It,
    var step: (It) -> Union2<T, salvo.core.iterator.Finished>,
    var keep: (T) -> Boolean,
    var done: Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> next__TakingWhile(t: TakingWhile<It, T>): Union2<T, salvo.core.iterator.Finished> {
    if (t.done) {
        return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val step: (It) -> Union2<T, salvo.core.iterator.Finished> = t.step
    val x: Union2<T, salvo.core.iterator.Finished> = step(t.src)
    if ((x is Union2.U2<*, *>)) {
        val x_1: salvo.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.core.iterator.Finished)
        t.done = true
        return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
    }
    val keep: (T) -> Boolean = t.keep
    val x_2: T = ((x as Union2.U1<*, *>).value as T)
    if (keep(x_2)) {
        return Union2.U1<T, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(x_2))
    }
    t.done = true
    return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
}

fun<It, T> takingWhile(it: It, keep: (T) -> Boolean, next: (It) -> Union2<T, salvo.core.iterator.Finished>): TakingWhile<It, T> {
    return TakingWhile<It, T>(src = it, step = next, keep = keep, done = false)
}

data class Skipping<It, T>(
    var src: It,
    var step: (It) -> Union2<T, salvo.core.iterator.Finished>,
    var left: Int,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> next__Skipping(t: Skipping<It, T>): Union2<T, salvo.core.iterator.Finished> {
    val step: (It) -> Union2<T, salvo.core.iterator.Finished> = t.step
    while (true) {
        if (!((t.left > 0))) {
            break
        }
        t.left = (t.left - 1)
        val x: Union2<T, salvo.core.iterator.Finished> = step(t.src)
        if ((x is Union2.U2<*, *>)) {
            val x_1: salvo.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.core.iterator.Finished)
            return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
        }
    }
    return step(t.src)
}

fun<It, T> skipping(it: It, n: Int, next: (It) -> Union2<T, salvo.core.iterator.Finished>): Skipping<It, T> {
    return Skipping<It, T>(src = it, step = next, left = n)
}

data class SkippingWhile<It, T>(
    var src: It,
    var step: (It) -> Union2<T, salvo.core.iterator.Finished>,
    var skip: (T) -> Boolean,
    var started: Boolean,
)

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> next__SkippingWhile(t: SkippingWhile<It, T>): Union2<T, salvo.core.iterator.Finished> {
    val step: (It) -> Union2<T, salvo.core.iterator.Finished> = t.step
    if (t.started) {
        return step(t.src)
    }
    val skip: (T) -> Boolean = t.skip
    while (true) {
        if (!(true)) {
            break
        }
        val x: Union2<T, salvo.core.iterator.Finished> = step(t.src)
        if ((x is Union2.U2<*, *>)) {
            val x_1: salvo.core.iterator.Finished = ((x as Union2.U2<*, *>).value as salvo.core.iterator.Finished)
            return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
        }
        val x_2: T = ((x as Union2.U1<*, *>).value as T)
        if (!(skip(x_2))) {
            t.started = true
            val x_3: T = ((x as Union2.U1<*, *>).value as T)
            return Union2.U1<T, salvo.core.iterator.Finished>(salvo.core.iterator.emitted(x_3))
        }
    }
    return Union2.U2<T, salvo.core.iterator.Finished>(salvo.core.iterator.finished())
}

fun<It, T> skippingWhile(it: It, skip: (T) -> Boolean, next: (It) -> Union2<T, salvo.core.iterator.Finished>): SkippingWhile<It, T> {
    return SkippingWhile<It, T>(src = it, step = next, skip = skip, started = false)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> toList(it: It, next: (It) -> Union2<T, salvo.core.iterator.Finished>): salvo.platform.core.list.MutList<T> {
    val out: salvo.platform.core.list.MutList<T> = mutableListOf<T>()
    while (true) {
        val __step_2: Union2<T, salvo.core.iterator.Finished> = next(it)
        when {
            (__step_2 is Union2.U1<*, *>) -> {
                val __emitted_3: T = ((__step_2 as Union2.U1<*, *>).value as T)
                val x: T = __emitted_3
                salvo.core.list.addPlatform(out, x)
            }
            else -> {
                break
            }
        }
    }
    return out
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<It, T> count(it: It, next: (It) -> Union2<T, salvo.core.iterator.Finished>): Int {
    var n: Int = 0
    while (true) {
        val __step_2: Union2<T, salvo.core.iterator.Finished> = next(it)
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

