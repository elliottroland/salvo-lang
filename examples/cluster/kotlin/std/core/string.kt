package salvo.std.core.string

import salvo.*

fun Mut_Str(parts: Array<String>): salvo.platform.std.core.string.MutStr {
    val out: salvo.platform.std.core.string.MutStr = salvo.std.core.string.emptyStrPlatform()
    for (part in parts) {
        salvo.std.core.string.appendPlatform(out, part)
    }
    return out
}

fun emptyStrPlatform(): salvo.platform.std.core.string.MutStr = salvo.platform.std.core.string.emptyStr()

fun sizePlatform(str: String): Int = salvo.platform.std.core.string.size(str)

fun byteSizePlatform(str: String): Long = salvo.platform.std.core.string.byteSize(str)

fun charAtPlatform(str: String, index: Int): Char? = salvo.platform.std.core.string.charAt(str, index)

fun iter(str: String): StrYield {
    return StrYield(text = str, at = 0)
}

data class StrYield(
    var text: String,
    var at: Int,
)

fun next(p: StrYield): Union2<Char, salvo.std.core.iterator.Finished> {
    val chr: Char? = salvo.std.core.string.charAtPlatform(p.text, p.at)
    if ((chr == null)) {
        return Union2.U2<Char, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.finished())
    }
    p.at = (p.at + 1)
    val chr_1: Char = chr!!
    return Union2.U1<Char, salvo.std.core.iterator.Finished>(salvo.std.core.iterator.emitted(chr_1))
}

fun splitPlatform(str: String, sep: String): salvo.platform.std.core.list.MutList<String> = salvo.platform.std.core.string.split(str, sep)

fun indexOfPlatform(str: String, needle: String): Int? = salvo.platform.std.core.string.indexOf(str, needle)

fun indexOf(str: String, needle: String, from: Int): Int? {
    return salvo.std.core.string.indexOfFromPlatform(str, needle, from)
}

fun indexOfFromPlatform(str: String, needle: String, from: Int): Int? = salvo.platform.std.core.string.indexOfFrom(str, needle, from)

fun lastIndexOfPlatform(str: String, needle: String): Int? = salvo.platform.std.core.string.lastIndexOf(str, needle)

fun replacePlatform(str: String, from: String, to: String): String = salvo.platform.std.core.string.replace(str, from, to)

fun trimStartPlatform(str: String): String = salvo.platform.std.core.string.trimStart(str)

fun trimEndPlatform(str: String): String = salvo.platform.std.core.string.trimEnd(str)

fun containsPlatform(str: String, needle: String): Boolean = salvo.platform.std.core.string.contains(str, needle)

fun startsWithPlatform(str: String, prefix: String): Boolean = salvo.platform.std.core.string.startsWith(str, prefix)

fun endsWithPlatform(str: String, suffix: String): Boolean = salvo.platform.std.core.string.endsWith(str, suffix)

fun trimPlatform(str: String): String = salvo.platform.std.core.string.trim(str)

fun trimPrefixPlatform(str: String, prefix: String): String = salvo.platform.std.core.string.trimPrefix(str, prefix)

fun trimSuffixPlatform(str: String, suffix: String): String = salvo.platform.std.core.string.trimSuffix(str, suffix)

fun substrPlatform(str: String, start: Int, end: Int): String? = salvo.platform.std.core.string.substr(str, start, end)

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
    return (((span.start >= 0) && (span.start <= span.end)) && (span.end <= salvo.std.core.string.sizePlatform(str)))
}

fun substr(str: String, at: Span): String {
    return kotlin.run {
        val __pick_1: String? = salvo.std.core.string.substrPlatform(str, at.start, at.end)
        (if ((__pick_1 != null)) {
            val __picked_2: String = __pick_1!!
            __picked_2
        } else {
            throw AssertionError("salvo: unreachable at std.core.string:150:45")
        })
    }
}

fun toUpperPlatform(str: String): String = salvo.platform.std.core.string.toUpper(str)

fun toLowerPlatform(str: String): String = salvo.platform.std.core.string.toLower(str)

fun joinPlatform(parts: List<String>, sep: String): String = salvo.platform.std.core.string.join(parts, sep)

fun parseIntPlatform(str: String): Int? = salvo.platform.std.core.string.parseInt(str)

fun parseLongPlatform(str: String): Long? = salvo.platform.std.core.string.parseLong(str)

