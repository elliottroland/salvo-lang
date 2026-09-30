package salvo.aws.s3

import salvo.*
import salvo.aws.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.other.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.stream.*
import salvo.time.*

data class PutObjectInput(
    val acl: String? = null,
    val body: InStream,
    val bucket: String,
    val cache_control: String? = null,
    val content_disposition: String? = null,
    val content_encoding: String? = null,
    val content_language: String? = null,
    val content_length: Long? = null,
    val content_md5: String? = null,
    val content_type: String? = null,
    val checksum_algorithm: String? = null,
    val checksum_crc32: String? = null,
    val checksum_crc32_c: String? = null,
    val checksum_crc64_nvme: String? = null,
    val checksum_sha1: String? = null,
    val checksum_sha256: String? = null,
    val checksum_sha512: String? = null,
    val checksum_md5: String? = null,
    val checksum_xxhash64: String? = null,
    val checksum_xxhash3: String? = null,
    val checksum_xxhash128: String? = null,
    val if_match: String? = null,
    val if_none_match: String? = null,
    val grant_full_control: String? = null,
    val grant_read: String? = null,
    val grant_read_acp: String? = null,
    val grant_write_acp: String? = null,
    val key: String,
    val write_offset_bytes: Long? = null,
    val metadata: Map<String, String>? = null,
    val server_side_encryption: String? = null,
    val storage_class: String? = null,
    val website_redirect_location: String? = null,
    val sse_customer_algorithm: String? = null,
    val sse_customer_key: String? = null,
    val sse_customer_key_md5: String? = null,
    val ssekms_key_id: String? = null,
    val ssekms_encryption_context: String? = null,
    val bucket_key_enabled: Boolean? = null,
    val request_payer: String? = null,
    val tagging: String? = null,
    val object_lock_mode: String? = null,
    val object_lock_retain_until_date: Instant? = null,
    val object_lock_legal_hold_status: String? = null,
    val object_lock_event_hold: String? = null,
    val object_lock_event_hold_duration_days: Int? = null,
    val object_lock_event_hold_duration_years: Int? = null,
    val expected_bucket_owner: String? = null,
)

fun close__4(streams: Streams, value: PutObjectInput): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(value.body)
}

data class PutObjectOutput(
    val expiration: String? = null,
    val e_tag: String? = null,
    val checksum_crc32: String? = null,
    val checksum_crc32_c: String? = null,
    val checksum_crc64_nvme: String? = null,
    val checksum_sha1: String? = null,
    val checksum_sha256: String? = null,
    val checksum_sha512: String? = null,
    val checksum_md5: String? = null,
    val checksum_xxhash64: String? = null,
    val checksum_xxhash3: String? = null,
    val checksum_xxhash128: String? = null,
    val checksum_type: String? = null,
    val server_side_encryption: String? = null,
    val version_id: String? = null,
    val sse_customer_algorithm: String? = null,
    val sse_customer_key_md5: String? = null,
    val ssekms_key_id: String? = null,
    val ssekms_encryption_context: String? = null,
    val bucket_key_enabled: Boolean? = null,
    val size: Long? = null,
    val request_charged: String? = null,
)

