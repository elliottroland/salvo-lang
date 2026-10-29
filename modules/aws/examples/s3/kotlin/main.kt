package salvo.main

import salvo.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun describe(e: Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>): String {
    if ((e is Union2.U2<*, *>)) {
        val e_1: salvo.aws.AwsError = ((e as Union2.U2<*, *>).value as salvo.aws.AwsError)
        return "${e_1.code}: ${e_1.message}"
    }
    val e_2: salvo.aws.s3.S3Error = ((e as Union2.U1<*, *>).value as salvo.aws.s3.S3Error)
    if ((e_2.code == "NoSuchKey")) {
        return "no such key"
    }
    val e_3: salvo.aws.s3.S3Error = ((e as Union2.U1<*, *>).value as salvo.aws.s3.S3Error)
    return "the service refused: ${e_3.code}"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun sizeOf(fs: salvo.std.fs.Fs, streams: salvo.std.stream.Streams, path: salvo.std.fs.path.Path): Long? {
    val info: Union2<salvo.std.fs.FileInfo, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.metadata(path)
    return when {
        (info is Union2.U1<*, *>) -> {
            val info_1: salvo.std.fs.FileInfo = ((info as Union2.U1<*, *>).value as salvo.std.fs.FileInfo)
            return info_1.size
        }
        (info is Union2.U2<*, *>) -> {
            val info_2: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((info as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.checked.ignore(info_2)
            return null
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:37:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun upload(s3: salvo.aws.s3.S3, fs: salvo.std.fs.Fs, console: salvo.std.core.console.Console, streams: salvo.std.stream.Streams, bucket: String, key: String, path: salvo.std.fs.path.Path, length: Long?) {
    val opened: Union2<salvo.std.stream.InStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openRead(path)
    if ((opened is Union2.U2<*, *>)) {
        val opened_1: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((opened as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
        salvo.std.core.console.println(console, "open ${salvo.std.fs.path.toStr(path)}: ${salvo.std.fs.toStr(salvo.std.core.checked.detach(opened_1))}")
        null
        return
    }
    val put: Union2<salvo.aws.s3.PutObjectOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.aws.s3.__Codec_PutObjectOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.s3.__Codec_S3Error, salvo.aws.__Codec_AwsError)))) })
        val opened_2: salvo.std.stream.InStream = ((opened as Union2.U1<*, *>).value as salvo.std.stream.InStream)
        s3.putObject(salvo.aws.s3.PutObjectInput(acl = null, body = opened_2, bucket = bucket, cacheControl = null, contentDisposition = null, contentEncoding = null, contentLanguage = null, contentLength = length, contentMd5 = null, contentType = null, checksumAlgorithm = null, checksumCrc32 = null, checksumCrc32C = null, checksumCrc64Nvme = null, checksumSha1 = null, checksumSha256 = null, checksumSha512 = null, checksumMd5 = null, checksumXxhash64 = null, checksumXxhash3 = null, checksumXxhash128 = null, ifMatch = null, ifNoneMatch = null, grantFullControl = null, grantRead = null, grantReadAcp = null, grantWriteAcp = null, key = key, writeOffsetBytes = null, metadata = null, serverSideEncryption = null, storageClass = null, websiteRedirectLocation = null, sseCustomerAlgorithm = null, sseCustomerKey = null, sseCustomerKeyMd5 = null, ssekmsKeyId = null, ssekmsEncryptionContext = null, bucketKeyEnabled = null, requestPayer = null, tagging = null, objectLockMode = null, objectLockRetainUntilDate = null, objectLockLegalHoldStatus = null, objectLockEventHold = null, objectLockEventHoldDurationDays = null, objectLockEventHoldDurationYears = null, expectedBucketOwner = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.s3.PutObjectOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>>
    }
    when {
        (put is Union2.U1<*, *>) -> {
            val put_3: salvo.aws.s3.PutObjectOutput = ((put as Union2.U1<*, *>).value as salvo.aws.s3.PutObjectOutput)
            salvo.std.core.console.println(console, "put ${key}: etag ${kotlin.run {
                val __pick_4: String? = put_3.eTag
                (if ((__pick_4 != null)) {
                    val __picked_5: String = __pick_4!!
                    __picked_5
                } else {
                    "?"
                })
            }}")
        }
        (put is Union2.U2<*, *>) -> {
            val put_7: salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>> = ((put as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>)
            salvo.std.core.console.println(console, "put ${key}: ${salvo.main.describe(salvo.std.core.checked.detach(put_7))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:58:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun download(s3: salvo.aws.s3.S3, fs: salvo.std.fs.Fs, console: salvo.std.core.console.Console, streams: salvo.std.stream.Streams, bucket: String, key: String, path: salvo.std.fs.path.Path) {
    val got: Union2<salvo.aws.s3.GetObjectOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> kotlin.Pair(false, null) })
        s3.getObject(salvo.aws.s3.GetObjectInput(bucket = bucket, ifMatch = null, ifModifiedSince = null, ifNoneMatch = null, ifUnmodifiedSince = null, key = key, range = null, responseCacheControl = null, responseContentDisposition = null, responseContentEncoding = null, responseContentLanguage = null, responseContentType = null, responseExpires = null, versionId = null, sseCustomerAlgorithm = null, sseCustomerKey = null, sseCustomerKeyMd5 = null, requestPayer = null, partNumber = null, expectedBucketOwner = null, checksumMode = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.s3.GetObjectOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>>
    }
    if ((got is Union2.U2<*, *>)) {
        val got_1: salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>> = ((got as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>)
        salvo.std.core.console.println(console, "get ${key}: ${salvo.main.describe(salvo.std.core.checked.detach(got_1))}")
        null
        return
    }
    val got_2: salvo.aws.s3.GetObjectOutput = ((got as Union2.U1<*, *>).value as salvo.aws.s3.GetObjectOutput)
    val __destructured_3: salvo.aws.s3.GetObjectOutput = got_2
    val body: salvo.std.stream.InStream = __destructured_3.body
    val contentLength: Long? = __destructured_3.contentLength
    salvo.std.core.console.println(console, "get ${key}: ${kotlin.run {
        val __pick_4: Long? = contentLength
        (if ((__pick_4 != null)) {
            val __picked_5: Long = __pick_4!!
            __picked_5
        } else {
            (-1L)
        })
    }} bytes")
    val target: Union2<salvo.std.stream.OutStream, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = fs.openWrite(path)
    if ((target is Union2.U2<*, *>)) {
        val target_7: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((target as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
        salvo.std.core.console.println(console, "open ${salvo.std.fs.path.toStr(path)}: ${salvo.std.fs.toStr(salvo.std.core.checked.detach(target_7))}")
        val closed: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = streams.close__InStream(body)
        if ((closed is Union2.U2<*, *>)) {
            val closed_8: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
            salvo.std.core.checked.ignore(closed_8)
        }
        null
        return
    }
    val copied: Union2<Long, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.LongCodec, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.std.stream.__Codec_InvalidUtf8, salvo.std.stream.__Codec_StreamFailed)))) })
        val target_9: salvo.std.stream.OutStream = ((target as Union2.U1<*, *>).value as salvo.std.stream.OutStream)
        salvo.std.stream.pipe(streams, body, target_9, r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<Long, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>>
    }
    when {
        (copied is Union2.U1<*, *>) -> {
            val copied_10: Long = ((copied as Union2.U1<*, *>).value as Long)
            salvo.std.core.console.println(console, "piped ${copied_10} bytes into ${salvo.std.fs.path.toStr(path)}")
        }
        (copied is Union2.U2<*, *>) -> {
            val copied_11: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((copied as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
            salvo.std.core.console.println(console, "pipe: ${salvo.std.stream.toStr(salvo.std.core.checked.detach(copied_11))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:88:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun roundTrip(s3: salvo.aws.s3.S3, fs: salvo.std.fs.Fs, console: salvo.std.core.console.Console, streams: salvo.std.stream.Streams, key: String) {
    salvo.main.upload(s3, fs, console, streams, "notes", key, salvo.std.fs.path.path("notes.txt"), salvo.main.sizeOf(fs, streams, salvo.std.fs.path.path("notes.txt")))
    salvo.main.download(s3, fs, console, streams, "notes", key, salvo.std.fs.path.path("back.txt"))
    val back: Union2<String, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.readToStr(fs, streams, salvo.std.fs.path.path("back.txt"))
    when {
        (back is Union2.U1<*, *>) -> {
            val back_1: String = ((back as Union2.U1<*, *>).value as String)
            salvo.std.core.console.println(console, "back.txt: ${salvo.std.core.string.sizePlatform(back_1)} bytes")
            console.print(back_1)
        }
        (back is Union2.U2<*, *>) -> {
            val back_2: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((back as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
            salvo.std.core.console.println(console, "back.txt: ${salvo.std.fs.toStr(salvo.std.core.checked.detach(back_2))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:99:5"))
        }
    }
}

class MemS3(private val __dep0: salvo.std.stream.Streams) : salvo.aws.s3.S3 {
    var objects: salvo.platform.std.core.map.MutMap<String, salvo.platform.std.core.bytes.Bytes> = salvo.std.core.map.Mut_Map_platform(arrayOf<kotlin.Pair<String, salvo.platform.std.core.bytes.Bytes>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun putObject(input: salvo.aws.s3.PutObjectInput, reply: salvo.SalvoReply) {
        val __destructured_1: salvo.aws.s3.PutObjectInput = input
        val bucket: String = __destructured_1.bucket
        val key: String = __destructured_1.key
        val body: salvo.std.stream.InStream = __destructured_1.body
        val buf: salvo.platform.std.core.bytes.MutBytes = salvo.std.core.bytes.mutBytes(arrayOf<salvo.platform.std.core.bytes.Bytes>())
        val filled: Union2<Long, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = salvo.std.stream.fillFrom(__dep0, body, buf)
        val closed: Union2<Unit, salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>> = __dep0.close__InStream(body)
        if ((closed is Union2.U2<*, *>)) {
            val closed_2: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((closed as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
            salvo.std.core.checked.ignore(closed_2)
        }
        if ((filled is Union2.U2<*, *>)) {
            val filled_3: salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>> = ((filled as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.std.stream.InvalidUtf8, salvo.std.stream.StreamFailed>>)
            salvo.SalvoSched.replyWire(reply, Union2.U2<salvo.aws.s3.PutObjectOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U2<salvo.aws.s3.S3Error, salvo.aws.AwsError>(salvo.aws.AwsError(code = "StreamFailed", message = "${salvo.std.stream.toStr(salvo.std.core.checked.detach(filled_3))}"))))), salvo.Union2Codec(salvo.aws.s3.__Codec_PutObjectOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.s3.__Codec_S3Error, salvo.aws.__Codec_AwsError))))
            null
            return
        }
        val data: salvo.platform.std.core.bytes.Bytes = buf
        val tag: String = "\"${salvo.std.core.bytes.sizePlatform(data)}\""
        salvo.std.core.map.putPlatform(objects, "${bucket}/${key}", data, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.SalvoSched.replyWire(reply, Union2.U1<salvo.aws.s3.PutObjectOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.s3.PutObjectOutput(expiration = null, eTag = tag, checksumCrc32 = null, checksumCrc32C = null, checksumCrc64Nvme = null, checksumSha1 = null, checksumSha256 = null, checksumSha512 = null, checksumMd5 = null, checksumXxhash64 = null, checksumXxhash3 = null, checksumXxhash128 = null, checksumType = null, serverSideEncryption = null, versionId = null, sseCustomerAlgorithm = null, sseCustomerKeyMd5 = null, ssekmsKeyId = null, ssekmsEncryptionContext = null, bucketKeyEnabled = null, size = null, requestCharged = null))), salvo.Union2Codec(salvo.aws.s3.__Codec_PutObjectOutput, salvo.std.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.s3.__Codec_S3Error, salvo.aws.__Codec_AwsError))))
    }
    override fun getObject(input: salvo.aws.s3.GetObjectInput, reply: salvo.SalvoReply) {
        val found: salvo.platform.std.core.bytes.Bytes? = salvo.std.core.map.getPlatform(objects, "${input.bucket}/${input.key}", { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((found == null)) {
            reply.send(Union2.U2<salvo.aws.s3.GetObjectOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.aws.s3.S3Error, salvo.aws.AwsError>(salvo.aws.s3.S3Error(code = "NoSuchKey", message = "The specified key does not exist.", status = 404, requestId = null, storageClass = null, accessTier = null))))))
            null
            return
        }
        val found_1: salvo.platform.std.core.bytes.Bytes = found!!
        val data: salvo.platform.std.core.bytes.Bytes = found_1
        val length: Long = (salvo.std.core.bytes.sizePlatform(data)).toLong()
        reply.send(Union2.U1<salvo.aws.s3.GetObjectOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.s3.S3Error, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.s3.GetObjectOutput(body = __dep0.fromBytes(data), deleteMarker = null, acceptRanges = null, expiration = null, restore = null, lastModified = null, contentLength = length, eTag = null, checksumCrc32 = null, checksumCrc32C = null, checksumCrc64Nvme = null, checksumSha1 = null, checksumSha256 = null, checksumSha512 = null, checksumMd5 = null, checksumXxhash64 = null, checksumXxhash3 = null, checksumXxhash128 = null, checksumType = null, missingMeta = null, versionId = null, cacheControl = null, contentDisposition = null, contentEncoding = null, contentLanguage = null, contentRange = null, contentType = null, websiteRedirectLocation = null, serverSideEncryption = null, metadata = null, sseCustomerAlgorithm = null, sseCustomerKeyMd5 = null, ssekmsKeyId = null, bucketKeyEnabled = null, storageClass = null, requestCharged = null, replicationStatus = null, partsCount = null, tagCount = null, objectLockMode = null, objectLockRetainUntilDate = null, objectLockLegalHoldStatus = null, objectLockEventHold = null, objectLockEventHoldDurationDays = null, objectLockEventHoldDurationYears = null))))
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(kotlin.collections.listOf(kotlin.Pair("Faults", salvo.std.core.actor.__PROTO_Faults), kotlin.Pair("Timer", salvo.std.time.__PROTO_Timer), kotlin.Pair("TimerCtl", salvo.std.time.__PROTO_TimerCtl), kotlin.Pair("Wheel", salvo.std.runtime.timers.__PROTO_Wheel)))
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val __use_3: salvo.std.fs.mem.MemFs = salvo.std.fs.mem.MemFs()
    val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
    val __handle_4: salvo.std.fs.Fs = salvo.std.fs.__Mon_Fs(__use_3, __lock___use_3)
    val __handle_5: salvo.std.stream.Streams = salvo.std.stream.__Mon_Streams(__use_3, __lock___use_3)
    val written: Union2<Long, salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>> = salvo.std.fs.writeStr(__handle_4, __handle_5, salvo.std.fs.path.path("notes.txt"), "hello from Salvo\nsecond line\n")
    if ((written is Union2.U2<*, *>)) {
        val written_6: salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>> = ((written as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union7<salvo.std.fs.NotFound, salvo.std.fs.PermissionDenied, salvo.std.fs.AlreadyExists, salvo.std.fs.NotADirectory, salvo.std.fs.PathEscapes, salvo.std.fs.IoError, salvo.std.fs.Streaming>>)
        salvo.std.core.console.println(__handle_2, "write: ${salvo.std.fs.toStr(salvo.std.core.checked.detach(written_6))}")
        null
        return
    }
    salvo.std.core.console.println(__handle_2, "-- FakeS3 --")
    kotlin.run {
        val __use_7: salvo.aws.s3.FakeS3 = salvo.aws.s3.FakeS3(__handle_5)
        val __lock___use_7 = java.util.concurrent.locks.ReentrantLock()
        val __handle_8: salvo.aws.s3.S3 = salvo.aws.s3.__Mon_S3(__use_7, __lock___use_7)
        val __handle_9: salvo.aws.s3.S3Calls = salvo.aws.s3.__Mon_S3Calls(__use_7, __lock___use_7)
        salvo.main.roundTrip(__handle_8, __handle_4, __handle_2, __handle_5, "greeting.txt")
        salvo.main.upload(__handle_8, __handle_4, __handle_2, __handle_5, "notes", "unsized.txt", salvo.std.fs.path.path("notes.txt"), null)
        salvo.std.core.console.println(__handle_2, "calls: ${salvo.std.core.list.toStr(__handle_9.calls(), { __a0 -> __a0 })}")
    }
    salvo.std.core.console.println(__handle_2, "-- MemS3 --")
    kotlin.run {
        val __use_10: MemS3 = MemS3(__handle_5)
        val __lock___use_10 = java.util.concurrent.locks.ReentrantLock()
        val __handle_11: salvo.aws.s3.S3 = salvo.aws.s3.__Mon_S3(__use_10, __lock___use_10)
        salvo.main.roundTrip(__handle_11, __handle_4, __handle_2, __handle_5, "greeting.txt")
        salvo.main.download(__handle_11, __handle_4, __handle_2, __handle_5, "notes", "missing.txt", salvo.std.fs.path.path("missing.txt"))
    }
}

