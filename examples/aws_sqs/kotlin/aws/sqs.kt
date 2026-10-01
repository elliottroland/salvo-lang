package salvo.aws.sqs

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

data class CreateQueueInput(
    val queueName: String,
    val attributes: Map<String, String>? = null,
    val tags: Map<String, String>? = null,
)

data class CreateQueueOutput(
    val queueUrl: String? = null,
)

object __Codec_CreateQueueOutput : salvo.WireCodec<CreateQueueOutput> {
    override fun enc(v: CreateQueueOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.queueUrl, out)
    }
    override fun dec(inp: salvo.WireIn): CreateQueueOutput = CreateQueueOutput(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetQueueUrlInput(
    val queueName: String,
    val queueOwnerAwsAccountId: String? = null,
)

object __Codec_GetQueueUrlInput : salvo.WireCodec<GetQueueUrlInput> {
    override fun enc(v: GetQueueUrlInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queueName, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.queueOwnerAwsAccountId, out)
    }
    override fun dec(inp: salvo.WireIn): GetQueueUrlInput = GetQueueUrlInput(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetQueueUrlOutput(
    val queueUrl: String? = null,
)

object __Codec_GetQueueUrlOutput : salvo.WireCodec<GetQueueUrlOutput> {
    override fun enc(v: GetQueueUrlOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.queueUrl, out)
    }
    override fun dec(inp: salvo.WireIn): GetQueueUrlOutput = GetQueueUrlOutput(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class SendMessageInput(
    val queueUrl: String,
    val messageBody: String,
    val delaySeconds: Int? = null,
    val messageAttributes: Map<String, MessageAttributeValue>? = null,
    val messageSystemAttributes: Map<String, MessageSystemAttributeValue>? = null,
    val messageDeduplicationId: String? = null,
    val messageGroupId: String? = null,
)

data class MessageAttributeValue(
    val stringValue: String? = null,
    val binaryValue: salvo.SalvoBytes? = null,
    val stringListValues: List<String>? = null,
    val binaryListValues: List<salvo.SalvoBytes>? = null,
    val dataType: String,
)

object __Codec_MessageAttributeValue : salvo.WireCodec<MessageAttributeValue> {
    override fun enc(v: MessageAttributeValue, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.stringValue, out)
        salvo.OptCodec(salvo.BytesCodec).enc(v.binaryValue, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.stringListValues, out)
        salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).enc(v.binaryListValues, out)
        salvo.StrCodec.enc(v.dataType, out)
    }
    override fun dec(inp: salvo.WireIn): MessageAttributeValue = MessageAttributeValue(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.BytesCodec).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).dec(inp), salvo.StrCodec.dec(inp))
}

data class MessageSystemAttributeValue(
    val stringValue: String? = null,
    val binaryValue: salvo.SalvoBytes? = null,
    val stringListValues: List<String>? = null,
    val binaryListValues: List<salvo.SalvoBytes>? = null,
    val dataType: String,
)

object __Codec_MessageSystemAttributeValue : salvo.WireCodec<MessageSystemAttributeValue> {
    override fun enc(v: MessageSystemAttributeValue, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.stringValue, out)
        salvo.OptCodec(salvo.BytesCodec).enc(v.binaryValue, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.stringListValues, out)
        salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).enc(v.binaryListValues, out)
        salvo.StrCodec.enc(v.dataType, out)
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeValue = MessageSystemAttributeValue(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.BytesCodec).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).dec(inp), salvo.StrCodec.dec(inp))
}

data class SendMessageOutput(
    val md5OfMessageBody: String? = null,
    val md5OfMessageAttributes: String? = null,
    val md5OfMessageSystemAttributes: String? = null,
    val messageId: String? = null,
    val sequenceNumber: String? = null,
)

