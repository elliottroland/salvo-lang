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
    val queue_name: String,
    val attributes: Map<String, String>? = null,
    val tags: Map<String, String>? = null,
)

data class CreateQueueOutput(
    val queue_url: String? = null,
)

object __Codec_CreateQueueOutput : salvo.WireCodec<CreateQueueOutput> {
    override fun enc(v: CreateQueueOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.queue_url, out)
    }
    override fun dec(inp: salvo.WireIn): CreateQueueOutput = CreateQueueOutput(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetQueueUrlInput(
    val queue_name: String,
    val queue_owner_aws_account_id: String? = null,
)

object __Codec_GetQueueUrlInput : salvo.WireCodec<GetQueueUrlInput> {
    override fun enc(v: GetQueueUrlInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queue_name, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.queue_owner_aws_account_id, out)
    }
    override fun dec(inp: salvo.WireIn): GetQueueUrlInput = GetQueueUrlInput(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GetQueueUrlOutput(
    val queue_url: String? = null,
)

object __Codec_GetQueueUrlOutput : salvo.WireCodec<GetQueueUrlOutput> {
    override fun enc(v: GetQueueUrlOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.queue_url, out)
    }
    override fun dec(inp: salvo.WireIn): GetQueueUrlOutput = GetQueueUrlOutput(salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class SendMessageInput(
    val queue_url: String,
    val message_body: String,
    val delay_seconds: Int? = null,
    val message_attributes: Map<String, MessageAttributeValue>? = null,
    val message_system_attributes: Map<String, MessageSystemAttributeValue>? = null,
    val message_deduplication_id: String? = null,
    val message_group_id: String? = null,
)

data class MessageAttributeValue(
    val string_value: String? = null,
    val binary_value: salvo.SalvoBytes? = null,
    val string_list_values: List<String>? = null,
    val binary_list_values: List<salvo.SalvoBytes>? = null,
    val data_type: String,
)

object __Codec_MessageAttributeValue : salvo.WireCodec<MessageAttributeValue> {
    override fun enc(v: MessageAttributeValue, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.string_value, out)
        salvo.OptCodec(salvo.BytesCodec).enc(v.binary_value, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.string_list_values, out)
        salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).enc(v.binary_list_values, out)
        salvo.StrCodec.enc(v.data_type, out)
    }
    override fun dec(inp: salvo.WireIn): MessageAttributeValue = MessageAttributeValue(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.BytesCodec).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).dec(inp), salvo.StrCodec.dec(inp))
}

data class MessageSystemAttributeValue(
    val string_value: String? = null,
    val binary_value: salvo.SalvoBytes? = null,
    val string_list_values: List<String>? = null,
    val binary_list_values: List<salvo.SalvoBytes>? = null,
    val data_type: String,
)

object __Codec_MessageSystemAttributeValue : salvo.WireCodec<MessageSystemAttributeValue> {
    override fun enc(v: MessageSystemAttributeValue, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.string_value, out)
        salvo.OptCodec(salvo.BytesCodec).enc(v.binary_value, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.string_list_values, out)
        salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).enc(v.binary_list_values, out)
        salvo.StrCodec.enc(v.data_type, out)
    }
    override fun dec(inp: salvo.WireIn): MessageSystemAttributeValue = MessageSystemAttributeValue(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.BytesCodec).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.BytesCodec)).dec(inp), salvo.StrCodec.dec(inp))
}

data class SendMessageOutput(
    val md5_of_message_body: String? = null,
    val md5_of_message_attributes: String? = null,
    val md5_of_message_system_attributes: String? = null,
    val message_id: String? = null,
    val sequence_number: String? = null,
)

object __Codec_SendMessageOutput : salvo.WireCodec<SendMessageOutput> {
    override fun enc(v: SendMessageOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.md5_of_message_body, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.md5_of_message_attributes, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.md5_of_message_system_attributes, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.message_id, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sequence_number, out)
    }
    override fun dec(inp: salvo.WireIn): SendMessageOutput = SendMessageOutput(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class ReceiveMessageInput(
    val queue_url: String,
    val attribute_names: List<String>? = null,
    val message_system_attribute_names: List<String>? = null,
    val message_attribute_names: List<String>? = null,
    val max_number_of_messages: Int? = null,
    val visibility_timeout: Int? = null,
    val wait_time_seconds: Int? = null,
    val receive_request_attempt_id: String? = null,
)

object __Codec_ReceiveMessageInput : salvo.WireCodec<ReceiveMessageInput> {
    override fun enc(v: ReceiveMessageInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queue_url, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.attribute_names, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.message_system_attribute_names, out)
        salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).enc(v.message_attribute_names, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.max_number_of_messages, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.visibility_timeout, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.wait_time_seconds, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.receive_request_attempt_id, out)
    }
    override fun dec(inp: salvo.WireIn): ReceiveMessageInput = ReceiveMessageInput(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.ListCodec(salvo.StrCodec)).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class ReceiveMessageOutput(
    val messages: List<Message>? = null,
)

