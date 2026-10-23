package salvo.args.lex

import salvo.*

data class LongToken(
    val name: String,
    val value: String?,
)

object __Codec_LongToken : salvo.WireCodec<LongToken> {
    override fun enc(v: LongToken, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.name, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): LongToken = LongToken(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class ShortToken(
    val letters: String,
)

object __Codec_ShortToken : salvo.WireCodec<ShortToken> {
    override fun enc(v: ShortToken, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.letters, out)
    }
    override fun dec(inp: salvo.WireIn): ShortToken = ShortToken(salvo.StrCodec.dec(inp))
}

data class WordToken(
    val value: String,
)

object __Codec_WordToken : salvo.WireCodec<WordToken> {
    override fun enc(v: WordToken, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): WordToken = WordToken(salvo.StrCodec.dec(inp))
}

class EndToken

object __Codec_EndToken : salvo.WireCodec<EndToken> {
    override fun enc(v: EndToken, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): EndToken = EndToken()
}

fun lex(args: List<String>): List<Union4<LongToken, ShortToken, WordToken, EndToken>> {
    val out: salvo.platform.core.list.MutList<Union4<LongToken, ShortToken, WordToken, EndToken>> = mutableListOf<Union4<LongToken, ShortToken, WordToken, EndToken>>()
    var ended: Boolean = false
    for (arg in salvo.platform.core.list.each(args)) {
        val a: String = arg
        if (ended) {
            salvo.core.list.addPlatform(out, Union4.U3<LongToken, ShortToken, WordToken, EndToken>(WordToken(value = a)))
        } else if (((a) == ("--"))) {
            ended = true
            salvo.core.list.addPlatform(out, Union4.U4<LongToken, ShortToken, WordToken, EndToken>(EndToken()))
        } else if (salvo.core.string.startsWithPlatform(a, "--")) {
            val body: String = salvo.core.string.trimPrefixPlatform(a, "--")
            val parts: Pair<String, String>? = salvo.core.string.splitOnce(body, "=")
            if ((parts == null)) {
                salvo.core.list.addPlatform(out, Union4.U1<LongToken, ShortToken, WordToken, EndToken>(LongToken(name = body, value = null)))
            } else {
                val parts_1: Pair<String, String> = parts!!
                val __destructured_2: Pair<String, String> = parts_1
                val name: String = __destructured_2.first
                val value: String = __destructured_2.second
                salvo.core.list.addPlatform(out, Union4.U1<LongToken, ShortToken, WordToken, EndToken>(LongToken(name = name, value = value)))
            }
        } else if (((salvo.core.string.startsWithPlatform(a, "-") && !(((a) == ("-")))) && !(isNumber(a)))) {
            salvo.core.list.addPlatform(out, Union4.U2<LongToken, ShortToken, WordToken, EndToken>(ShortToken(letters = salvo.core.string.trimPrefixPlatform(a, "-"))))
        } else {
            salvo.core.list.addPlatform(out, Union4.U3<LongToken, ShortToken, WordToken, EndToken>(WordToken(value = a)))
        }
    }
    return out
}

fun isNumber(a: String): Boolean {
    val first: Char? = salvo.core.string.charAtPlatform(a, 1)
    if ((first == null)) {
        return false
    }
    val first_1: Char = first!!
    if (isDigit(first_1)) {
        return true
    }
    val second: Char? = salvo.core.string.charAtPlatform(a, 2)
    val first_2: Char = first!!
    if (((first_2 == '.') && !((second == null)))) {
        val second_3: Char = second!!
        return isDigit(second_3)
    }
    return false
}

fun isDigit(c: Char): Boolean {
    return ((c >= '0') && (c <= '9'))
}