object __Codec_SendMessageOutput : salvo.WireCodec<SendMessageOutput> {
    override fun enc(v: SendMessageOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.md5OfMessageBody, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.md5OfMessageAttributes, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.md5OfMessageSystemAttributes, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.messageId, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sequenceNumber, out)
    }
    override fun dec(inp: salvo.WireIn): SendMessageOutput = SendMessageOutput(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class ReceiveMessageInput(
    val queueUrl: String,
    val attributeNames: List<String>? = null,
    val messageSystemAttributeNames: List<String>? = null,
    val messageAttributeNames: List<String>? = null,
    val maxNumberOfMessages: Int? = null,
    val visibilityTimeout: Int? = null,
    val waitTimeSeconds: Int? = null,
    val receiveRequestAttemptId: String? = null,
)

object __Codec_ReceiveMessageInput : salvo.WireCodec<ReceiveMessageInput> {
    override fun enc(v: ReceiveMessageInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queueUrl, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.attributeNames, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.messageSystemAttributeNames, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.messageAttributeNames, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.maxNumberOfMessages, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.visibilityTimeout, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.waitTimeSeconds, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.receiveRequestAttemptId, out)
    }
    override fun dec(inp: salvo.WireIn): ReceiveMessageInput = ReceiveMessageInput(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class ReceiveMessageOutput(
    val messages: List<Message>? = null,
)

data class Message(
    val messageId: String? = null,
    val receiptHandle: String? = null,
    val md5OfBody: String? = null,
    val body: String? = null,
    val attributes: Map<String, String>? = null,
    val md5OfMessageAttributes: String? = null,
    val messageAttributes: Map<String, MessageAttributeValue>? = null,
)

data class DeleteMessageInput(
    val queueUrl: String,
    val receiptHandle: String,
)

object __Codec_DeleteMessageInput : salvo.WireCodec<DeleteMessageInput> {
    override fun enc(v: DeleteMessageInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queueUrl, out)
        salvo.StrCodec.enc(v.receiptHandle, out)
    }
    override fun dec(inp: salvo.WireIn): DeleteMessageInput = DeleteMessageInput(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class DeleteQueueInput(
    val queueUrl: String,
)

object __Codec_DeleteQueueInput : salvo.WireCodec<DeleteQueueInput> {
    override fun enc(v: DeleteQueueInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queueUrl, out)
    }
    override fun dec(inp: salvo.WireIn): DeleteQueueInput = DeleteQueueInput(salvo.StrCodec.dec(inp))
}

data class SqsError(
    val code: String,
    val message: String,
    val status: Int,
    val requestId: String? = null,
)

object __Codec_SqsError : salvo.WireCodec<SqsError> {
    override fun enc(v: SqsError, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.code, out)
        salvo.StrCodec.enc(v.message, out)
        salvo.IntCodec.enc(v.status, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.requestId, out)
    }
    override fun dec(inp: salvo.WireIn): SqsError = SqsError(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

interface Sqs {
    fun createQueue(input: CreateQueueInput, reply: salvo.SalvoReply)
    fun getQueueUrl(input: GetQueueUrlInput, reply: salvo.SalvoReply)
    fun sendMessage(input: SendMessageInput, reply: salvo.SalvoReply)
    fun receiveMessage(input: ReceiveMessageInput, reply: salvo.SalvoReply)
    fun deleteMessage(input: DeleteMessageInput, reply: salvo.SalvoReply)
    fun deleteQueue(input: DeleteQueueInput, reply: salvo.SalvoReply)
}

class __Mon_Sqs(private val inner: Sqs) : Sqs {
    override fun createQueue(input: CreateQueueInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.createQueue(input, reply) }
    override fun getQueueUrl(input: GetQueueUrlInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.getQueueUrl(input, reply) }
    override fun sendMessage(input: SendMessageInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.sendMessage(input, reply) }
    override fun receiveMessage(input: ReceiveMessageInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.receiveMessage(input, reply) }
    override fun deleteMessage(input: DeleteMessageInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.deleteMessage(input, reply) }
    override fun deleteQueue(input: DeleteQueueInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.deleteQueue(input, reply) }
}

interface SqsCalls {
    fun calls(): List<String>
}

class __Mon_SqsCalls(private val inner: SqsCalls) : SqsCalls {
    override fun calls(): List<String> =
        synchronized(inner) { inner.calls() }
}

class FakeSqs : Sqs, SqsCalls {
    private var recorded: MutableList<String> = mutableListOf<String>()

    override fun createQueue(input: CreateQueueInput, reply: salvo.SalvoReply) {
        recorded.add("create_queue")
        salvo.SalvoSched.replyWire(reply, Union2.U1<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>(ok(CreateQueueOutput())), salvo.Union2Codec(__Codec_CreateQueueOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun getQueueUrl(input: GetQueueUrlInput, reply: salvo.SalvoReply) {
        recorded.add("get_queue_url")
        salvo.SalvoSched.replyWire(reply, Union2.U1<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>(ok(GetQueueUrlOutput())), salvo.Union2Codec(__Codec_GetQueueUrlOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun sendMessage(input: SendMessageInput, reply: salvo.SalvoReply) {
        recorded.add("send_message")
        salvo.SalvoSched.replyWire(reply, Union2.U1<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>(ok(SendMessageOutput())), salvo.Union2Codec(__Codec_SendMessageOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun receiveMessage(input: ReceiveMessageInput, reply: salvo.SalvoReply) {
        recorded.add("receive_message")
        reply.send(Union2.U1<ReceiveMessageOutput, Checked<Union2<SqsError, AwsError>>>(ok(ReceiveMessageOutput())))
    }

    override fun deleteMessage(input: DeleteMessageInput, reply: salvo.SalvoReply) {
        recorded.add("delete_message")
        salvo.SalvoSched.replyWire(reply, Union2.U1<Unit, Checked<Union2<SqsError, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun deleteQueue(input: DeleteQueueInput, reply: salvo.SalvoReply) {
        recorded.add("delete_queue")
        salvo.SalvoSched.replyWire(reply, Union2.U1<Unit, Checked<Union2<SqsError, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun calls(): List<String> {
        return recorded.toMutableList()
    }
}
