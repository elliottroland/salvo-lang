package salvo.std.core.result

import salvo.*

fun<T> ok(value: T): T {
    return value
}

fun<T> err(value: T): T {
    return value
}

fun<T> Ok(value: T): T {
    return value
}

fun<T> Err(value: T): T {
    return value
}

fun<E> errs(first: E): List<E> {
    return listOf<E>(first)
}

fun<T> Errs(value: T): T {
    return value
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<A, B, E> both(a: Union2<A, List<E>>, b: Union2<B, List<E>>, copy: (E) -> E): Union2<kotlin.Pair<A, B>, List<E>> {
    return when {
        (a is Union2.U1<*, *>) -> {
            val a_1: A = ((a as Union2.U1<*, *>).value as A)
            when {
                (b is Union2.U1<*, *>) -> {
                    val b_2: B = ((b as Union2.U1<*, *>).value as B)
                    return Union2.U1<kotlin.Pair<A, B>, List<E>>(salvo.std.core.result.Ok(kotlin.Pair(a_1, b_2)))
                }
                (b is Union2.U2<*, *>) -> {
                    val b_3: List<E> = ((b as Union2.U2<*, *>).value as List<E>)
                    return Union2.U2<kotlin.Pair<A, B>, List<E>>(salvo.std.core.result.Errs(b_3))
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.core.result:70:13"))
                }
            }
        }
        (a is Union2.U2<*, *>) -> {
            val a_4: List<E> = ((a as Union2.U2<*, *>).value as List<E>)
            when {
                (b is Union2.U1<*, *>) -> {
                    val b_5: B = ((b as Union2.U1<*, *>).value as B)
                    return Union2.U2<kotlin.Pair<A, B>, List<E>>(salvo.std.core.result.Errs(a_4))
                }
                (b is Union2.U2<*, *>) -> {
                    val b_6: List<E> = ((b as Union2.U2<*, *>).value as List<E>)
                    return Union2.U2<kotlin.Pair<A, B>, List<E>>(salvo.std.core.result.Errs(salvo.std.core.result.joined(a_4, b_6, copy)))
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.core.result:76:13"))
                }
            }
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.core.result:68:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<A, B, C, E> map2(a: Union2<A, List<E>>, b: Union2<B, List<E>>, f: (A, B) -> C, copy: (E) -> E): Union2<C, List<E>> {
    return when {
        (a is Union2.U1<*, *>) -> {
            val a_1: A = ((a as Union2.U1<*, *>).value as A)
            when {
                (b is Union2.U1<*, *>) -> {
                    val b_2: B = ((b as Union2.U1<*, *>).value as B)
                    return Union2.U1<C, List<E>>(salvo.std.core.result.Ok(f(a_1, b_2)))
                }
                (b is Union2.U2<*, *>) -> {
                    val b_3: List<E> = ((b as Union2.U2<*, *>).value as List<E>)
                    return Union2.U2<C, List<E>>(salvo.std.core.result.Errs(b_3))
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.core.result:90:13"))
                }
            }
        }
        (a is Union2.U2<*, *>) -> {
            val a_4: List<E> = ((a as Union2.U2<*, *>).value as List<E>)
            when {
                (b is Union2.U1<*, *>) -> {
                    val b_5: B = ((b as Union2.U1<*, *>).value as B)
                    return Union2.U2<C, List<E>>(salvo.std.core.result.Errs(a_4))
                }
                (b is Union2.U2<*, *>) -> {
                    val b_6: List<E> = ((b as Union2.U2<*, *>).value as List<E>)
                    return Union2.U2<C, List<E>>(salvo.std.core.result.Errs(salvo.std.core.result.joined(a_4, b_6, copy)))
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.core.result:96:13"))
                }
            }
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at std.core.result:88:5"))
        }
    }
}

fun<X> joined(a: List<X>, b: List<X>, copy: (X) -> X): List<X> {
    val errors: salvo.platform.std.core.list.MutList<X> = mutableListOf<X>()
    for (e in salvo.platform.std.core.list.each(a)) {
        salvo.std.core.list.addPlatform(errors, copy(e))
    }
    for (e in salvo.platform.std.core.list.each(b)) {
        salvo.std.core.list.addPlatform(errors, copy(e))
    }
    if (salvo.std.core.list.NonEmpty_qualifies(errors)) {
        return errors
    }
    return throw AssertionError(("salvo: " + ("two non-empty lists joined are not empty") + " at std.core.result:118:5"))
}

