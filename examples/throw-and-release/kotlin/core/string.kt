package salvo.core.string

import salvo.*
import salvo.core.iterator.*
import salvo.core.list.*

fun iter__7(str: String): StrYield {
    return StrYield(text = str, at = 0)
}

data class StrYield(
    var text: String,
    var at: Int,
)

fun next__5(p: StrYield): Union2<Char, Finished> {
    val chr = p.text.getOrNull(p.at)
    if (chr == null) {
        return U2_2<Char, Finished>(finished())
    }
    p.at = p.at + 1
    return U2_1<Char, Finished>(emitted(chr))
}
