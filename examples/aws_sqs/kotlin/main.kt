package salvo.main

import salvo.*
import salvo.aws.*
import salvo.aws.sqs.*
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

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun describe(e: Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>): String {
    if (e is U21_21<*, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *>) {
        return "${(e.value as AwsError).code}: ${(e.value as AwsError).message}"
    }
    if (e is U21_16<*, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *, *>) {
        return "no such queue"
    }
    return "the service refused"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun round_trip(sqs: Sqs, console: Console, name: String) {
    val created = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(__Codec_CreateQueueOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError)))) })
        sqs.create_queue(CreateQueueInput(queue_name = name), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<CreateQueueOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>
    }
    if (created is U2_2<*, *>) {
        println(console, "create_queue: ${describe(detach((created.value as Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>)))}")
        return
    }
    val url = ((created.value as CreateQueueOutput).queue_url ?: "?")
    println(console, "created $url")
    val sent = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(__Codec_SendMessageOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError)))) })
        sqs.send_message(SendMessageInput(queue_url = url, message_body = "hello from Salvo"), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>
    }
    when (sent) {
        is U2_1<*, *> -> {
            println(console, "sent ${((sent.value as SendMessageOutput).message_id ?: "?")}")
        }
        is U2_2<*, *> -> {
            println(console, "send_message: ${describe(detach((sent.value as Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>)))}")
        }
    }
    val got = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> Pair(false, null) })
        sqs.receive_message(ReceiveMessageInput(queue_url = url, max_number_of_messages = 10), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<ReceiveMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>
    }
    when (got) {
        is U2_1<*, *> -> {
            val messages: List<Message> = ((got.value as ReceiveMessageOutput).messages ?: listOf<Message>())
            println(console, "received ${messages.size} message(s)")
            for (m in messages) {
                println(console, "  ${(m.body ?: "")}")
            }
        }
        is U2_2<*, *> -> {
            println(console, "receive_message: ${describe(detach((got.value as Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>)))}")
        }
    }
    val gone = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError)))) })
        sqs.delete_queue(DeleteQueueInput(queue_url = url), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<Unit, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>
    }
    when (gone) {
        is U2_1<*, *> -> {
            println(console, "deleted")
        }
        is U2_2<*, *> -> {
            println(console, "delete_queue: ${describe(detach((gone.value as Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>)))}")
        }
    }
}

class MemSqs : Sqs {
    private var queues: MutableMap<String, List<String>> = linkedMapOf<String, List<String>>().also { __m -> __m.putAll(listOf()) }