fun parseDoublePlatform(str: String): Double? = salvo.platform.std.core.string.parseDouble(str)

fun parseBool(str: String): Boolean? {
    if (((str) == ("true"))) {
        return true
    }
    if (((str) == ("false"))) {
        return false
    }
    return null
}

fun fromStr__Str_StrOpt(s: String): String? {
    return s
}

fun fromStr__Str_IntOpt(s: String): Int? {
    return salvo.std.core.string.parseIntPlatform(s)
}

fun fromStr__Str_LongOpt(s: String): Long? {
    return salvo.std.core.string.parseLongPlatform(s)
}

fun fromStr__Str_DoubleOpt(s: String): Double? {
    return salvo.std.core.string.parseDoublePlatform(s)
}

fun fromStr__Str_BoolOpt(s: String): Boolean? {
    return salvo.std.core.string.parseBool(s)
}

fun appendPlatform(str: salvo.platform.std.core.string.MutStr, text: String) = salvo.platform.std.core.string.append(str, text)

fun setPlatform(str: salvo.platform.std.core.string.MutStr, index: Int, chr: Char): Boolean = salvo.platform.std.core.string.set(str, index, chr)

fun clearPlatform(str: salvo.platform.std.core.string.MutStr) = salvo.platform.std.core.string.clear(str)

fun isEmpty(str: String): Boolean {
    return (salvo.std.core.string.sizePlatform(str) == 0)
}

fun repeat(str: String, n: Int): String {
    val out: salvo.platform.std.core.string.MutStr = salvo.std.core.string.Mut_Str(arrayOf<String>())
    var i: Int = 0
    while (true) {
        if (!((i < n))) {
            break
        }
        salvo.std.core.string.appendPlatform(out, str)
        i = (i + 1)
    }
    return out.toString()
}

fun lines(str: String): salvo.platform.std.core.list.MutList<String> {
    val parts: salvo.platform.std.core.list.MutList<String> = salvo.std.core.string.splitPlatform(str, "\n")
    if (((salvo.std.core.list.sizePlatform(parts) > 1) && salvo.std.core.string.endsWithPlatform(str, "\n"))) {
        val _end: salvo.platform.std.core.list.MutList<String> = salvo.std.core.list.removeBack(parts, 1)
    }
    val out: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    for (p in salvo.platform.std.core.list.each(parts)) {
        salvo.std.core.list.addPlatform(out, salvo.std.core.string.trimSuffixPlatform(p, "\r"))
    }
    return out
}

fun splitOnce(str: String, sep: String): kotlin.Pair<String, String>? {
    val at: Int? = salvo.std.core.string.indexOfPlatform(str, sep)
    if ((at != null)) {
        val i: Int = at!!
        val before: String = kotlin.run {
            val __pick_1: String? = salvo.std.core.string.substrPlatform(str, 0, i)
            (if ((__pick_1 != null)) {
                val __picked_2: String = __pick_1!!
                __picked_2
            } else {
                ""
            })
        }
        val after: String = kotlin.run {
            val __pick_4: String? = salvo.std.core.string.substrPlatform(str, (i + salvo.std.core.string.sizePlatform(sep)), salvo.std.core.string.sizePlatform(str))
            (if ((__pick_4 != null)) {
                val __picked_5: String = __pick_4!!
                __picked_5
            } else {
                ""
            })
        }
        return kotlin.Pair(before, after)
    }
    return null
}

fun splitLast(str: String, sep: String): kotlin.Pair<String, String>? {
    val at: Int? = salvo.std.core.string.lastIndexOfPlatform(str, sep)
    if ((at != null)) {
        val i: Int = at!!
        val before: String = kotlin.run {
            val __pick_1: String? = salvo.std.core.string.substrPlatform(str, 0, i)
            (if ((__pick_1 != null)) {
                val __picked_2: String = __pick_1!!
                __picked_2
            } else {
                ""
            })
        }
        val after: String = kotlin.run {
            val __pick_4: String? = salvo.std.core.string.substrPlatform(str, (i + salvo.std.core.string.sizePlatform(sep)), salvo.std.core.string.sizePlatform(str))
            (if ((__pick_4 != null)) {
                val __picked_5: String = __pick_4!!
                __picked_5
            } else {
                ""
            })
        }
        return kotlin.Pair(before, after)
    }
    return null
}

