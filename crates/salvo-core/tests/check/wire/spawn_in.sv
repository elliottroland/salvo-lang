// [actor-group] The `in GROUP` clause of a spawn: the group must be of one
// of the spawn's faces, and must be a group handle at all.
import std.net

actor effect Ping { send fn ping() }
actor effect Pong { send fn pong() }
handler Pinging() of Ping { mailbox { capacity: 1 } send fn ping() {} }
handler Both() of Ping, Pong { mailbox { capacity: 1 } send fn ping() {} send fn pong() {} }

fn main(pings: eff[ActorGroup<Ping>], pongs: eff[ActorGroup<Pong>], n: Int) [spawn] -> None
    => !pings, !pongs {
    let _ok = spawn Pinging() on pool(1) in pings
    let (_p, _q) = spawn Both() on pool(1) in pongs
    //~ error[actor-group:no-face]
    let _wrong = spawn Pinging() on pool(1) in pongs
    //~ error[actor-group:not-a-group]
    let _not = spawn Pinging() on pool(1) in n
}
