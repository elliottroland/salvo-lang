package salvo.core.bytes

import salvo.*
import salvo.core.iterator.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.sorted.*
import salvo.core.string.*

fun iter__2(data: salvo.SalvoBytes): BytesYield {
    return BytesYield(data = data, at = 0)
}

data class BytesYield(
    var data: salvo.SalvoBytes,
    var at: Int,
)

fun next__2(p: BytesYield): Union2<UByte, Finished> {
    val b = p.data.getOrNull(p.at)
    if (b == null) {
        return U2_2<UByte, Finished>(finished())
    }
    p.at = p.at + 1
    return U2_1<UByte, Finished>(emitted(b))
}
