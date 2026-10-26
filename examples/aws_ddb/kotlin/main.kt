package salvo.main

import salvo.*

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun show(v: Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>): String {
    return when {
        (v is Union10.U1<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_1: salvo.aws.ddb.AttributeValue.S = ((v as Union10.U1<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.S)
            return "\"${v_1.value}\""
        }
        (v is Union10.U2<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_2: salvo.aws.ddb.AttributeValue.N = ((v as Union10.U2<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.N)
            return v_2.value
        }
        (v is Union10.U9<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_3: salvo.aws.ddb.AttributeValue.Bool = ((v as Union10.U9<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Bool)
            return "${v_3.value}"
        }
        (v is Union10.U10<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_4: salvo.aws.ddb.AttributeValue.Null = ((v as Union10.U10<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Null)
            return "null"
        }
        (v is Union10.U4<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_5: salvo.aws.ddb.AttributeValue.Ss = ((v as Union10.U4<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Ss)
            return "{${salvo.std.core.string.joinPlatform(v_5.value, ", ")}}"
        }
        (v is Union10.U5<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_6: salvo.aws.ddb.AttributeValue.Ns = ((v as Union10.U5<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Ns)
            return "{${salvo.std.core.string.joinPlatform(v_6.value, ", ")}}"
        }
        (v is Union10.U3<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_7: salvo.aws.ddb.AttributeValue.B = ((v as Union10.U3<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.B)
            return "<${salvo.std.core.bytes.sizePlatform(v_7.value)} bytes>"
        }
        (v is Union10.U6<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_8: salvo.aws.ddb.AttributeValue.Bs = ((v as Union10.U6<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Bs)
            return "<${salvo.std.core.list.sizePlatform(v_8.value)} blobs>"
        }
        (v is Union10.U8<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_9: salvo.aws.ddb.AttributeValue.L = ((v as Union10.U8<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.L)
            return "[${salvo.std.core.list.sizePlatform(v_9.value)} items]"
        }
        (v is Union10.U7<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_10: salvo.aws.ddb.AttributeValue.M = ((v as Union10.U7<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.M)
            return "{${salvo.std.core.map.sizePlatform(v_10.value)} attrs}"
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:22:5"))
        }
    }
}

fun showItem(item: salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>): String {
    val names: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    for (k in salvo.platform.std.core.list.each(salvo.std.core.map.keysPlatform(item))) {
        salvo.std.core.list.addPlatform(names, k)
    }
    val ks: List<String> = names
    val parts: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    for (k in salvo.platform.std.core.list.each(salvo.std.core.list.sort(ks, { __a0, __a1 -> salvo.__salvoCompare(__a0, __a1) }))) {
        val v: Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>? = salvo.std.core.map.getPlatform(item, k, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((v == null)) {
            continue
        }
        val v_1: Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null> = v!!
        salvo.std.core.list.addPlatform(parts, "${k}=${show(v_1)}")
    }
    val joined: List<String> = parts
    return "{${salvo.std.core.string.joinPlatform(joined, ", ")}}"
}

fun rebuild(item: salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>): salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>> {
    val out: salvo.platform.std.core.map.MutMap<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>> = salvo.std.core.map.Mut_Map_platform(arrayOf<kotlin.Pair<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    for (k in salvo.platform.std.core.map.each(item)) {
        val v: Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>? = salvo.std.core.map.getPlatform(item, k, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((v == null)) {
            continue
        }
        val v_1: Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null> = v!!
        salvo.std.core.map.putPlatform(out, k, v_1, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    }
    val result: salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>> = out
    return result
}

fun s(value: String): Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null> {
    return Union10.U1<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>(salvo.aws.ddb.AttributeValue.S(value = value))
}

fun n(value: String): Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null> {
    return Union10.U2<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>(salvo.aws.ddb.AttributeValue.N(value = value))
}

fun boolean(value: Boolean): Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null> {
    return Union10.U9<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>(salvo.aws.ddb.AttributeValue.Bool(value = value))
}

fun noTable(name: String): salvo.aws.ddb.DdbError {
    return salvo.aws.ddb.DdbError(code = "ResourceNotFoundException", message = "no table named ${name}", status = 400, requestId = null, item = null, throttlingReasons = null)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun roundTrip(ddb: salvo.aws.ddb.Ddb, console: salvo.std.core.console.Console, table: String) {
    val made: Union2<salvo.aws.ddb.CreateTableOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> kotlin.Pair(false, null) })
        ddb.createTable(salvo.aws.ddb.CreateTableInput(attributeDefinitions = listOf<salvo.aws.ddb.AttributeDefinition>(salvo.aws.ddb.AttributeDefinition(attributeName = "id", attributeType = "S")), tableName = table, keySchema = listOf<salvo.aws.ddb.KeySchemaElement>(salvo.aws.ddb.KeySchemaElement(attributeName = "id", keyType = "HASH")), localSecondaryIndexes = null, globalSecondaryIndexes = null, billingMode = null, provisionedThroughput = null, streamSpecification = null, sseSpecification = null, tags = null, tableClass = null, deletionProtectionEnabled = null, warmThroughput = null, resourcePolicy = null, onDemandThroughput = null, globalTableSourceArn = null, globalTableSettingsReplicationMode = null, vectorIndexes = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.ddb.CreateTableOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>
    }
    when {
        (made is Union2.U1<*, *>) -> {
            val made_1: salvo.aws.ddb.CreateTableOutput = ((made as Union2.U1<*, *>).value as salvo.aws.ddb.CreateTableOutput)
            salvo.std.core.console.println(console, "created table ${table}")
        }
        (made is Union2.U2<*, *>) -> {
            val made_2: salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>> = ((made as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>)
            salvo.std.core.console.println(console, "create_table: ${describe(salvo.std.core.checked.detach(made_2))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:107:5"))
        }
    }
    val item: salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>> = salvo.std.core.map.Map_platform(arrayOf<kotlin.Pair<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>>(kotlin.Pair("id", s("user-42")), kotlin.Pair("name", s("Ada")), kotlin.Pair("age", n("37")), kotlin.Pair("active", boolean(true))), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    salvo.std.core.console.println(console, "put ${showItem(item)}")
    val put: Union2<salvo.aws.ddb.PutItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> kotlin.Pair(false, null) })
        ddb.putItem(salvo.aws.ddb.PutItemInput(tableName = table, item = item, expected = null, returnValues = null, returnConsumedCapacity = null, returnItemCollectionMetrics = null, conditionalOperator = null, conditionExpression = null, expressionAttributeNames = null, expressionAttributeValues = null, returnValuesOnConditionCheckFailure = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.ddb.PutItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>
    }
    when {
        (put is Union2.U1<*, *>) -> {
            val put_3: salvo.aws.ddb.PutItemOutput = ((put as Union2.U1<*, *>).value as salvo.aws.ddb.PutItemOutput)
        }
        (put is Union2.U2<*, *>) -> {
            val put_4: salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>> = ((put as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>)
            salvo.std.core.console.println(console, "put_item: ${describe(salvo.std.core.checked.detach(put_4))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:124:5"))
        }
    }
    val got: Union2<salvo.aws.ddb.GetItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> kotlin.Pair(false, null) })
        ddb.getItem(salvo.aws.ddb.GetItemInput(tableName = table, key = salvo.std.core.map.Map_platform(arrayOf<kotlin.Pair<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>>(kotlin.Pair("id", s("user-42"))), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }), attributesToGet = null, consistentRead = null, returnConsumedCapacity = null, projectionExpression = null, expressionAttributeNames = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.ddb.GetItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>
    }
    when {
        (got is Union2.U1<*, *>) -> {
            val got_5: salvo.aws.ddb.GetItemOutput = ((got as Union2.U1<*, *>).value as salvo.aws.ddb.GetItemOutput)
            val found: salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>? = got_5.item
            if ((found == null)) {
                salvo.std.core.console.println(console, "got nothing")
            } else {
                val found_6: salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>> = found!!
                salvo.std.core.console.println(console, "got ${showItem(found_6)}")
            }
        }
        (got is Union2.U2<*, *>) -> {
            val got_7: salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>> = ((got as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>)
            salvo.std.core.console.println(console, "get_item: ${describe(salvo.std.core.checked.detach(got_7))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:133:5"))
        }
    }
    val gone: Union2<salvo.aws.ddb.DeleteItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>> = kotlin.run {
        val (r, __wid) = salvo.SalvoSched.waiter()
        salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> kotlin.Pair(false, null) })
        ddb.deleteItem(salvo.aws.ddb.DeleteItemInput(tableName = table, key = salvo.std.core.map.Map_platform(arrayOf<kotlin.Pair<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>>(kotlin.Pair("id", s("user-42"))), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }), expected = null, conditionalOperator = null, returnValues = null, returnConsumedCapacity = null, returnItemCollectionMetrics = null, conditionExpression = null, expressionAttributeNames = null, expressionAttributeValues = null, returnValuesOnConditionCheckFailure = null), r)
        salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.ddb.DeleteItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>
    }
    when {
        (gone is Union2.U1<*, *>) -> {
            val gone_8: salvo.aws.ddb.DeleteItemOutput = ((gone as Union2.U1<*, *>).value as salvo.aws.ddb.DeleteItemOutput)
            salvo.std.core.console.println(console, "deleted user-42")
        }
        (gone is Union2.U2<*, *>) -> {
            val gone_9: salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>> = ((gone as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>)
            salvo.std.core.console.println(console, "delete_item: ${describe(salvo.std.core.checked.detach(gone_9))}")
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:149:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun describe(e: Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>): String {
    if ((e is Union2.U2<*, *>)) {
        val e_1: salvo.aws.AwsError = ((e as Union2.U2<*, *>).value as salvo.aws.AwsError)
        return "${e_1.code}: ${e_1.message}"
    }
    val e_2: salvo.aws.ddb.DdbError = ((e as Union2.U1<*, *>).value as salvo.aws.ddb.DdbError)
    if ((e_2.code == "ResourceNotFoundException")) {
        return "no such table"
    }
    val e_3: salvo.aws.ddb.DdbError = ((e as Union2.U1<*, *>).value as salvo.aws.ddb.DdbError)
    return "the service refused: ${e_3.code}"
}

class MemDdb : salvo.aws.ddb.Ddb {
    var rows: salvo.platform.std.core.map.MutMap<String, salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>> = salvo.std.core.map.Mut_Map_platform(arrayOf<kotlin.Pair<String, salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>>>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    var tables: salvo.platform.std.core.set.MutSet<String> = salvo.std.core.set.Mut_Set_platform(arrayOf<String>(), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    override fun createTable(input: salvo.aws.ddb.CreateTableInput, reply: salvo.SalvoReply) {
        salvo.std.core.set.addPlatform(tables, input.tableName, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        reply.send(Union2.U1<salvo.aws.ddb.CreateTableOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.ddb.CreateTableOutput(tableDescription = null))))
    }
    override fun deleteTable(input: salvo.aws.ddb.DeleteTableInput, reply: salvo.SalvoReply) {
        if (!(salvo.std.core.set.containsPlatform(tables, input.tableName, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            reply.send(Union2.U2<salvo.aws.ddb.DeleteTableOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.aws.ddb.DdbError, salvo.aws.AwsError>(noTable(input.tableName))))))
            null
            return
        }
        salvo.std.core.set.removePlatform(tables, input.tableName, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        reply.send(Union2.U1<salvo.aws.ddb.DeleteTableOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.ddb.DeleteTableOutput(tableDescription = null))))
    }
    override fun putItem(input: salvo.aws.ddb.PutItemInput, reply: salvo.SalvoReply) {
        if (!(salvo.std.core.set.containsPlatform(tables, input.tableName, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            reply.send(Union2.U2<salvo.aws.ddb.PutItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.aws.ddb.DdbError, salvo.aws.AwsError>(noTable(input.tableName))))))
            null
            return
        }
        val rowKey: String = "${input.tableName}\t${keyText(input.item, "id")}"
        salvo.std.core.map.putPlatform(rows, rowKey, rebuild(input.item), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        reply.send(Union2.U1<salvo.aws.ddb.PutItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.ddb.PutItemOutput(attributes = null, consumedCapacity = null, itemCollectionMetrics = null))))
    }
    override fun getItem(input: salvo.aws.ddb.GetItemInput, reply: salvo.SalvoReply) {
        if (!(salvo.std.core.set.containsPlatform(tables, input.tableName, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            reply.send(Union2.U2<salvo.aws.ddb.GetItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.aws.ddb.DdbError, salvo.aws.AwsError>(noTable(input.tableName))))))
            null
            return
        }
        val rowKey: String = "${input.tableName}\t${keyText(input.key, "id")}"
        val row: salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>? = salvo.std.core.map.getPlatform(rows, rowKey, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        if ((row == null)) {
            reply.send(Union2.U1<salvo.aws.ddb.GetItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.ddb.GetItemOutput(item = null, consumedCapacity = null))))
            null
            return
        }
        val row_1: salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>> = row!!
        reply.send(Union2.U1<salvo.aws.ddb.GetItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.ddb.GetItemOutput(item = rebuild(row_1), consumedCapacity = null))))
    }
    override fun deleteItem(input: salvo.aws.ddb.DeleteItemInput, reply: salvo.SalvoReply) {
        if (!(salvo.std.core.set.containsPlatform(tables, input.tableName, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }))) {
            reply.send(Union2.U2<salvo.aws.ddb.DeleteItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.err(salvo.std.core.checked.checked(Union2.U1<salvo.aws.ddb.DdbError, salvo.aws.AwsError>(noTable(input.tableName))))))
            null
            return
        }
        val rowKey: String = "${input.tableName}\t${keyText(input.key, "id")}"
        if (salvo.std.core.map.containsKeyPlatform(rows, rowKey, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            salvo.std.core.map.removePlatform(rows, rowKey, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
        }
        reply.send(Union2.U1<salvo.aws.ddb.DeleteItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.ddb.DeleteItemOutput(attributes = null, consumedCapacity = null, itemCollectionMetrics = null))))
    }
    override fun query(input: salvo.aws.ddb.QueryInput, reply: salvo.SalvoReply) {
        reply.send(Union2.U1<salvo.aws.ddb.QueryOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>(salvo.std.core.result.ok(salvo.aws.ddb.QueryOutput(items = null, count = 0, scannedCount = 0, lastEvaluatedKey = null, consumedCapacity = null))))
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun keyText(item: salvo.platform.std.core.map.Map<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>, name: String): String {
    val found: Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>? = salvo.std.core.map.getPlatform(item, name, { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) })
    if ((found == null)) {
        return ""
    }
    val found_1: Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null> = found!!
    val v: Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null> = found_1
    return when {
        (v is Union10.U1<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_2: salvo.aws.ddb.AttributeValue.S = ((v as Union10.U1<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.S)
            return v_2.value
        }
        (v is Union10.U2<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_3: salvo.aws.ddb.AttributeValue.N = ((v as Union10.U2<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.N)
            return v_3.value
        }
        (v is Union10.U3<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_4: salvo.aws.ddb.AttributeValue.B = ((v as Union10.U3<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.B)
            return ""
        }
        (v is Union10.U4<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_5: salvo.aws.ddb.AttributeValue.Ss = ((v as Union10.U4<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Ss)
            return ""
        }
        (v is Union10.U5<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_6: salvo.aws.ddb.AttributeValue.Ns = ((v as Union10.U5<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Ns)
            return ""
        }
        (v is Union10.U6<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_7: salvo.aws.ddb.AttributeValue.Bs = ((v as Union10.U6<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Bs)
            return ""
        }
        (v is Union10.U7<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_8: salvo.aws.ddb.AttributeValue.M = ((v as Union10.U7<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.M)
            return ""
        }
        (v is Union10.U8<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_9: salvo.aws.ddb.AttributeValue.L = ((v as Union10.U8<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.L)
            return ""
        }
        (v is Union10.U9<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_10: salvo.aws.ddb.AttributeValue.Bool = ((v as Union10.U9<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Bool)
            return ""
        }
        (v is Union10.U10<*, *, *, *, *, *, *, *, *, *>) -> {
            val v_11: salvo.aws.ddb.AttributeValue.Null = ((v as Union10.U10<*, *, *, *, *, *, *, *, *, *>).value as salvo.aws.ddb.AttributeValue.Null)
            return ""
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:246:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    salvo.SalvoSched.setProtocols(kotlin.collections.listOf(kotlin.Pair("Faults", salvo.std.core.actor.__PROTO_Faults), kotlin.Pair("Timer", salvo.std.time.__PROTO_Timer), kotlin.Pair("TimerCtl", salvo.std.time.__PROTO_TimerCtl), kotlin.Pair("Wheel", salvo.std.runtime.timers.__PROTO_Wheel)))
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    salvo.std.core.console.println(__handle_2, "-- FakeDdb --")
    kotlin.run {
        val __use_3: salvo.aws.ddb.FakeDdb = salvo.aws.ddb.FakeDdb()
        val __lock___use_3 = java.util.concurrent.locks.ReentrantLock()
        val __handle_4: salvo.aws.ddb.Ddb = salvo.aws.ddb.__Mon_Ddb(__use_3, __lock___use_3)
        val __handle_5: salvo.aws.ddb.DdbCalls = salvo.aws.ddb.__Mon_DdbCalls(__use_3, __lock___use_3)
        roundTrip(__handle_4, __handle_2, "users")
        salvo.std.core.console.println(__handle_2, "calls: ${salvo.std.core.list.toStr(__handle_5.calls(), { __a0 -> __a0 })}")
    }
    salvo.std.core.console.println(__handle_2, "-- MemDdb --")
    kotlin.run {
        val __use_6: MemDdb = MemDdb()
        val __lock___use_6 = java.util.concurrent.locks.ReentrantLock()
        val __handle_7: salvo.aws.ddb.Ddb = salvo.aws.ddb.__Mon_Ddb(__use_6, __lock___use_6)
        roundTrip(__handle_7, __handle_2, "users")
        val missing: Union2<salvo.aws.ddb.GetItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>> = kotlin.run {
            val (r, __wid) = salvo.SalvoSched.waiter()
            salvo.SalvoSched.waiterDecoder(__wid, { _: ByteArray -> kotlin.Pair(false, null) })
            __handle_7.getItem(salvo.aws.ddb.GetItemInput(tableName = "nowhere", key = salvo.std.core.map.Map_platform(arrayOf<kotlin.Pair<String, Union10<salvo.aws.ddb.AttributeValue.S, salvo.aws.ddb.AttributeValue.N, salvo.aws.ddb.AttributeValue.B, salvo.aws.ddb.AttributeValue.Ss, salvo.aws.ddb.AttributeValue.Ns, salvo.aws.ddb.AttributeValue.Bs, salvo.aws.ddb.AttributeValue.M, salvo.aws.ddb.AttributeValue.L, salvo.aws.ddb.AttributeValue.Bool, salvo.aws.ddb.AttributeValue.Null>>>(kotlin.Pair("id", s("x"))), { __a0 -> (__a0).hashCode().toLong() }, { __a0, __a1 -> ((__a0) == (__a1)) }), attributesToGet = null, consistentRead = null, returnConsumedCapacity = null, projectionExpression = null, expressionAttributeNames = null), r)
            salvo.SalvoSched.awaitReply(__wid) as Union2<salvo.aws.ddb.GetItemOutput, salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>>
        }
        when {
            (missing is Union2.U1<*, *>) -> {
                val missing_8: salvo.aws.ddb.GetItemOutput = ((missing as Union2.U1<*, *>).value as salvo.aws.ddb.GetItemOutput)
                salvo.std.core.console.println(__handle_2, "unexpected ok")
            }
            (missing is Union2.U2<*, *>) -> {
                val missing_9: salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>> = ((missing as Union2.U2<*, *>).value as salvo.std.core.checked.Checked<Union2<salvo.aws.ddb.DdbError, salvo.aws.AwsError>>)
                salvo.std.core.console.println(__handle_2, "get_item: ${describe(salvo.std.core.checked.detach(missing_9))}")
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:281:9"))
            }
        }
    }
}

