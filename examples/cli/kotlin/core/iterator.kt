package salvo.core.iterator

import salvo.*

class Finished

object __Codec_Finished : salvo.WireCodec<Finished> {
    override fun enc(v: Finished, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): Finished = Finished()
}

fun<T> emitted(value: T): T {
    return value
}

fun finished(): Finished {
    return Finished()
}

