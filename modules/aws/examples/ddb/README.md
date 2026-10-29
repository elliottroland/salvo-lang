# aws example: ddb

Amazon DynamoDB from Salvo, with no AWS anywhere. `aws.ddb` is **generated**
from the service's Smithy model by `modules/aws/codegen`; this program is
written against its `Ddb` effect and runs against two handlers that need no
SDK.

Run it:

```bash
cd modules/aws/examples/ddb
cargo run -q --manifest-path ../../../../Cargo.toml -- run
```

## What to look for

**`AttributeValue` is a namespace.** DynamoDB's central type is a Smithy
*union* — an attribute is one of ten wire types (`S`, `N`, `B`, `M`, `L`, …) —
so the generator emits it as a `namespace`:

```
export namespace AttributeValue {
    export struct S { value: Str }
    export struct N { value: Str }
    export struct M { value: Map<Str, AttributeValue> }
    …
}
export type AttributeValue = AttributeValue.S | AttributeValue.N | …
```

An item is a `Map<Str, AttributeValue>`. The program builds one with a small
helper per arm (`s("Ada")` is `AttributeValue.S { value: "Ada" }`), and reads
one back in `show`, where the **leading-dot shorthand** names an arm by a
suffix of its namespaced type — `is .S` is `is AttributeValue.S`, unambiguous
because only one arm ends in `S`:

```
when v {
    is .S { return "\"${v.value}\"" }
    is .N { return v.value }
    …
}
```

**Two handlers, no SDK.** `FakeDdb` (generated) records each call and answers an
empty success, so its `get` returns nothing. `MemDdb` (hand-written) keeps items
in memory, so the put/get round-trips the whole item, and a table that was
never created answers the modeled `ResourceNotFoundException`.

Nothing here imports `aws.ddb.host`, so the build reaches no host glue and needs
no SDK: a dependency's host libraries join a build only when its platform code
is reached. `demo/ddb_live` in the module exercises the real `HostDdb` against a
local stand-in.
