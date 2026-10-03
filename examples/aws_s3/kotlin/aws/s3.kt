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
    val cacheControl: String? = null,
    val contentDisposition: String? = null,
    val contentEncoding: String? = null,
    val contentLanguage: String? = null,
    val contentLength: Long? = null,
    val contentMd5: String? = null,
    val contentType: String? = null,
    val checksumAlgorithm: String? = null,
    val checksumCrc32: String? = null,
    val checksumCrc32C: String? = null,
    val checksumCrc64Nvme: String? = null,
    val checksumSha1: String? = null,
    val checksumSha256: String? = null,
    val checksumSha512: String? = null,
    val checksumMd5: String? = null,
    val checksumXxhash64: String? = null,
    val checksumXxhash3: String? = null,
    val checksumXxhash128: String? = null,
    val ifMatch: String? = null,
    val ifNoneMatch: String? = null,
    val grantFullControl: String? = null,
    val grantRead: String? = null,
    val grantReadAcp: String? = null,
    val grantWriteAcp: String? = null,
    val key: String,
    val writeOffsetBytes: Long? = null,
    val metadata: Map<String, String>? = null,
    val serverSideEncryption: String? = null,
    val storageClass: String? = null,
    val websiteRedirectLocation: String? = null,
    val sseCustomerAlgorithm: String? = null,
    val sseCustomerKey: String? = null,
    val sseCustomerKeyMd5: String? = null,
    val ssekmsKeyId: String? = null,
    val ssekmsEncryptionContext: String? = null,
    val bucketKeyEnabled: Boolean? = null,
    val requestPayer: String? = null,
    val tagging: String? = null,
    val objectLockMode: String? = null,
    val objectLockRetainUntilDate: Instant? = null,
    val objectLockLegalHoldStatus: String? = null,
    val objectLockEventHold: String? = null,
    val objectLockEventHoldDurationDays: Int? = null,
    val objectLockEventHoldDurationYears: Int? = null,
    val expectedBucketOwner: String? = null,
)

fun close__4(streams: Streams, value: PutObjectInput): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(value.body)
}

data class PutObjectOutput(
    val expiration: String? = null,
    val eTag: String? = null,
    val checksumCrc32: String? = null,
    val checksumCrc32C: String? = null,
    val checksumCrc64Nvme: String? = null,
    val checksumSha1: String? = null,
    val checksumSha256: String? = null,
    val checksumSha512: String? = null,
    val checksumMd5: String? = null,
    val checksumXxhash64: String? = null,
    val checksumXxhash3: String? = null,
    val checksumXxhash128: String? = null,
    val checksumType: String? = null,
    val serverSideEncryption: String? = null,
    val versionId: String? = null,
    val sseCustomerAlgorithm: String? = null,
    val sseCustomerKeyMd5: String? = null,
    val ssekmsKeyId: String? = null,
    val ssekmsEncryptionContext: String? = null,
    val bucketKeyEnabled: Boolean? = null,
    val size: Long? = null,
    val requestCharged: String? = null,
)