object __Codec_PutObjectOutput : salvo.WireCodec<PutObjectOutput> {
    override fun enc(v: PutObjectOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.expiration, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.e_tag, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_crc32, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_crc32_c, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_crc64_nvme, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_sha1, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_sha256, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_sha512, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_md5, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_xxhash64, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_xxhash3, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_xxhash128, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_type, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.server_side_encryption, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.version_id, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_algorithm, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_key_md5, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.ssekms_key_id, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.ssekms_encryption_context, out)
        salvo.OptCodec(salvo.BoolCodec).enc(v.bucket_key_enabled, out)
        salvo.OptCodec(salvo.LongCodec).enc(v.size, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.request_charged, out)
    }
    override fun dec(inp: salvo.WireIn): PutObjectOutput = PutObjectOutput(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.BoolCodec).dec(inp), salvo.OptCodec(salvo.LongCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetObjectInput(
    val bucket: String,
    val if_match: String? = null,
    val if_modified_since: Instant? = null,
    val if_none_match: String? = null,
    val if_unmodified_since: Instant? = null,
    val key: String,
    val range: String? = null,
    val response_cache_control: String? = null,
    val response_content_disposition: String? = null,
    val response_content_encoding: String? = null,
    val response_content_language: String? = null,
    val response_content_type: String? = null,
    val response_expires: Instant? = null,
    val version_id: String? = null,
    val sse_customer_algorithm: String? = null,
    val sse_customer_key: String? = null,
    val sse_customer_key_md5: String? = null,
    val request_payer: String? = null,
    val part_number: Int? = null,
    val expected_bucket_owner: String? = null,
    val checksum_mode: String? = null,
)

object __Codec_GetObjectInput : salvo.WireCodec<GetObjectInput> {
    override fun enc(v: GetObjectInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.bucket, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.if_match, out)
        salvo.OptCodec(__Codec_Instant).enc(v.if_modified_since, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.if_none_match, out)
        salvo.OptCodec(__Codec_Instant).enc(v.if_unmodified_since, out)
        salvo.StrCodec.enc(v.key, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.range, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_cache_control, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_content_disposition, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_content_encoding, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_content_language, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_content_type, out)
        salvo.OptCodec(__Codec_Instant).enc(v.response_expires, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.version_id, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_algorithm, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_key, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_key_md5, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.request_payer, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.part_number, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.expected_bucket_owner, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_mode, out)
    }
    override fun dec(inp: salvo.WireIn): GetObjectInput = GetObjectInput(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(__Codec_Instant).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(__Codec_Instant).dec(inp), salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(__Codec_Instant).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetObjectOutput(
    val body: InStream,
    val delete_marker: Boolean? = null,
    val accept_ranges: String? = null,
    val expiration: String? = null,
    val restore: String? = null,
    val last_modified: Instant? = null,
    val content_length: Long? = null,
    val e_tag: String? = null,
    val checksum_crc32: String? = null,
    val checksum_crc32_c: String? = null,
    val checksum_crc64_nvme: String? = null,
    val checksum_sha1: String? = null,
    val checksum_sha256: String? = null,
    val checksum_sha512: String? = null,
    val checksum_md5: String? = null,
    val checksum_xxhash64: String? = null,
    val checksum_xxhash3: String? = null,
    val checksum_xxhash128: String? = null,
    val checksum_type: String? = null,
    val missing_meta: Int? = null,
    val version_id: String? = null,
    val cache_control: String? = null,
    val content_disposition: String? = null,
    val content_encoding: String? = null,
    val content_language: String? = null,
    val content_range: String? = null,
    val content_type: String? = null,
    val website_redirect_location: String? = null,
    val server_side_encryption: String? = null,
    val metadata: Map<String, String>? = null,
    val sse_customer_algorithm: String? = null,
    val sse_customer_key_md5: String? = null,
    val ssekms_key_id: String? = null,
    val bucket_key_enabled: Boolean? = null,
    val storage_class: String? = null,
    val request_charged: String? = null,
    val replication_status: String? = null,
    val parts_count: Int? = null,
    val tag_count: Int? = null,
    val object_lock_mode: String? = null,
    val object_lock_retain_until_date: Instant? = null,
    val object_lock_legal_hold_status: String? = null,
    val object_lock_event_hold: String? = null,
    val object_lock_event_hold_duration_days: Int? = null,
    val object_lock_event_hold_duration_years: Int? = null,
)

fun close__5(streams: Streams, value: GetObjectOutput): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(value.body)
}

data class S3Error(
    val code: String,
    val message: String,
    val status: Int,
    val request_id: String? = null,
    val storage_class: String? = null,
    val access_tier: String? = null,
)

object __Codec_S3Error : salvo.WireCodec<S3Error> {
    override fun enc(v: S3Error, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.code, out)
        salvo.StrCodec.enc(v.message, out)
        salvo.IntCodec.enc(v.status, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.request_id, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.storage_class, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.access_tier, out)
    }
    override fun dec(inp: salvo.WireIn): S3Error = S3Error(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

interface S3 {
    fun put_object(input: PutObjectInput, reply: salvo.SalvoReply)
    fun get_object(input: GetObjectInput, reply: salvo.SalvoReply)
}

class __Mon_S3(private val inner: S3) : S3 {
    override fun put_object(input: PutObjectInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.put_object(input, reply) }
    override fun get_object(input: GetObjectInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.get_object(input, reply) }
}

interface S3Calls {
    fun calls(): List<String>
}

class __Mon_S3Calls(private val inner: S3Calls) : S3Calls {
    override fun calls(): List<String> =
        synchronized(inner) { inner.calls() }
}

class FakeS3(private val __dep_Streams: Streams) : S3, S3Calls {
    private var recorded: MutableList<String> = mutableListOf<String>()

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun put_object(input: PutObjectInput, reply: salvo.SalvoReply) {
        recorded.add("put_object")
        val unsized = input.content_length == null
        val closed = close__4(__dep_Streams, input)
        if (closed is U2_2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        if (unsized) {
            salvo.SalvoSched.replyWire(reply, U2_2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>(err(checked<Union2<S3Error, AwsError>>(U2_2<S3Error, AwsError>(AwsError(code = "MissingContentLength", message = "S3 PutObject streams its body, so the input needs content_length: the body's length in bytes"))))), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError))))
            return
        }
        salvo.SalvoSched.replyWire(reply, U2_1<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>(ok(PutObjectOutput())), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError))))
    }

    override fun get_object(input: GetObjectInput, reply: salvo.SalvoReply) {
        recorded.add("get_object")
        reply.send(U2_1<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>(ok(GetObjectOutput(body = __dep_Streams.from_bytes(salvo.SalvoBytes.of(arrayOf<UByte>()))))))
    }

    override fun calls(): List<String> {
        return recorded.toMutableList()
    }
}
