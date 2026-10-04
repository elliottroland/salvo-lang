package salvo.main

import salvo.*
import salvo.aws.*
import salvo.aws.s3.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.console.*
import salvo.core.deque.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.fs.*
import salvo.fs.mem.*
import salvo.stream.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun describe(e: Union2<S3Error, AwsError>): String {
    if (e is Union2.U2<*, *>) {
        return "${(e.value as AwsError).code}: ${(e.value as AwsError).message}"
    }
    if (((e.value as S3Error).code == "NoSuchKey")) {
        return "no such key"
    }
    return "the service refused: ${(e.value as S3Error).code}"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun sizeOf(fs: Fs, streams: salvo.stream.Streams, path: String): Long? {
    val info = fs.metadata(path)
    when (info) {
        is Union2.U1<*, *> -> {
            return (info.value as FileInfo).size
        }
        is Union2.U2<*, *> -> {
            ignore((info.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
            return null
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun upload(s3: S3, fs: Fs, console: Console, streams: salvo.stream.Streams, bucket: String, key: String, path: String, length: Long?) {
    val opened = fs.openRead(path)
    if (opened is Union2.U2<*, *>) {
        println(console, "open $path: ${toStr(detach((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        return
    }
    val put = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError)))) })
        s3.putObject(PutObjectInput(bucket = bucket, key = key, body = (opened.value as InStream), contentLength = length), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>
    }
    when (put) {
        is Union2.U1<*, *> -> {
            println(console, "put $key: etag ${((put.value as PutObjectOutput).eTag ?: "?")}")
        }
        is Union2.U2<*, *> -> {
            println(console, "put $key: ${describe(detach((put.value as Checked<Union2<S3Error, AwsError>>)))}")
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun download(s3: S3, fs: Fs, console: Console, streams: salvo.stream.Streams, bucket: String, key: String, path: String) {
    val got = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> Pair(false, null) })
        s3.getObject(GetObjectInput(bucket = bucket, key = key), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>
    }
    if (got is Union2.U2<*, *>) {
        println(console, "get $key: ${describe(detach((got.value as Checked<Union2<S3Error, AwsError>>)))}")
        return
    }
    val __destructured1 = (got.value as GetObjectOutput)
    val body = __destructured1.body
    val contentLength = __destructured1.contentLength
    println(console, "get $key: ${(contentLength ?: -1L)} bytes")
    val target = fs.openWrite(path)
    if (target is Union2.U2<*, *>) {
        println(console, "open $path: ${toStr(detach((target.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        val closed = streams.close(body)
        if (closed is Union2.U2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        return
    }
    val copied = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.LongCodec, __Codec_Checked(salvo.Union2Codec(__Codec_InvalidUtf8, __Codec_StreamFailed)))) })
        pipe(streams, body, (target.value as OutStream), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<Long, Checked<Union2<InvalidUtf8, StreamFailed>>>
    }
    when (copied) {
        is Union2.U1<*, *> -> {
            println(console, "piped ${(copied.value as Long)} bytes into $path")
        }
        is Union2.U2<*, *> -> {
            println(console, "pipe: ${toStr__5(detach((copied.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun roundTrip(s3: S3, fs: Fs, console: Console, streams: salvo.stream.Streams, key: String) {
    upload(s3, fs, console, streams, "notes", key, "notes.txt", sizeOf(fs, streams, "notes.txt"))
    download(s3, fs, console, streams, "notes", key, "back.txt")
    val back = readToStr(fs, streams, "back.txt")
    when (back) {
        is Union2.U1<*, *> -> {
            println(console, "back.txt: ${sizePlatform((back.value as String))} bytes")
            console.print((back.value as String))
        }
        is Union2.U2<*, *> -> {
            println(console, "back.txt: ${toStr(detach((back.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
}

class MemS3(private val __dep_salvo_stream_Streams: salvo.stream.Streams) : S3 {
    private var objects: MutableMap<String, salvo.platform.core.bytes.Bytes> = linkedMapOf<String, salvo.platform.core.bytes.Bytes>().also { __m -> __m.putAll(listOf()) }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
    override fun putObject(input: PutObjectInput, reply: salvo.SalvoReply) {
        val __destructured2 = input
        val bucket = __destructured2.bucket
        val key = __destructured2.key
        val body = __destructured2.body
        val buf = mutBytes(arrayOf())
        val filled = fillFrom(__dep_salvo_stream_Streams, body, buf)
        val closed = __dep_salvo_stream_Streams.close(body)
        if (closed is Union2.U2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        if (filled is Union2.U2<*, *>) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>(err(checked<Union2<S3Error, AwsError>>(Union2.U2<S3Error, AwsError>(AwsError(code = "StreamFailed", message = "${toStr__5(detach((filled.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}"))))), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError))))
            return
        }
        val data: salvo.platform.core.bytes.Bytes = buf
        val tag = "\"${sizePlatform(data)}\""
        objects.put("$bucket/$key", data)
        salvo.SalvoSched.replyWire(reply, Union2.U1<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>(ok(PutObjectOutput(eTag = tag))), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError))))
    }

    override fun getObject(input: GetObjectInput, reply: salvo.SalvoReply) {
        val found = objects["${input.bucket}/${input.key}"]
        if (found == null) {
            reply.send(Union2.U2<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>(err(checked<Union2<S3Error, AwsError>>(Union2.U1<S3Error, AwsError>(S3Error(code = "NoSuchKey", message = "The specified key does not exist.", status = 404))))))
            return
        }
        val data: salvo.platform.core.bytes.Bytes = found
        val length = (sizePlatform(data)).toLong()
        reply.send(Union2.U1<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>(ok(GetObjectOutput(body = __dep_salvo_stream_Streams.fromBytes(data), contentLength = length))))
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Faults", salvo.core.actor.__PROTO_Faults), Pair("Timer", salvo.time.__PROTO_Timer), Pair("TimerCtl", salvo.time.__PROTO_TimerCtl), Pair("Wheel", salvo.runtime.timers.__PROTO_Wheel)))
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val __h = MemFs()
    val __l = java.util.concurrent.locks.ReentrantLock()
    val fs: Fs = __Mon_Fs(__h, __l)
    val streams: salvo.stream.Streams = salvo.stream.__Mon_Streams(__h, __l)
    val written = writeStr(fs, streams, "notes.txt", "hello from Salvo\nsecond line\n")
    if (written is Union2.U2<*, *>) {
        println(console, "write: ${toStr(detach((written.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        return
    }
    println(console, "-- FakeS3 --")
    if (true) {
        val __h2 = FakeS3(streams)
        val __l2 = java.util.concurrent.locks.ReentrantLock()
        val s3: S3 = __Mon_S3(__h2, __l2)
        val s3_calls: S3Calls = __Mon_S3Calls(__h2, __l2)
        roundTrip(s3, fs, console, streams, "greeting.txt")
        upload(s3, fs, console, streams, "notes", "unsized.txt", "notes.txt", null)
        println(console, "calls: ${s3_calls.calls().joinToString(", ", "[", "]")}")
    }
    println(console, "-- MemS3 --")
    if (true) {
        val s32: S3 = __Mon_S3(MemS3(streams))
        roundTrip(s32, fs, console, streams, "greeting.txt")
        download(s32, fs, console, streams, "notes", "missing.txt", "missing.txt")
    }
}
