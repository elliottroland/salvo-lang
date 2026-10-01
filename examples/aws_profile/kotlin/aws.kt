package salvo.aws

import salvo.*
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

class EnvironmentCredentials

object __Codec_EnvironmentCredentials : salvo.WireCodec<EnvironmentCredentials> {
    override fun enc(v: EnvironmentCredentials, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): EnvironmentCredentials = EnvironmentCredentials()
}

class DefaultChain

object __Codec_DefaultChain : salvo.WireCodec<DefaultChain> {
    override fun enc(v: DefaultChain, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): DefaultChain = DefaultChain()
}

data class Region(
    val code: String,
)

object __Codec_Region : salvo.WireCodec<Region> {
    override fun enc(v: Region, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.code, out)
    }
    override fun dec(inp: salvo.WireIn): Region = Region(salvo.StrCodec.dec(inp))
}

data class AwsConfig(
    val credentials: Union3<ProfileCredentials, EnvironmentCredentials, DefaultChain> = U3_3<ProfileCredentials, EnvironmentCredentials, DefaultChain>(DefaultChain()),
    val region: Region,
    val endpoint: String? = null,
)

object __Codec_AwsConfig : salvo.WireCodec<AwsConfig> {
    override fun enc(v: AwsConfig, out: salvo.WireOut) {
        salvo.Union3Codec(__Codec_ProfileCredentials, __Codec_EnvironmentCredentials, __Codec_DefaultChain).enc(v.credentials, out)
        __Codec_Region.enc(v.region, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.endpoint, out)
    }
    override fun dec(inp: salvo.WireIn): AwsConfig = AwsConfig(salvo.Union3Codec(__Codec_ProfileCredentials, __Codec_EnvironmentCredentials, __Codec_DefaultChain).dec(inp), __Codec_Region.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class AwsError(
    val code: String,
    val message: String,
)

object __Codec_AwsError : salvo.WireCodec<AwsError> {
    override fun enc(v: AwsError, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.code, out)
        salvo.StrCodec.enc(v.message, out)
    }
    override fun dec(inp: salvo.WireIn): AwsError = AwsError(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun profileOf(c: Union3<ProfileCredentials, EnvironmentCredentials, DefaultChain>): ProfileCredentials? {
    if (c is U3_1<*, *, *>) {
        return (c.value as ProfileCredentials)
    }
    return null
}

fun usesEnvironment(c: Union3<ProfileCredentials, EnvironmentCredentials, DefaultChain>): Boolean {
    return c is U3_2<*, *, *>
}

fun toStr__7(value: ProfileCredentials): String {
    val out: StringBuilder = StringBuilder(listOf("ProfileCredentials {").joinToString(""))
    out.append(" ")
    out.append("profile: ${value.profile}")
    out.append(", ")
    out.append("path: ${value.path}")
    out.append(" }")
    return out.toString()
}
