package salvo.core.string

import salvo.*
import salvo.core.array.*
import salvo.core.bytes.*
import salvo.core.deque.*
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

fun next__20(p: StrYield): Union2<Char, Finished> {
    val chr = p.text.getOrNull(p.at)
    if (chr == null) {
        return Union2.U2<Char, Finished>(finished())
    }
    p.at = p.at + 1
    return Union2.U1<Char, Finished>(emitted(chr))
}

data class Span(
    val start: Int,
    val end: Int,
)

object __Codec_Span : salvo.WireCodec<Span> {
    override fun enc(v: Span, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.start, out)
        salvo.IntCodec.enc(v.end, out)
    }
    override fun dec(inp: salvo.WireIn): Span = Span(salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun SpanOf_qualifies(span: Span, str: String): Boolean {
    return span.start >= 0 && span.start <= span.end && span.end <= str.length
}

fun substr(str: String, at: Span): String {
    return (run { val __s = str; val __i = at.start; val __j = at.end; if (__i >= 0 && __j >= __i && __j <= __s.length) __s.substring(__i, __j) else null } ?: throw AssertionError("salvo: value is absent at core.string:134:12"))
}

fun isEmpty__2(str: String): Boolean {
    return str.length == 0
}

fun repeat(str: String, n: Int): String {
    val out = StringBuilder()
    var i = 0
    while (i < n) {
        out.append(str)
        i = i + 1
    }
    return out.toString()
}

fun lines(str: String): MutableList<String> {
    val parts = str.split("\n").toMutableList()
    if (parts.size > 1 && str.endsWith("\n")) {
        val _end = removeBack(parts, 1)
    }
    val out = mutableListOf<String>()
    for (p in parts) {
        out.add(p.removeSuffix("\r"))
    }
    return out
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun splitOnce(str: String, sep: String): Pair<String, String>? {
    val at = str.indexOf(sep).takeIf { it >= 0 }
    if (at != null) {
        val i = at as Int
        val before = (run { val __s = str; val __i = 0; val __j = i; if (__i >= 0 && __j >= __i && __j <= __s.length) __s.substring(__i, __j) else null } ?: "")
        val after = (run { val __s = str; val __i = i + sep.length; val __j = str.length; if (__i >= 0 && __j >= __i && __j <= __s.length) __s.substring(__i, __j) else null } ?: "")
        return Pair(before, after)
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun splitLast(str: String, sep: String): Pair<String, String>? {
    val at = str.lastIndexOf(sep).takeIf { it >= 0 }
    if (at != null) {
        val i = at as Int
        val before = (run { val __s = str; val __i = 0; val __j = i; if (__i >= 0 && __j >= __i && __j <= __s.length) __s.substring(__i, __j) else null } ?: "")
        val after = (run { val __s = str; val __i = i + sep.length; val __j = str.length; if (__i >= 0 && __j >= __i && __j <= __s.length) __s.substring(__i, __j) else null } ?: "")
        return Pair(before, after)
    }
    return null
}