data class Message(
    val message_id: String? = null,
    val receipt_handle: String? = null,
    val md5_of_body: String? = null,
    val body: String? = null,
    val attributes: Map<String, String>? = null,
    val md5_of_message_attributes: String? = null,
    val message_attributes: Map<String, MessageAttributeValue>? = null,
)

data class DeleteMessageInput(
    val queue_url: String,
    val receipt_handle: String,
)

object __Codec_DeleteMessageInput : salvo.WireCodec<DeleteMessageInput> {
    override fun enc(v: DeleteMessageInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queue_url, out)
        salvo.StrCodec.enc(v.receipt_handle, out)
    }
    override fun dec(inp: salvo.WireIn): DeleteMessageInput = DeleteMessageInput(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class DeleteQueueInput(
    val queue_url: String,
)

object __Codec_DeleteQueueInput : salvo.WireCodec<DeleteQueueInput> {
    override fun enc(v: DeleteQueueInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.queue_url, out)
    }
    override fun dec(inp: salvo.WireIn): DeleteQueueInput = DeleteQueueInput(salvo.StrCodec.dec(inp))
}

data class SqsError(
    val code: String,
    val message: String,
    val status: Int,
    val request_id: String? = null,
)

object __Codec_SqsError : salvo.WireCodec<SqsError> {
    override fun enc(v: SqsError, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.code, out)
        salvo.StrCodec.enc(v.message, out)
        salvo.IntCodec.enc(v.status, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.request_id, out)
    }
    override fun dec(inp: salvo.WireIn): SqsError = SqsError(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

interface Sqs {
    fun create_queue(input: CreateQueueInput, reply: salvo.SalvoReply)
    fun get_queue_url(input: GetQueueUrlInput, reply: salvo.SalvoReply)
    fun send_message(input: SendMessageInput, reply: salvo.SalvoReply)
    fun receive_message(input: ReceiveMessageInput, reply: salvo.SalvoReply)
    fun delete_message(input: DeleteMessageInput, reply: salvo.SalvoReply)
    fun delete_queue(input: DeleteQueueInput, reply: salvo.SalvoReply)
}

class __Mon_Sqs(private val inner: Sqs) : Sqs {
    override fun create_queue(input: CreateQueueInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.create_queue(input, reply) }
    override fun get_queue_url(input: GetQueueUrlInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.get_queue_url(input, reply) }
    override fun send_message(input: SendMessageInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.send_message(input, reply) }
    override fun receive_message(input: ReceiveMessageInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.receive_message(input, reply) }
    override fun delete_message(input: DeleteMessageInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.delete_message(input, reply) }
    override fun delete_queue(input: DeleteQueueInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.delete_queue(input, reply) }
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

    override fun create_queue(input: CreateQueueInput, reply: salvo.SalvoReply) {
        recorded.add("create_queue")
        salvo.SalvoSched.replyWire(reply, U2_1<CreateQueueOutput, Checked<Union2<SqsError, AwsError>>>(ok(CreateQueueOutput())), salvo.Union2Codec(__Codec_CreateQueueOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun get_queue_url(input: GetQueueUrlInput, reply: salvo.SalvoReply) {
        recorded.add("get_queue_url")
        salvo.SalvoSched.replyWire(reply, U2_1<GetQueueUrlOutput, Checked<Union2<SqsError, AwsError>>>(ok(GetQueueUrlOutput())), salvo.Union2Codec(__Codec_GetQueueUrlOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun send_message(input: SendMessageInput, reply: salvo.SalvoReply) {
        recorded.add("send_message")
        salvo.SalvoSched.replyWire(reply, U2_1<SendMessageOutput, Checked<Union2<SqsError, AwsError>>>(ok(SendMessageOutput())), salvo.Union2Codec(__Codec_SendMessageOutput, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun receive_message(input: ReceiveMessageInput, reply: salvo.SalvoReply) {
        recorded.add("receive_message")
        reply.send(U2_1<ReceiveMessageOutput, Checked<Union2<SqsError, AwsError>>>(ok(ReceiveMessageOutput())))
    }

    override fun delete_message(input: DeleteMessageInput, reply: salvo.SalvoReply) {
        recorded.add("delete_message")
        salvo.SalvoSched.replyWire(reply, U2_1<Unit, Checked<Union2<SqsError, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun delete_queue(input: DeleteQueueInput, reply: salvo.SalvoReply) {
        recorded.add("delete_queue")
        salvo.SalvoSched.replyWire(reply, U2_1<Unit, Checked<Union2<SqsError, AwsError>>>(ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, __Codec_Checked(salvo.Union2Codec(__Codec_SqsError, __Codec_AwsError))))
    }

    override fun calls(): List<String> {
        return recorded.toMutableList()
    }
}
