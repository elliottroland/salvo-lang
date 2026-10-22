// Amazon DynamoDB from Salvo, with no AWS anywhere: the program is written
// against the generated `Ddb` effect, and two handlers that need no SDK stand
// in for the service.
//
// `aws.ddb` is generated from the service's Smithy model
// (`modules/aws/codegen`). Its central type, `AttributeValue`, is a Smithy
// union — a value is one of ten wire types — so the generator emits it as a
// **namespace**: `AttributeValue.S`, `AttributeValue.N`, `AttributeValue.M`,
// and so on, with the union `type AttributeValue = AttributeValue.S | …` over
// them. An item is a `Map<Str, AttributeValue>`.
//
// Nothing here imports `aws.ddb.host`, so the build reaches no host glue and
// needs no SDK.

import aws
import aws.ddb

// Render one attribute value, in a line. The `.Thing` shorthand names an arm
// of `AttributeValue` by a suffix of its namespaced type — `.S` is
// `AttributeValue.S`, unambiguous because only one arm ends in `S`.
fn show(v: AttributeValue) [] -> Str => v {
    when v {
        is .S { return "\"${v.value}\"" }
        is .N { return v.value }
        is .Bool { return "${v.value}" }
        is .Null { return "null" }
        is .Ss { return "{${join(v.value, ", ")}}" }
        is .Ns { return "{${join(v.value, ", ")}}" }
        is .B { return "<${size(v.value)} bytes>" }
        is .Bs { return "<${size(v.value)} blobs>" }
        is .L { return "[${size(v.value)} items]" }
        is .M { return "{${size(v.value)} attrs}" }
    }
}

// Render an item (a map of named attributes) in a stable order.
fn show_item(item: Map<Str, AttributeValue>) [] -> Str => item {
    let names = Mut List<Str>()
    for k in keys(item) {
        names.add(copy(k))
    }
    let ks: List<Str> = names
    let parts = Mut List<Str>()
    for k in sort(ks) {
        let v = get(item, k)
        if v is None {
            continue
        }
        parts.add("${k}=${show(copy(v))}")
    }
    let joined: List<Str> = parts
    return "{${join(joined, ", ")}}"
}

// A fresh owned copy of an item, built entry by entry. Copying each
// `AttributeValue` (a union of immutable arms) is supported on both backends,
// where copying the whole `Map` at once is not yet on Kotlin.
fn rebuild(item: Map<Str, AttributeValue>) [] -> Map<Str, AttributeValue> => item {
    let out: Mut Map<Str, AttributeValue> = Mut Map()
    for k in item {
        let v = get(item, k)
        if v is None {
            continue
        }
        put(out, copy(k), copy(v))
    }
    let result: Map<Str, AttributeValue> = out
    return result
}

// A string attribute. The `.S` shorthand names the arm by its namespace
// suffix — the full type is `AttributeValue.S`, left to the return type.
fn s(value: Str) [] -> AttributeValue => !value {
    return .S { value: value }
}

// A number attribute (DynamoDB numbers travel as decimal strings).
fn n(value: Str) [] -> AttributeValue => !value {
    return .N { value: value }
}

// A boolean attribute.
fn boolean(value: Bool) [] -> AttributeValue => value {
    return .Bool { value: value }
}

// The error the service answers for a table it does not have.
fn no_table(name: Str) [] -> DdbError => !name {
    return DdbError {
        code: "ResourceNotFoundException",
        message: "no table named ${name}",
        status: 400
    }
}

// A one-key item: the partition key `id` plus the given attributes, which the
// test workflow puts and gets back.
fn round_trip(table: Str) [Ddb, Console] => table {
    // Create the table so the double has somewhere to put the item.
    let made = waitfor r: Reply<Ok CreateTableOutput | Err Checked<DdbFailure>> {
        create_table(CreateTableInput {
            table_name: copy(table),
            key_schema: List(KeySchemaElement { attribute_name: "id", key_type: "HASH" }),
            attribute_definitions: List(AttributeDefinition { attribute_name: "id", attribute_type: "S" })
        }, r)
    }
    when made {
        is Ok { println("created table ${table}") }
        is Err { println("create_table: ${describe(detach(made))}") }
    }

    // Put an item: a map of named `AttributeValue`s.
    let item = Map(
        ("id", s("user-42")),
        ("name", s("Ada")),
        ("age", n("37")),
        ("active", boolean(true))
    )
    // Show the item (a kept read), then move it into the put.
    println("put ${show_item(item)}")
    let put = waitfor r: Reply<Ok PutItemOutput | Err Checked<DdbFailure>> {
        put_item(PutItemInput { table_name: copy(table), item: item }, r)
    }
    when put {
        is Ok {}
        is Err { println("put_item: ${describe(detach(put))}") }
    }

    // Get it back by key.
    let got = waitfor r: Reply<Ok GetItemOutput | Err Checked<DdbFailure>> {
        get_item(GetItemInput { table_name: copy(table), key: Map(("id", s("user-42"))) }, r)
    }
    when got {
        is Ok {
            let found = got.item
            if found is None {
                println("got nothing")
            } else {
                println("got ${show_item(found)}")
            }
        }
        is Err { println("get_item: ${describe(detach(got))}") }
    }

    // Delete it.
    let gone = waitfor r: Reply<Ok DeleteItemOutput | Err Checked<DdbFailure>> {
        delete_item(DeleteItemInput { table_name: copy(table), key: Map(("id", s("user-42"))) }, r)
    }
    when gone {
        is Ok { println("deleted user-42") }
        is Err { println("delete_item: ${describe(detach(gone))}") }
    }
}

