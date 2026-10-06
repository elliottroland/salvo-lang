package salvo.main

import salvo.*
import salvo.aws.AwsError
import salvo.aws.__Codec_AwsError
import salvo.aws.sqs.CreateQueueInput
import salvo.aws.sqs.CreateQueueOutput
import salvo.aws.sqs.DeleteMessageInput
import salvo.aws.sqs.DeleteQueueInput
import salvo.aws.sqs.FakeSqs
import salvo.aws.sqs.GetQueueUrlInput
import salvo.aws.sqs.GetQueueUrlOutput
import salvo.aws.sqs.Message
import salvo.aws.sqs.ReceiveMessageInput
import salvo.aws.sqs.ReceiveMessageOutput
import salvo.aws.sqs.SendMessageInput
import salvo.aws.sqs.SendMessageOutput
import salvo.aws.sqs.Sqs
import salvo.aws.sqs.SqsCalls
import salvo.aws.sqs.SqsError
import salvo.aws.sqs.__Codec_CreateQueueOutput
import salvo.aws.sqs.__Codec_GetQueueUrlOutput
import salvo.aws.sqs.__Codec_SendMessageOutput
import salvo.aws.sqs.__Codec_SqsError
import salvo.aws.sqs.__Mon_Sqs
import salvo.aws.sqs.__Mon_SqsCalls
import salvo.core.actor.eq
import salvo.core.checked.Checked
import salvo.core.checked.__Codec_Checked
import salvo.core.checked.checked
import salvo.core.checked.detach
import salvo.core.console.Console
import salvo.core.console.__Platform_StdOutConsole
import salvo.core.console.println
import salvo.core.deque.get
import salvo.core.list.addPlatform
import salvo.core.list.get
import salvo.core.list.sizePlatform
import salvo.core.list.toStr
import salvo.core.map.containsKeyPlatform
import salvo.core.map.eq
import salvo.core.map.get
import salvo.core.map.getPlatform
import salvo.core.map.mutMapOfPlatform
import salvo.core.map.putPlatform
import salvo.core.map.removePlatform
import salvo.core.result.err
import salvo.core.result.ok
import salvo.core.set.eq

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun describe(e: Union2<SqsError, AwsError>): String {
    if (e is Union2.U2<*, *>) {
        return "${(e.value as AwsError).code}: ${(e.value as AwsError).message}"
    }
    if (((e.value as SqsError).code == "QueueDoesNotExist")) {
        return "no such queue"
    }
    return "the service refused: ${(e.value as SqsError).code}"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun roundTrip(sqs: Sqs, console: Console, name: String) {
    val created = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(__Codec_CreateQueueOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError)))) })
        sqs.createQueue(CreateQueueInput(queueName = name), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>
    }
    if (created is Union2.U2<*, *>) {
        println(console, "create_queue: ${describe(detach((created.value as Checked<Union2<SqsError, AwsError>>)))}")
        return
    }
    val url = ((created.value as CreateQueueOutput).queueUrl ?: "?")
    println(console, "created $url")
    val sent = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(__Codec_SendMessageOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError)))) })
        sqs.sendMessage(SendMessageInput(queueUrl = url, messageBody = "hello from Salvo"), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>
    }
    when (sent) {
        is Union2.U1<*, *> -> {
            println(console, "sent ${((sent.value as SendMessageOutput).messageId ?: "?")}")
        }
        is Union2.U2<*, *> -> {
            println(console, "send_message: ${describe(detach((sent.value as Checked<Union2<SqsError, AwsError>>)))}")
        }
    }
    val got = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> Pair(false, null) })
        sqs.receiveMessage(ReceiveMessageInput(queueUrl = url, maxNumberOfMessages = 10), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<ReceiveMessageOutput, Checked<Union2<SqsError, AwsError>>>
    }
    when (got) {
        is Union2.U1<*, *> -> {
            val messages: List<Message> = ((got.value as ReceiveMessageOutput).messages ?: listOf<Message>())
            println(console, "received ${sizePlatform(messages)} message(s)")
            for (m in salvo.platform.core.list.each(messages)) {
                println(console, "  ${(m.body ?: "")}")
            }
        }
        is Union2.U2<*, *> -> {
            println(console, "receive_message: ${describe(detach((got.value as Checked<Union2<SqsError, AwsError>>)))}")
        }
    }
    val gone = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError)))) })
        sqs.deleteQueue(DeleteQueueInput(queueUrl = url), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<Unit, Checked<Union2<SqsError, AwsError>>>
    }
    when (gone) {
        is Union2.U1<*, *> -> {
            println(console, "deleted")
        }
        is Union2.U2<*, *> -> {
            println(console, "delete_queue: ${describe(detach((gone.value as Checked<Union2<SqsError, AwsError>>)))}")
        }
    }
}

fun noQueue(url: String): SqsError {
    return SqsError(code = "QueueDoesNotExist", message = "no queue at $url", status = 400)
}

class MemSqs : Sqs {
    private var queues: salvo.platform.core.map.MutMap<String, List<String>> = mutMapOfPlatform(arrayOf(), { __i0 -> (__i0).hashCode().toLong() }, { __i0, __i1 -> ((__i0) == (__i1)) })