    override fun create_queue(input: CreateQueueInput, reply: salvo.SalvoReply) {
        val url = "mem://${input.queue_name}"
        if (!queues.containsKey(url)) {
            queues.put(url, listOf<String>())
        }
        salvo.SalvoSched.replyWire(reply, U2_1<CreateQueueOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(CreateQueueOutput(queue_url = url))), salvo.Union2Codec(__Codec_CreateQueueOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }

    override fun get_queue_url(input: GetQueueUrlInput, reply: salvo.SalvoReply) {
        val url = "mem://${input.queue_name}"
        if (!queues.containsKey(url)) {
            salvo.SalvoSched.replyWire(reply, U2_2<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(err(checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>(U21_16<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>(QueueDoesNotExist(message = url))))), salvo.Union2Codec(__Codec_GetQueueUrlOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
            return
        }
        salvo.SalvoSched.replyWire(reply, U2_1<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(GetQueueUrlOutput(queue_url = url))), salvo.Union2Codec(__Codec_GetQueueUrlOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }

    override fun send_message(input: SendMessageInput, reply: salvo.SalvoReply) {
        val held = queues[input.queue_url]
        if (held == null) {
            salvo.SalvoSched.replyWire(reply, U2_2<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(err(checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>(U21_16<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>(QueueDoesNotExist(message = input.queue_url))))), salvo.Union2Codec(__Codec_SendMessageOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
            return
        }
        val grown = mutableListOf<String>()
        for (b in held) {
            grown.add(b)
        }
        val id = "m${grown.size + 1}"
        grown.add(input.message_body)
        val stored: List<String> = grown
        queues.put(input.queue_url, stored)
        salvo.SalvoSched.replyWire(reply, U2_1<SendMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(SendMessageOutput(message_id = id))), salvo.Union2Codec(__Codec_SendMessageOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }

    override fun receive_message(input: ReceiveMessageInput, reply: salvo.SalvoReply) {
        val held = queues[input.queue_url]
        if (held == null) {
            reply.send(U2_2<ReceiveMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(err(checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>(U21_16<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>(QueueDoesNotExist(message = input.queue_url))))))
            return
        }
        val out = mutableListOf<Message>()
        var i = 0
        for (b in held) {
            i = i + 1
            out.add(Message(message_id = "m$i", receipt_handle = "m$i", body = b))
        }
        val messages: List<Message> = out
        reply.send(U2_1<ReceiveMessageOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(ReceiveMessageOutput(messages = messages))))
    }

    override fun delete_message(input: DeleteMessageInput, reply: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(reply, U2_1<Unit, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }

    override fun delete_queue(input: DeleteQueueInput, reply: salvo.SalvoReply) {
        if (!queues.containsKey(input.queue_url)) {
            salvo.SalvoSched.replyWire(reply, U2_2<Unit, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(err(checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>(U21_16<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>(QueueDoesNotExist(message = input.queue_url))))), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
            return
        }
        queues.remove(input.queue_url)
        salvo.SalvoSched.replyWire(reply, U2_1<Unit, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError))))
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Faults", salvo.core.actor.__PROTO_Faults)))
    val console: Console = StdOutConsole()
    println(console, "-- FakeSqs --")
    if (true) {
        val __h = FakeSqs()
        val sqs: Sqs = __Mon_Sqs(__h)
        val sqs_calls: SqsCalls = __Mon_SqsCalls(__h)
        round_trip(sqs, console, "orders")
        println(console, "calls: ${sqs_calls.calls().joinToString(", ", "[", "]")}")
    }
    println(console, "-- MemSqs --")
    if (true) {
        val sqs2: Sqs = __Mon_Sqs(MemSqs())
        round_trip(sqs2, console, "orders")
        val missing = run {
            val (r, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(__Codec_GetQueueUrlOutput, __Codec_Checked(salvo.Union21Codec(__Codec_InvalidAddress, __Codec_InvalidAttributeName, __Codec_InvalidAttributeValue, __Codec_InvalidIdFormat, __Codec_InvalidMessageContents, __Codec_InvalidSecurity, __Codec_KmsAccessDenied, __Codec_KmsDisabled, __Codec_KmsInvalidKeyUsage, __Codec_KmsInvalidState, __Codec_KmsNotFound, __Codec_KmsOptInRequired, __Codec_KmsThrottled, __Codec_OverLimit, __Codec_QueueDeletedRecently, __Codec_QueueDoesNotExist, __Codec_QueueNameExists, __Codec_ReceiptHandleIsInvalid, __Codec_RequestThrottled, __Codec_UnsupportedOperation, __Codec_AwsError)))) })
            sqs2.get_queue_url(GetQueueUrlInput(queue_name = "nowhere"), r)
            salvo.SalvoSched.awaitReply(__wid) as Union2<GetQueueUrlOutput, Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>>
        }
        when (missing) {
            is U2_1<*, *> -> {
                println(console, "unexpected: ${((missing.value as GetQueueUrlOutput).queue_url ?: "?")}")
            }
            is U2_2<*, *> -> {
                println(console, "get_queue_url: ${describe(detach((missing.value as Checked<Union21<InvalidAddress, InvalidAttributeName, InvalidAttributeValue, InvalidIdFormat, InvalidMessageContents, InvalidSecurity, KmsAccessDenied, KmsDisabled, KmsInvalidKeyUsage, KmsInvalidState, KmsNotFound, KmsOptInRequired, KmsThrottled, OverLimit, QueueDeletedRecently, QueueDoesNotExist, QueueNameExists, ReceiptHandleIsInvalid, RequestThrottled, UnsupportedOperation, AwsError>>)))}")
            }
        }
    }
}
