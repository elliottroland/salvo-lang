// [actor-router] A router is a handler of the protocol it routes; one
// generic over its protocol (`for E`) answers it with `sent`.
import std.net
import std.net.shard

actor effect Inventory {
    send fn reserve(ShardKey sku: Str, qty: Int, out: Reply<Str>) => !sku, !out
    send fn restock(qty: Int)
}

fn key(m: Message<Inventory>) [] -> Long? => m by shard

handler Stocking() for Inventory {
    mailbox { capacity: 8 }
    send fn reserve(sku: Str, qty: Int, out: Reply<Str>) => !sku, !out {
        send(out, "${sku}: ${qty}")
    }
    send fn restock(qty: Int) {}
}

// A router of one's own: the first member on this node.
fn local_first<E>(view: RouteView<E>) [] -> eff[E]? => view {
    for x in view.members {
        if x.local { return copy(x.addr) }
    }
    for x in view.members { return copy(x.addr) }
    return None
}

handler LocalFirst<E>(group: eff[ActorGroup<E>], config: RouteConfig = default_route_config()) for E {
    mailbox { capacity: 16 }
    send fn sent(m: Message<E>) => !m {
        let wait = copy(config.first_wait)
        while true {
            let view = route_view(group)
            let pick = local_first(view)
            if !(pick is None) {
                send(pick, m)
                return
            }
            wait = route_wait(group, view, wait, config)
        }
    }
}

// Serving a type parameter without `sent`: no member of it can be named.
//~ error[actor-sent:generic-face]
handler Blind<E>(back: eff[E]) for E {
}

fn shop(stock: eff[ActorGroup<Inventory>]) [use, Console] -> None => stock {
    use Sharded(copy(stock))
    println(waitfor out: Reply<Str> { reserve("apple", 1, out) })
    use LocalFirst(copy(stock))
    restock(2)
    use LeastLoaded(copy(stock), true)
    restock(3)
}

//--- two_shard_keys
// [actor-router] A member marks at most one `ShardKey`: the `key`
// template's comptime refusal.
import std.net
import std.net.shard

actor effect Pairs {
    send fn put(ShardKey a: Str, ShardKey b: Str) => !a, !b
}

//~ error[comptime-refuse:refused]
fn key(m: Message<Pairs>) [] -> Long? => m by shard

//--- sharded_needs_a_key
// [actor-router] `Sharded` reads the protocol's `key` fn; with none in
// scope the `use` cannot find it.
import std.net

actor effect Bare {
    send fn go(n: Int)
}

fn run(g: eff[ActorGroup<Bare>]) [use] -> None => g {
    //~ error[implicit-resolve:none]
    use Sharded(copy(g))
    go(1)
}
