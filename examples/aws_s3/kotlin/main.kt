package salvo.main

import salvo.*
import salvo.aws.*
import salvo.aws.s3.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.console.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.fs.*
import salvo.fs.mem.*
import salvo.stream.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun describe(e: Union2<S3Error, AwsError>): String {
    if (e is U2_2<*, *>) {
        return "${(e.value as AwsError).code}: ${(e.value as AwsError).message}"
    }
    if (((e.value as S3Error).code == "NoSuchKey")) {
        return "no such key"
    }
    return "the service refused: ${((e.value as S3Error).code).toString()}"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun size_of(fs: Fs, streams: Streams, path: String): Long? {
    val info = fs.metadata(path)
    when (info) {
        is U2_1<*, *> -> {
            return (info.value as FileInfo).size
        }
        is U2_2<*, *> -> {
            ignore((info.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>))
            return null
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun upload(s3: S3, fs: Fs, console: Console, streams: Streams, bucket: String, key: String, path: String, length: Long?) {
    val opened = fs.open_read(path)
    if (opened is U2_2<*, *>) {
        println(console, "open $path: ${to_str(detach((opened.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        return
    }
    val put = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError)))) })
        s3.put_object(PutObjectInput(bucket = bucket, key = key, body = (opened.value as InStream), content_length = length), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>
    }
    when (put) {
        is U2_1<*, *> -> {
            println(console, "put $key: etag ${((put.value as PutObjectOutput).e_tag ?: "?")}")
        }
        is U2_2<*, *> -> {
            println(console, "put $key: ${describe(detach((put.value as Checked<Union2<S3Error, AwsError>>)))}")
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun download(s3: S3, fs: Fs, console: Console, streams: Streams, bucket: String, key: String, path: String) {
    val got = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> Pair(false, null) })
        s3.get_object(GetObjectInput(bucket = bucket, key = key), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>
    }
    if (got is U2_2<*, *>) {
        println(console, "get $key: ${describe(detach((got.value as Checked<Union2<S3Error, AwsError>>)))}")
        return
    }
    val __destructured1 = (got.value as GetObjectOutput)
    val body = __destructured1.body
    val content_length = __destructured1.content_length
    println(console, "get $key: ${(content_length ?: -1L)} bytes")
    val target = fs.open_write(path)
    if (target is U2_2<*, *>) {
        println(console, "open $path: ${to_str(detach((target.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        val closed = streams.close(body)
        if (closed is U2_2<*, *>) {
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
        is U2_1<*, *> -> {
            println(console, "piped ${(copied.value as Long)} bytes into $path")
        }
        is U2_2<*, *> -> {
            println(console, "pipe: ${to_str__4(detach((copied.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}")
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun round_trip(s3: S3, fs: Fs, console: Console, streams: Streams, key: String) {
    upload(s3, fs, console, streams, "notes", key, "notes.txt", size_of(fs, streams, "notes.txt"))
    download(s3, fs, console, streams, "notes", key, "back.txt")
    val back = read_to_str(fs, streams, "back.txt")
    when (back) {
        is U2_1<*, *> -> {
            println(console, "back.txt: ${(back.value as String).length} bytes")
            console.print((back.value as String))
        }
        is U2_2<*, *> -> {
            println(console, "back.txt: ${to_str(detach((back.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        }
    }
}

class MemS3(private val __dep_Streams: Streams) : S3 {
    private var objects: MutableMap<String, salvo.SalvoBytes> = linkedMapOf<String, salvo.SalvoBytes>().also { __m -> __m.putAll(listOf()) }

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun put_object(input: PutObjectInput, reply: salvo.SalvoReply) {
        val __destructured2 = input
        val bucket = __destructured2.bucket
        val key = __destructured2.key
        val body = __destructured2.body
        val buf = salvo.SalvoBytes.joined()
        val filled = fill_from(__dep_Streams, body, buf)
        val closed = __dep_Streams.close(body)
        if (closed is U2_2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        if (filled is U2_2<*, *>) {
            salvo.SalvoSched.replyWire(reply, U2_2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>(err(checked<Union2<S3Error, AwsError>>(U2_2<S3Error, AwsError>(AwsError(code = "StreamFailed", message = "${to_str__4(detach((filled.value as Checked<Union2<InvalidUtf8, StreamFailed>>)))}"))))), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError))))
            return
        }
        val data: salvo.SalvoBytes = buf
        val tag = "\"${data.size}\""
        objects.put("$bucket/$key", data)
        salvo.SalvoSched.replyWire(reply, U2_1<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>(ok(PutObjectOutput(e_tag = tag))), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union2Codec(__Codec_S3Error, __Codec_AwsError))))
    }

    override fun get_object(input: GetObjectInput, reply: salvo.SalvoReply) {
        val found = objects["${input.bucket}/${input.key}"]
        if (found == null) {
            reply.send(U2_2<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>(err(checked<Union2<S3Error, AwsError>>(U2_1<S3Error, AwsError>(S3Error(code = "NoSuchKey", message = "The specified key does not exist.", status = 404))))))
            return
        }
        val data: salvo.SalvoBytes = salvo.SalvoBytes(found)
        val length = (data.size).toLong()
        reply.send(U2_1<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>(ok(GetObjectOutput(body = __dep_Streams.from_bytes(data), content_length = length))))
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Faults", salvo.core.actor.__PROTO_Faults), Pair("Timer", salvo.time.__PROTO_Timer), Pair("TimerCtl", salvo.time.__PROTO_TimerCtl)))
    val console: Console = StdOutConsole()
    val __h = MemFs()
    val fs: Fs = __Mon_Fs(__h)
    val streams: Streams = __Mon_Streams(__h)
    val written = write_str(fs, streams, "notes.txt", "hello from Salvo\nsecond line\n")
    if (written is U2_2<*, *>) {
        println(console, "write: ${to_str(detach((written.value as Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>)))}")
        return
    }
    println(console, "-- FakeS3 --")
    if (true) {
        val __h2 = FakeS3(streams)
        val s3: S3 = __Mon_S3(__h2)
        val s3_calls: S3Calls = __Mon_S3Calls(__h2)
        round_trip(s3, fs, console, streams, "greeting.txt")
        upload(s3, fs, console, streams, "notes", "unsized.txt", "notes.txt", null)
        println(console, "calls: ${s3_calls.calls().joinToString(", ", "[", "]")}")
    }
    println(console, "-- MemS3 --")
    if (true) {
        val s32: S3 = __Mon_S3(MemS3(streams))
        round_trip(s32, fs, console, streams, "greeting.txt")
        download(s32, fs, console, streams, "notes", "missing.txt", "missing.txt")
    }
}