    override fun createQueue(input: CreateQueueInput, reply: salvo.SalvoReply) {
        val url = "mem://${input.queueName}"
        if (!containsKeyPlatform(queues, url, { __i0 -> (__i0).hashCode().toLong() }, { __i0, __i1 -> ((__i0) == (__i1)) })) {
            putPlatform(queues, url, listOf<String>(), { __i0 -> (__i0).hashCode().toLong() }, { __i0, __i1 -> ((__i0) == (__i1)) })
        }
        salvo.SalvoSched.replyWire(reply, Union2.U1<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>(ok(CreateQueueOutput(queueUrl = url))), salvo.Union2Codec(__Codec_CreateQueueOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun getQueueUrl(input: GetQueueUrlInput, reply: salvo.SalvoReply) {
        val url = "mem://${input.queueName}"
        if (!containsKeyPlatform(queues, url, { __i0 -> (__i0).hashCode().toLong() }, { __i0, __i1 -> ((__i0) == (__i1)) })) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>(err(checked<Union2<SqsError, AwsError>>(Union2.U1<SqsError, AwsError>(noQueue(url))))), salvo.Union2Codec(__Codec_GetQueueUrlOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
            return
        }
        salvo.SalvoSched.replyWire(reply, Union2.U1<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>(ok(GetQueueUrlOutput(queueUrl = url))), salvo.Union2Codec(__Codec_GetQueueUrlOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun sendMessage(input: SendMessageInput, reply: salvo.SalvoReply) {
        val held = getPlatform(queues, input.queueUrl, { __i0 -> (__i0).hashCode().toLong() }, { __i0, __i1 -> ((__i0) == (__i1)) })
        if (held == null) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>(err(checked<Union2<SqsError, AwsError>>(Union2.U1<SqsError, AwsError>(noQueue(input.queueUrl))))), salvo.Union2Codec(__Codec_SendMessageOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
            return
        }
        val grown = mutableListOf<String>()
        for (b in salvo.platform.core.list.each(held)) {
            addPlatform(grown, b)
        }
        val id = "m${sizePlatform(grown) + 1}"
        addPlatform(grown, input.messageBody)
        val stored: List<String> = grown
        putPlatform(queues, input.queueUrl, stored, { __i0 -> (__i0).hashCode().toLong() }, { __i0, __i1 -> ((__i0) == (__i1)) })
        salvo.SalvoSched.replyWire(reply, Union2.U1<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>(ok(SendMessageOutput(messageId = id))), salvo.Union2Codec(__Codec_SendMessageOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun receiveMessage(input: ReceiveMessageInput, reply: salvo.SalvoReply) {
        val held = getPlatform(queues, input.queueUrl, { __i0 -> (__i0).hashCode().toLong() }, { __i0, __i1 -> ((__i0) == (__i1)) })
        if (held == null) {
            reply.send(Union2.U2<ReceiveMessageOutput, Checked<Union2<SqsError, AwsError>>>(err(checked<Union2<SqsError, AwsError>>(Union2.U1<SqsError, AwsError>(noQueue(input.queueUrl))))))
            return
        }
        val out = mutableListOf<Message>()
        var i = 0
        for (b in salvo.platform.core.list.each(held)) {
            i = i + 1
            addPlatform(out, Message(messageId = "m$i", receiptHandle = "m$i", body = b))
        }
        val messages: List<Message> = out
        reply.send(Union2.U1<ReceiveMessageOutput, Checked<Union2<SqsError, AwsError>>>(ok(ReceiveMessageOutput(messages = messages))))
    }

    override fun deleteMessage(input: DeleteMessageInput, reply: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(reply, Union2.U1<Unit, Checked<Union2<SqsError, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun deleteQueue(input: DeleteQueueInput, reply: salvo.SalvoReply) {
        if (!containsKeyPlatform(queues, input.queueUrl, { __i0 -> (__i0).hashCode().toLong() }, { __i0, __i1 -> ((__i0) == (__i1)) })) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<Unit, Checked<Union2<SqsError, AwsError>>>(err(checked<Union2<SqsError, AwsError>>(Union2.U1<SqsError, AwsError>(noQueue(input.queueUrl))))), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
            return
        }
        removePlatform(queues, input.queueUrl, { __i0 -> (__i0).hashCode().toLong() }, { __i0, __i1 -> ((__i0) == (__i1)) })
        salvo.SalvoSched.replyWire(reply, Union2.U1<Unit, Checked<Union2<SqsError, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Faults", salvo.core.actor.__PROTO_Faults)))
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    println(console, "-- FakeSqs --")
    if (true) {
        val __h = FakeSqs()
        val __l = java.util.concurrent.locks.ReentrantLock()
        val sqs: Sqs = __Mon_Sqs(__h, __l)
        val sqs_calls: SqsCalls = __Mon_SqsCalls(__h, __l)
        roundTrip(sqs, console, "orders")
        println(console, "calls: ${toStr(sqs_calls.calls(), { __i0 -> __i0 })}")
    }
    println(console, "-- MemSqs --")
    if (true) {
        val sqs2: Sqs = __Mon_Sqs(MemSqs())
        roundTrip(sqs2, console, "orders")
        val missing = run {
            val (r, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(__Codec_GetQueueUrlOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError)))) })
            sqs2.getQueueUrl(GetQueueUrlInput(queueName = "nowhere"), r)
            salvo.SalvoSched.awaitReply(__wid) as Union2<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>
        }
        when (missing) {
            is Union2.U1<*, *> -> {
                println(console, "unexpected: ${((missing.value as GetQueueUrlOutput).queueUrl ?: "?")}")
            }
            is Union2.U2<*, *> -> {
                println(console, "get_queue_url: ${describe(detach((missing.value as Checked<Union2<SqsError, AwsError>>)))}")
            }
        }
    }
}
