package salvo.core.string

import salvo.*
import salvo.core.iterator.*
import salvo.core.list.*

fun iter__3(str: String): StrYield {
    return StrYield(text = str, at = 0)
}

data class StrYield(
    var text: String,
    var at: Int,
)

fun next__5(pass: StrYield): Union2<Char, Finished> {
    val chr = pass.text.getOrNull(pass.at)
    if (chr == null) {
        return U2_2<Char, Finished>(finished())
    }
    pass.at = pass.at + 1
    return U2_1<Char, Finished>(emitted(chr))
}
