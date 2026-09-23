package salvo.core.string

import salvo.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*

fun iter__9(str: String): StrYield {
    return StrYield(text = str, at = 0)
}

data class StrYield(
    var text: String,
    var at: Int,
)

fun next__12(p: StrYield): Union2<Char, Finished> {
    val chr = p.text.getOrNull(p.at)
    if (chr == null) {
        return U2_2<Char, Finished>(finished())
    }
    p.at = p.at + 1
    return U2_1<Char, Finished>(emitted(chr))
}

data class Span(
    val start: Int,
    val end: Int,
)

fun SpanOf_qualifies(span: Span, str: String): Boolean {
    return span.start >= 0 && span.start <= span.end && span.end <= str.length
}

fun substr(str: String, at: Span): String {
    return (run { val __s = str; val __i = at.start; val __j = at.end; if (__i >= 0 && __j >= __i && __j <= __s.length) __s.substring(__i, __j) else null } ?: throw AssertionError("salvo: value is absent at core.string:116:12"))
}