object __Codec_PutObjectOutput : salvo.WireCodec<PutObjectOutput> {
    override fun enc(v: PutObjectOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.expiration, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.eTag, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumCrc32, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumCrc32C, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumCrc64Nvme, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumSha1, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumSha256, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumSha512, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumMd5, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumXxhash64, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumXxhash3, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumXxhash128, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumType, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.serverSideEncryption, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.versionId, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sseCustomerAlgorithm, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sseCustomerKeyMd5, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.ssekmsKeyId, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.ssekmsEncryptionContext, out)
        salvo.OptCodec(salvo.BoolCodec).enc(v.bucketKeyEnabled, out)
        salvo.OptCodec(salvo.LongCodec).enc(v.size, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.requestCharged, out)
    }
    override fun dec(inp: salvo.WireIn): PutObjectOutput = PutObjectOutput(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.BoolCodec).dec(inp), salvo.OptCodec(salvo.LongCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetObjectInput(
    val bucket: String,
    val ifMatch: String? = null,
    val ifModifiedSince: Instant? = null,
    val ifNoneMatch: String? = null,
    val ifUnmodifiedSince: Instant? = null,
    val key: String,
    val range: String? = null,
    val responseCacheControl: String? = null,
    val responseContentDisposition: String? = null,
    val responseContentEncoding: String? = null,
    val responseContentLanguage: String? = null,
    val responseContentType: String? = null,
    val responseExpires: Instant? = null,
    val versionId: String? = null,
    val sseCustomerAlgorithm: String? = null,
    val sseCustomerKey: String? = null,
    val sseCustomerKeyMd5: String? = null,
    val requestPayer: String? = null,
    val partNumber: Int? = null,
    val expectedBucketOwner: String? = null,
    val checksumMode: String? = null,
)

object __Codec_GetObjectInput : salvo.WireCodec<GetObjectInput> {
    override fun enc(v: GetObjectInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.bucket, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.ifMatch, out)
        salvo.OptCodec(__Codec_Instant).enc(v.ifModifiedSince, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.ifNoneMatch, out)
        salvo.OptCodec(__Codec_Instant).enc(v.ifUnmodifiedSince, out)
        salvo.StrCodec.enc(v.key, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.range, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.responseCacheControl, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.responseContentDisposition, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.responseContentEncoding, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.responseContentLanguage, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.responseContentType, out)
        salvo.OptCodec(__Codec_Instant).enc(v.responseExpires, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.versionId, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sseCustomerAlgorithm, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sseCustomerKey, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sseCustomerKeyMd5, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.requestPayer, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.partNumber, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.expectedBucketOwner, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksumMode, out)
    }
    override fun dec(inp: salvo.WireIn): GetObjectInput = GetObjectInput(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(__Codec_Instant).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(__Codec_Instant).dec(inp), salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(__Codec_Instant).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetObjectOutput(
    val body: InStream,
    val deleteMarker: Boolean? = null,
    val acceptRanges: String? = null,
    val expiration: String? = null,
    val restore: String? = null,
    val lastModified: Instant? = null,
    val contentLength: Long? = null,
    val eTag: String? = null,
    val checksumCrc32: String? = null,
    val checksumCrc32C: String? = null,
    val checksumCrc64Nvme: String? = null,
    val checksumSha1: String? = null,
    val checksumSha256: String? = null,
    val checksumSha512: String? = null,
    val checksumMd5: String? = null,
    val checksumXxhash64: String? = null,
    val checksumXxhash3: String? = null,
    val checksumXxhash128: String? = null,
    val checksumType: String? = null,
    val missingMeta: Int? = null,
    val versionId: String? = null,
    val cacheControl: String? = null,
    val contentDisposition: String? = null,
    val contentEncoding: String? = null,
    val contentLanguage: String? = null,
    val contentRange: String? = null,
    val contentType: String? = null,
    val websiteRedirectLocation: String? = null,
    val serverSideEncryption: String? = null,
    val metadata: Map<String, String>? = null,
    val sseCustomerAlgorithm: String? = null,
    val sseCustomerKeyMd5: String? = null,
    val ssekmsKeyId: String? = null,
    val bucketKeyEnabled: Boolean? = null,
    val storageClass: String? = null,
    val requestCharged: String? = null,
    val replicationStatus: String? = null,
    val partsCount: Int? = null,
    val tagCount: Int? = null,
    val objectLockMode: String? = null,
    val objectLockRetainUntilDate: Instant? = null,
    val objectLockLegalHoldStatus: String? = null,
    val objectLockEventHold: String? = null,
    val objectLockEventHoldDurationDays: Int? = null,
    val objectLockEventHoldDurationYears: Int? = null,
)

fun close__5(streams: Streams, value: GetObjectOutput): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(value.body)
}

data class S3Error(
    val code: String,
    val message: String,
    val status: Int,
    val requestId: String? = null,
    val storageClass: String? = null,
    val accessTier: String? = null,
)

object __Codec_S3Error : salvo.WireCodec<S3Error> {
    override fun enc(v: S3Error, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.code, out)
        salvo.StrCodec.enc(v.message, out)
        salvo.IntCodec.enc(v.status, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.requestId, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.storageClass, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.accessTier, out)
    }
    override fun dec(inp: salvo.WireIn): S3Error = S3Error(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

// Factories for the host: one per arm of the union [platform-factory].
object S3Failures {
    fun s3Error(value: S3Error): Union2<S3Error, AwsError> = salvo.Union2.U1(value)
    fun awsError(value: AwsError): Union2<S3Error, AwsError> = salvo.Union2.U2(value)
}

interface S3 {
    fun putObject(input: PutObjectInput, reply: salvo.SalvoReply)
    fun getObject(input: GetObjectInput, reply: salvo.SalvoReply)
}

class __Mon_S3(
    private val inner: S3,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : S3 {
    override fun putObject(input: PutObjectInput, reply: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.putObject(input, reply) } finally { lock.unlock() }
    }
    override fun getObject(input: GetObjectInput, reply: salvo.SalvoReply) {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { inner.getObject(input, reply) } finally { lock.unlock() }
    }
}

interface S3Calls {
    fun calls(): List<String>
}

class __Mon_S3Calls(
    private val inner: S3Calls,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : S3Calls {
    override fun calls(): List<String> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.calls() } finally { lock.unlock() }
    }
}

class FakeS3(private val __dep_Streams: Streams) : S3, S3Calls {
    private var recorded: MutableList<String> = mutableListOf<String>()

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun putObject(input: PutObjectInput, reply: salvo.SalvoReply) {
        recorded.add("put_object")
        val unsized = input.contentLength == null
        val closed = close__4(__dep_Streams, input)
        if (closed is Union2.U2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        if (unsized) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>(err(checked<Union2<S3Error, AwsError>>(Union2.U2<S3Error, AwsError>(AwsError(code = "MissingContentLength", message = "S3 PutObject streams its body, so the input needs content_length: the body's length in bytes"))))), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError))))
            return
        }
        salvo.SalvoSched.replyWire(reply, Union2.U1<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>(ok(PutObjectOutput())), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError))))
    }

    override fun getObject(input: GetObjectInput, reply: salvo.SalvoReply) {
        recorded.add("get_object")
        reply.send(Union2.U1<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>(ok(GetObjectOutput(body = __dep_Streams.fromBytes(salvo.SalvoBytes.of(arrayOf<UByte>()))))))
    }

    override fun calls(): List<String> {
        return recorded.toMutableList()
    }
}
