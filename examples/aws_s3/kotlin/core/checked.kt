package salvo.core.checked

import salvo.*

data class Checked<T>(
    val value: T,
)

class __Codec_Checked<T>(private val __c_T: salvo.WireCodec<T>) : salvo.WireCodec<Checked<T>> {
    override fun enc(v: Checked<T>, out: salvo.WireOut) {
        __c_T.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): Checked<T> = Checked(__c_T.dec(inp))
}

fun<T> checked(value: T): Checked<T> {
    return Checked<T>(value = value)
}

fun<T> ignore(checked: Checked<T>) {
    run { checked; Unit }
}

fun<T> detach(checked: Checked<T>): T {
    return checked.value
}

fun<T> toStr(checked: Checked<T>, toStr: (T) -> String): String {
    return toStr(checked.value)
}

