// The keys of sharded routing: which argument of a message decides the
// member of a group that takes it, for [Sharded](std.net).
//
// Mark the deciding parameter of a `send fn` with [ShardKey], and stamp the
// protocol's `key` fn from this module: it answers the hash of the marked
// argument (with `key_hash`), or `None` for a message whose member marks
// none, and [Sharded](std.net) reduces the hash to a member. A member may
// mark at most one parameter; stamping refuses a protocol that marks two.
//
// ```
// import std.net.shard
//
// actor effect Inventory {
//     send fn reserve(ShardKey sku: Str, qty: Int, out: Reply<Str>) => !sku, !out
//     send fn restock(qty: Int)
// }
//
// fn key(m: message[Inventory]) [] -> Long? => m by shard
//
// fn same_shard() [] -> Bool {
//     let a = key(Inventory.Restock { qty: 1 })     // None: no key
//     return a is None
// }
// ```
// @compilerrules [actor-router, comptime-eval]

import std.net

// Marks the parameter of a `send fn` whose value decides which member of a
// group takes the message, for [Sharded](std.net): written before the
// parameter's name, and erased everywhere but the stamped `key`.
// @compilerrules [actor-router, comptime-qualifier]
export comptime qualifier ShardKey for Field

// The key of message [m]: the hash of its member's [ShardKey] argument, or
// `None` when the member marks none. Stamped per protocol, as `fn key(m:
// message[P]) [] -> Long? => m by shard`.
// @compilerrules [actor-router, comptime-eval]
export comptime fn key<T is Union>(m: T) [] -> Long? => m {
    [when m] {
        [arm] {
            [let keys = filter(arm.type.fields, (f) -> f is ShardKey)]
            [if size(keys) > 1] {
                refuse!("`${arm.name}` marks ${size(keys)} parameters as `ShardKey`; a send marks at most one")
            }
            [if size(keys) == 0] {
                return None
            }
            [for f in keys] ...
            return key_hash@net(m.[f])
        }
    }
}