// What a failure says, in a line.
fn describe(e: DdbFailure) [] -> Str => e {
    if e is AwsError {
        return "${e.code}: ${e.message}"
    }
    if e.code is "ResourceNotFoundException" {
        return "no such table"
    }
    return "the service refused: ${e.code}"
}

// DynamoDB in memory, written the way a test would: a flat map from a
// `table\tid` key to the whole item, with the set of known tables alongside.
// The generated effect is the contract, so any handler of it will do.
handler MemDdb() of Ddb {
    state {
        rows: Mut Map<Str, Map<Str, AttributeValue>> = Mut Map()
        tables: Mut Set<Str> = Mut Set()
    }

    fn create_table(input: CreateTableInput, reply: Reply<Ok CreateTableOutput | Err Checked<DdbFailure>>) -> None
    => !input, !reply {
        tables.add(copy(input.table_name))
        reply.send(ok(CreateTableOutput {}))
    }

    fn delete_table(input: DeleteTableInput, reply: Reply<Ok DeleteTableOutput | Err Checked<DdbFailure>>) -> None
    => !input, !reply {
        if !contains(tables, input.table_name) {
            reply.send(err(checked<DdbFailure>(no_table(copy(input.table_name)))))
            return None
        }
        remove(tables, input.table_name)
        reply.send(ok(DeleteTableOutput {}))
    }

    fn put_item(input: PutItemInput, reply: Reply<Ok PutItemOutput | Err Checked<DdbFailure>>) -> None
    => !input, !reply {
        if !contains(tables, input.table_name) {
            reply.send(err(checked<DdbFailure>(no_table(copy(input.table_name)))))
            return None
        }
        let row_key = "${input.table_name}\t${key_text(input.item, "id")}"
        put(rows, row_key, rebuild(input.item))
        reply.send(ok(PutItemOutput {}))
    }

    fn get_item(input: GetItemInput, reply: Reply<Ok GetItemOutput | Err Checked<DdbFailure>>) -> None
    => !input, !reply {
        if !contains(tables, input.table_name) {
            reply.send(err(checked<DdbFailure>(no_table(copy(input.table_name)))))
            return None
        }
        let row_key = "${input.table_name}\t${key_text(input.key, "id")}"
        let row = get(rows, row_key)
        if row is None {
            reply.send(ok(GetItemOutput {}))
            return None
        }
        reply.send(ok(GetItemOutput { item: rebuild(row) }))
    }

    fn delete_item(input: DeleteItemInput, reply: Reply<Ok DeleteItemOutput | Err Checked<DdbFailure>>) -> None
    => !input, !reply {
        if !contains(tables, input.table_name) {
            reply.send(err(checked<DdbFailure>(no_table(copy(input.table_name)))))
            return None
        }
        let row_key = "${input.table_name}\t${key_text(input.key, "id")}"
        if contains_key(rows, row_key) {
            remove(rows, row_key)
        }
        reply.send(ok(DeleteItemOutput {}))
    }

    fn query(input: QueryInput, reply: Reply<Ok QueryOutput | Err Checked<DdbFailure>>) -> None
    => !input, !reply {
        // The double does not implement query expressions; a test that needs
        // them implements `Ddb` itself.
        reply.send(ok(QueryOutput {}))
    }
}

// The string value of a key attribute, for the mem double's row id. Every key
// here is a string (`S`); a non-string key maps to the empty string.
fn key_text(item: Map<Str, AttributeValue>, name: Str) [] -> Str => item, !name {
    let found = get(item, name)
    if found is None {
        return ""
    }
    let v: AttributeValue = copy(found)
    when v {
        is .S { return copy(v.value) }
        is .N { return copy(v.value) }
        is .B { return "" }
        is .Ss { return "" }
        is .Ns { return "" }
        is .Bs { return "" }
        is .M { return "" }
        is .L { return "" }
        is .Bool { return "" }
        is .Null { return "" }
    }
}

fn main() [use] {
    use StdConsole()

    // 1. The generated recording fake: every call answered as an empty success
    //    where the output allows one, and noted by name.
    println("-- FakeDdb --")
    if true {
        use FakeDdb()
        round_trip("users")
        println("calls: ${calls()}")
    }

    // 2. A double with real item semantics.
    println("-- MemDdb --")
    if true {
        use MemDdb()
        round_trip("users")
        // A modeled error: a table that was never created.
        let missing = waitfor r: Reply<Ok GetItemOutput | Err Checked<DdbFailure>> {
            get_item(GetItemInput { table_name: "nowhere", key: Map(("id", s("x"))) }, r)
        }
        when missing {
            is Ok { println("unexpected ok") }
            is Err { println("get_item: ${describe(detach(missing))}") }
        }
    }
}
