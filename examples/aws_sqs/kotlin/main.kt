package salvo.main

import salvo.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun describe(e: Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>): String {
    if ((e is Union2.U2<*, *>)) {
        val e_1: salvo.aws.AwsError = ((e as Union2.U2<*, *>).value as salvo.aws.AwsError)
        return "${e_1.code}: ${e_1.message}"
    }
    val e_2: salvo.aws.sqs.SqsError = ((e as Union2.U1<*, *>).value as salvo.aws.sqs.SqsError)
    if ((e_2.code == "QueueDoesNotExist")) {
        return "no such queue"
    }
    val e_3: salvo.aws.sqs.SqsError = ((e as Union2.U1<*, *>).value as salvo.aws.sqs.SqsError)
    return "the service refused: ${e_3.code}"
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun roundTrip(sqs: salvo.aws.sqs.Sqs, console: salvo.core.console.Console, name: String) {
    val created: Union2<salvo.aws.sqs.CreateQueueOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.aws.sqs.__Codec_CreateQueueOutput, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError)))) })
        sqs.createQueue(salvo.aws.sqs.CreateQueueInput(queueName = name, attributes = null, tags = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.sqs.CreateQueueOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
    }
    if ((created is Union2.U2<*, *>)) {
        val created_1: salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((created as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
        salvo.core.console.println(console, "create_queue: ${describe(salvo.core.checked.detach(created_1))}")
        null
        return
    }
    val created_2: salvo.aws.sqs.CreateQueueOutput = ((created as Union2.U1<*, *>).value as salvo.aws.sqs.CreateQueueOutput)
    val url: String = run {
        val __elv_3: String? = created_2.queueUrl
        when {
            (__elv_3 == null) -> {
                "?"
            }
            else -> {
                val __some_4: String = __elv_3!!
                __some_4
            }
        }
    }
    salvo.core.console.println(console, "created ${url}")
    val sent: Union2<salvo.aws.sqs.SendMessageOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.aws.sqs.__Codec_SendMessageOutput, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError)))) })
        sqs.sendMessage(salvo.aws.sqs.SendMessageInput(queueUrl = url, messageBody = "hello from Salvo", delaySeconds = null, messageAttributes = null, messageSystemAttributes = null, messageDeduplicationId = null, messageGroupId = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.sqs.SendMessageOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
    }
    when {
        (sent is Union2.U1<*, *>) -> {
            val sent_5: salvo.aws.sqs.SendMessageOutput = ((sent as Union2.U1<*, *>).value as salvo.aws.sqs.SendMessageOutput)
            salvo.core.console.println(console, "sent ${run {
                val __elv_6: String? = sent_5.messageId
                when {
                    (__elv_6 == null) -> {
                        "?"
                    }
                    else -> {
                        val __some_7: String = __elv_6!!
                        __some_7
                    }
                }
            }}")
        }
        (sent is Union2.U2<*, *>) -> {
            val sent_8: salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((sent as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
            salvo.core.console.println(console, "send_message: ${describe(salvo.core.checked.detach(sent_8))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val got: Union2<salvo.aws.sqs.ReceiveMessageOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> Pair(false, null) })
        sqs.receiveMessage(salvo.aws.sqs.ReceiveMessageInput(queueUrl = url, attributeNames = null, messageSystemAttributeNames = null, messageAttributeNames = null, maxNumberOfMessages = 10, visibilityTimeout = null, waitTimeSeconds = null, receiveRequestAttemptId = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.sqs.ReceiveMessageOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
    }
    when {
        (got is Union2.U1<*, *>) -> {
            val got_9: salvo.aws.sqs.ReceiveMessageOutput = ((got as Union2.U1<*, *>).value as salvo.aws.sqs.ReceiveMessageOutput)
            val messages: List<salvo.aws.sqs.Message> = run {
                val __elv_10: List<salvo.aws.sqs.Message>? = got_9.messages
                when {
                    (__elv_10 == null) -> {
                        listOf<salvo.aws.sqs.Message>()
                    }
                    else -> {
                        val __some_11: List<salvo.aws.sqs.Message> = __elv_10!!
                        __some_11
                    }
                }
            }
            salvo.core.console.println(console, "received ${salvo.core.list.sizePlatform(messages)} message(s)")
            for (m in salvo.platform.core.list.each(messages)) {
                salvo.core.console.println(console, "  ${run {
                    val __elv_12: String? = m.body
                    when {
                        (__elv_12 == null) -> {
                            ""
                        }
                        else -> {
                            val __some_13: String = __elv_12!!
                            __some_13
                        }
                    }
                }}")
            }
        }
        (got is Union2.U2<*, *>) -> {
            val got_14: salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((got as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
            salvo.core.console.println(console, "receive_message: ${describe(salvo.core.checked.detach(got_14))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
    val gone: Union2<Unit, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.UnitCodec, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError)))) })
        sqs.deleteQueue(salvo.aws.sqs.DeleteQueueInput(queueUrl = url), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<Unit, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
    }
    when {
        (gone is Union2.U1<*, *>) -> {
            val gone_15: Unit = ((gone as Union2.U1<*, *>).value as Unit)
            salvo.core.console.println(console, "deleted")
        }
        (gone is Union2.U2<*, *>) -> {
            val gone_16: salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((gone as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
            salvo.core.console.println(console, "delete_queue: ${describe(salvo.core.checked.detach(gone_16))}")
        }
        else -> throw IllegalStateException("salvo: unreachable arm")
    }
}

fun noQueue(url: String): salvo.aws.sqs.SqsError {
    return salvo.aws.sqs.SqsError(code = "QueueDoesNotExist", message = "no queue at ${url}", status = 400, requestId = null)
}

class MemSqs : salvo.aws.sqs.Sqs {
    var queues: salvo.platform.core.map.MutMap<String, List<String>> = salvo.core.map.mutMapOfPlatform(arrayOf<Pair<String, List<String>>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    override fun createQueue(input: salvo.aws.sqs.CreateQueueInput, reply: salvo.SalvoReply) {
        val url: String = "mem://${input.queueName}"
        if (!(salvo.core.map.containsKeyPlatform(queues, url, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            salvo.core.map.putPlatform(queues, url, listOf<String>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        }
        salvo.SalvoSched.replyWire(reply, Union2.U1<salvo.aws.sqs.CreateQueueOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.ok(salvo.aws.sqs.CreateQueueOutput(queueUrl = url))), salvo.Union2Codec(salvo.aws.sqs.__Codec_CreateQueueOutput, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
    override fun getQueueUrl(input: salvo.aws.sqs.GetQueueUrlInput, reply: salvo.SalvoReply) {
        val url: String = "mem://${input.queueName}"
        if (!(salvo.core.map.containsKeyPlatform(queues, url, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<salvo.aws.sqs.GetQueueUrlOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U1<salvo.aws.sqs.SqsError, salvo.aws.AwsError>(noQueue(url))))), salvo.Union2Codec(salvo.aws.sqs.__Codec_GetQueueUrlOutput, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
            null
            return
        }
        salvo.SalvoSched.replyWire(reply, Union2.U1<salvo.aws.sqs.GetQueueUrlOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.ok(salvo.aws.sqs.GetQueueUrlOutput(queueUrl = url))), salvo.Union2Codec(salvo.aws.sqs.__Codec_GetQueueUrlOutput, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
    override fun sendMessage(input: salvo.aws.sqs.SendMessageInput, reply: salvo.SalvoReply) {
        val held: List<String>? = salvo.core.map.getPlatform(queues, input.queueUrl, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((held == null)) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<salvo.aws.sqs.SendMessageOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U1<salvo.aws.sqs.SqsError, salvo.aws.AwsError>(noQueue(input.queueUrl))))), salvo.Union2Codec(salvo.aws.sqs.__Codec_SendMessageOutput, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
            null
            return
        }
        val grown: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
        val held_1: List<String> = held!!
        for (b in salvo.platform.core.list.each(held_1)) {
            salvo.core.list.addPlatform(grown, b)
        }
        val id: String = "m${(salvo.core.list.sizePlatform(grown) + 1)}"
        salvo.core.list.addPlatform(grown, input.messageBody)
        val stored: List<String> = grown
        salvo.core.map.putPlatform(queues, input.queueUrl, stored, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.SalvoSched.replyWire(reply, Union2.U1<salvo.aws.sqs.SendMessageOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.ok(salvo.aws.sqs.SendMessageOutput(md5OfMessageBody = null, md5OfMessageAttributes = null, md5OfMessageSystemAttributes = null, messageId = id, sequenceNumber = null))), salvo.Union2Codec(salvo.aws.sqs.__Codec_SendMessageOutput, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
    override fun receiveMessage(input: salvo.aws.sqs.ReceiveMessageInput, reply: salvo.SalvoReply) {
        val held: List<String>? = salvo.core.map.getPlatform(queues, input.queueUrl, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((held == null)) {
            reply.send(Union2.U2<salvo.aws.sqs.ReceiveMessageOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U1<salvo.aws.sqs.SqsError, salvo.aws.AwsError>(noQueue(input.queueUrl))))))
            null
            return
        }
        val out: salvo.platform.core.list.MutList<salvo.aws.sqs.Message> = mutableListOf<salvo.aws.sqs.Message>()
        var i: Int = 0
        val held_1: List<String> = held!!
        for (b in salvo.platform.core.list.each(held_1)) {
            i = (i + 1)
            salvo.core.list.addPlatform(out, salvo.aws.sqs.Message(messageId = "m${i}", receiptHandle = "m${i}", md5OfBody = null, body = b, attributes = null, md5OfMessageAttributes = null, messageAttributes = null))
        }
        val messages: List<salvo.aws.sqs.Message> = out
        reply.send(Union2.U1<salvo.aws.sqs.ReceiveMessageOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.ok(salvo.aws.sqs.ReceiveMessageOutput(messages = messages))))
    }
    override fun deleteMessage(input: salvo.aws.sqs.DeleteMessageInput, reply: salvo.SalvoReply) {
        salvo.SalvoSched.replyWire(reply, Union2.U1<Unit, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
    override fun deleteQueue(input: salvo.aws.sqs.DeleteQueueInput, reply: salvo.SalvoReply) {
        if (!(salvo.core.map.containsKeyPlatform(queues, input.queueUrl, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            salvo.SalvoSched.replyWire(reply, Union2.U2<Unit, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.err(salvo.core.checked.checked(Union2.U1<salvo.aws.sqs.SqsError, salvo.aws.AwsError>(noQueue(input.queueUrl))))), salvo.Union2Codec(salvo.UnitCodec, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
            null
            return
        }
        salvo.core.map.removePlatform(queues, input.queueUrl, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        salvo.SalvoSched.replyWire(reply, Union2.U1<Unit, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>(salvo.core.result.ok(Unit)), salvo.Union2Codec(salvo.UnitCodec, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError))))
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(listOf(Pair("Faults", salvo.core.actor.__PROTO_Faults)))
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    salvo.core.console.println(__handle_2, "-- FakeSqs --")
    run {
        val __use_3: salvo.aws.sqs.FakeSqs = salvo.aws.sqs.FakeSqs()
        val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
        val __handle_4: salvo.aws.sqs.Sqs = salvo.aws.sqs.__Mon_Sqs(__use_3, __lock___use_3)
        val __handle_5: salvo.aws.sqs.SqsCalls = salvo.aws.sqs.__Mon_SqsCalls(__use_3, __lock___use_3)
        roundTrip(__handle_4, __handle_2, "orders")
        salvo.core.console.println(__handle_2, "calls: ${salvo.core.list.toStr(__handle_5.calls(), { __a0 -> __a0 })}")
    }
    salvo.core.console.println(__handle_2, "-- MemSqs --")
    run {
        val __use_6: MemSqs = MemSqs()
        val __lock___use_6 = java.util.concurrent.locks.ReentrantLock()
        val __handle_7: salvo.aws.sqs.Sqs = salvo.aws.sqs.__Mon_Sqs(__use_6, __lock___use_6)
        roundTrip(__handle_7, __handle_2, "orders")
        val missing: Union2<salvo.aws.sqs.GetQueueUrlOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>> = run {
            val (r, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), salvo.Union2Codec(salvo.aws.sqs.__Codec_GetQueueUrlOutput, salvo.core.checked.__Codec_Checked(salvo.Union2Codec(salvo.aws.sqs.__Codec_SqsError, salvo.aws.__Codec_AwsError)))) })
            __handle_7.getQueueUrl(salvo.aws.sqs.GetQueueUrlInput(queueName = "nowhere", queueOwnerAwsAccountId = null), r)
            salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.sqs.GetQueueUrlOutput, salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>>
        }
        when {
            (missing is Union2.U1<*, *>) -> {
                val missing_8: salvo.aws.sqs.GetQueueUrlOutput = ((missing as Union2.U1<*, *>).value as salvo.aws.sqs.GetQueueUrlOutput)
                salvo.core.console.println(__handle_2, "unexpected: ${run {
                    val __elv_9: String? = missing_8.queueUrl
                    when {
                        (__elv_9 == null) -> {
                            "?"
                        }
                        else -> {
                            val __some_10: String = __elv_9!!
                            __some_10
                        }
                    }
                }}")
            }
            (missing is Union2.U2<*, *>) -> {
                val missing_11: salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>> = ((missing as Union2.U2<*, *>).value as salvo.core.checked.Checked<Union2<salvo.aws.sqs.SqsError, salvo.aws.AwsError>>)
                salvo.core.console.println(__handle_2, "get_queue_url: ${describe(salvo.core.checked.detach(missing_11))}")
            }
            else -> throw IllegalStateException("salvo: unreachable arm")
        }
    }
}

