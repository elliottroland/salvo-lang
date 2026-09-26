package salvo.main

import salvo.core.console.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.seq.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

data class Fighter(
    var name: String,
    var hp: Int,
    var energy: Int,
)

fun named(roster: List<Fighter>, name: String): Fighter? {
    for (f in roster) {
        if (f.name == name) {
            return f
        }
    }
    return null
}

data class Window(
    var roster: List<Fighter>,
    var at: Int,
)

fun window(roster: List<Fighter>): Window {
    return Window(roster = roster, at = 0)
}

fun peek(w: Window): Fighter? {
    return w.roster.getOrNull(w.at)
}

fun heal(f: Fighter) {
    f.hp = f.hp + 10
    return
}

fun wounded(squad: List<Fighter>): Fighter? {
    for (f in squad) {
        if (f.hp < 10) {
            return f
        }
    }
    return null
}

fun<L> rally_at(squad: List<Fighter>, l: L, at: (List<Fighter>, L) -> Fighter?) {
    heal((at(squad, l) ?: throw AssertionError("salvo: value is absent at main:100:10")))
    return
}

fun duel(a: Fighter, d: Fighter) {
    a.hp = a.hp - 1
    d.hp = d.hp - 2
    return
}

fun strike(a: Fighter, d: Fighter) {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
    return
}

data class Squad(
    var banner: String,
    var members: List<Fighter>,
)

fun rotate(squad: Squad, from: Fighter, to: Fighter) {
    from.energy = from.energy - 1
    to.energy = to.energy + 1
    return
}

data class Camp(
    var supplies: Int,
    var banners: MutableList<String>,
)

fun spend(camp: Camp, n: Int) {
    camp.supplies = camp.supplies - n
    return
}

fun hoist(camp: Camp, banner: String) {
    camp.banners.add(banner)
    return
}

fun main() {
    val console: Console = StdOutConsole()
    val roster: List<Fighter> = listOf<Fighter>(Fighter(name = "Ada", hp = 30, energy = 4), Fighter(name = "Bo", hp = 8, energy = 9))
    val ada = (named(roster, "Ada") ?: throw AssertionError("salvo: value is absent at main:192:15"))
    println(console, "1. found ${ada.name}, hp ${ada.hp}")
    val names: MutableList<String> = mutableListOf<String>()
    names.add(ada.name)
    println(console, "1. copied out ${names.joinToString(", ", "[", "]")}")
    val w = window(roster)
    w.at = 1
    println(console, "1. window at ${w.at}: ${(peek(w) ?: throw AssertionError("salvo: value is absent at main:206:38")).name}")
    val pass = iter__3(roster)
    val standing = filter(pass, { f: Fighter ->
    f.hp > 10
}, ::next__3)
    println(console, "1. ${standing.size} of ${roster.size} still standing")
    val bench: MutableList<Fighter> = mutableListOf<Fighter>(Fighter(name = "Cy", hp = 12, energy = 2))
    bench.add(Fighter(name = "Dee", hp = 6, energy = 7))
    println(console, "2. bench ${bench.size}, front ${(bench.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:221:47")).name} (a reading, not a handle)")
    val squad: List<Fighter> = listOf<Fighter>(Fighter(name = "Ada", hp = 30, energy = 4), Fighter(name = "Bo", hp = 8, energy = 9))
    val boss = (squad.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:232:16"))
    boss.hp = boss.hp + 1
    val n = squad.size
    boss.hp = boss.hp + n
    println(console, "2. ${(squad.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:236:19")).name} at ${(squad.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:236:45")).hp} after a read in the middle")
    heal((wounded(squad) ?: throw AssertionError("salvo: value is absent at main:239:10")))
    rally_at(squad, 1, ::at)
    println(console, "2. after the searches: ${(squad.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:244:39")).hp} ${(squad.getOrNull(1) ?: throw AssertionError("salvo: value is absent at main:244:60")).hp}")
    val i = 0
    val j = 1
    if (NotEq_qualifies(j, i)) {
        val a = (squad.getOrNull(i) ?: throw AssertionError("salvo: value is absent at main:260:17"))
        val d = (squad.getOrNull(j) ?: throw AssertionError("salvo: value is absent at main:261:17"))
        a.hp = a.hp + 1
        d.hp = d.hp + 1
        duel(a, d)
    }
    if (Idx_qualifies(i, squad)) {
        if (Idx_qualifies(j, squad)) {
            update(squad, i, { f: Fighter ->
    f.energy = f.energy + 1
})
            if (NotEq_qualifies(j, i)) {
                update2(squad, i, j, { a: Fighter, b: Fighter ->
    a.energy = a.energy + 100
    b.energy = b.energy + 200
})
            }
            println(console, "3. ${get(squad, i).energy} ${get(squad, j).energy} (total reads: `Idx` survived)")
        }
    }
    if (Idx_qualifies(i, squad)) {
        if (Idx_qualifies(j, squad)) {
            strike(get(squad, i), get(squad, j))
            strike(get(squad, i), get(squad, i))
        }
    }
    println(console, "4. ${(squad.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:294:19")).hp} hp / ${(squad.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:294:45")).energy} energy after striking itself")
    val team = Squad(banner = "Red", members = listOf<Fighter>(Fighter(name = "Cy", hp = 12, energy = 2), Fighter(name = "Dee", hp = 6, energy = 7)))
    rotate(team, (team.members.getOrNull(i) ?: throw AssertionError("salvo: value is absent at main:301:18")), (team.members.getOrNull(j) ?: throw AssertionError("salvo: value is absent at main:301:41")))
    println(console, "4. ${team.banner}: ${(team.members.getOrNull(0) ?: throw AssertionError("salvo: value is absent at main:302:35")).energy} ${(team.members.getOrNull(1) ?: throw AssertionError("salvo: value is absent at main:302:67")).energy}")
    val camp = Camp(supplies = 10, banners = mutableListOf<String>("red"))
    val banners = camp.banners
    spend(camp, 3)
    hoist(camp, "blue")
    println(console, "5. supplies ${camp.supplies}, banners ${banners.joinToString(", ", "[", "]")}")
}
