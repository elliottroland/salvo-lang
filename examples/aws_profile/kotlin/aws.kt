package salvo.aws

import salvo.core.string.*

data class ProfileCredentials(
    val profile: String = "default",
    val path: String = "~/.aws/credentials",
)

object __Codec_ProfileCredentials : salvo.WireCodec<ProfileCredentials> {
    override fun enc(v: ProfileCredentials, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.profile, out)
        salvo.StrCodec.enc(v.path, out)
    }
    override fun dec(inp: salvo.WireIn): ProfileCredentials = ProfileCredentials(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

fun to_str__6(value: ProfileCredentials): String {
    val out: StringBuilder = StringBuilder(listOf("ProfileCredentials {").joinToString(""))
    out.append(" ")
    out.append("profile: ${value.profile}")
    out.append(", ")
    out.append("path: ${value.path}")
    out.append(" }")
    return out.toString()
}
