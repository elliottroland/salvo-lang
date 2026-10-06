package salvo.main

import salvo.core.console.Console
import salvo.core.console.println
import salvo.core.index.Idx_qualifies
import salvo.core.index.NotEq_qualifies
import salvo.core.list.addPlatform
import salvo.core.list.at
import salvo.core.list.get
import salvo.core.list.getPlatform
import salvo.core.list.iter
import salvo.core.list.next__ListYield
import salvo.core.list.sizePlatform
import salvo.core.list.toStr
import salvo.core.list.update
import salvo.core.list.update2
import salvo.core.seq.filter

data class Fighter(
    var name: String,
    var hp: Int,
    var energy: Int,
)

object __Codec_Fighter : salvo.WireCodec<Fighter> {
    override fun enc(v: Fighter, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.name, out)
        salvo.IntCodec.enc(v.hp, out)
        salvo.IntCodec.enc(v.energy, out)
    }
    override fun dec(inp: salvo.WireIn): Fighter = Fighter(salvo.StrCodec.dec(inp), salvo.IntCodec.dec(inp), salvo.IntCodec.dec(inp))
}

fun named(roster: List<Fighter>, name: String): Fighter? {
    for (f in salvo.platform.core.list.each(roster)) {
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
    return getPlatform(w.roster, w.at)
}

fun heal(f: Fighter) {
    f.hp = f.hp + 10
    return
}

fun wounded(squad: List<Fighter>): Fighter? {
    for (f in salvo.platform.core.list.each(squad)) {
        if (f.hp < 10) {
            return f
        }
    }
    return null
}

fun<L> rallyAt(squad: List<Fighter>, l: L, at: (List<Fighter>, L) -> Fighter?) {
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

object __Codec_Squad : salvo.WireCodec<Squad> {
    override fun enc(v: Squad, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.banner, out)
        salvo.ListCodec(__Codec_Fighter).enc(v.members, out)
    }
    override fun dec(inp: salvo.WireIn): Squad = Squad(salvo.StrCodec.dec(inp), salvo.ListCodec(__Codec_Fighter).dec(inp))
}

fun rotate(squad: Squad, from: Fighter, to: Fighter) {
    from.energy = from.energy - 1
    to.energy = to.energy + 1
    return
}

data class Camp(
    var supplies: Int,
    var banners: salvo.platform.core.list.MutList<String>,
)

object __Codec_Camp : salvo.WireCodec<Camp> {
    override fun enc(v: Camp, out: salvo.WireOut) {
        salvo.IntCodec.enc(v.supplies, out)
        salvo.MutListCodec(salvo.StrCodec).enc(v.banners, out)
    }
    override fun dec(inp: salvo.WireIn): Camp = Camp(salvo.IntCodec.dec(inp), salvo.MutListCodec(salvo.StrCodec).dec(inp))
}

fun spend(camp: Camp, n: Int) {
    camp.supplies = camp.supplies - n
    return
}

fun hoist(camp: Camp, banner: String) {
    addPlatform(camp.banners, banner)
    return
}

fun main() {
    val console: Console = salvo.core.console.__Platform_StdOutConsole()
    val roster: List<Fighter> = listOf<Fighter>(Fighter(name = "Ada", hp = 30, energy = 4), Fighter(name = "Bo", hp = 8, energy = 9))
    val ada = (named(roster, "Ada") ?: throw AssertionError("salvo: value is absent at main:192:15"))
    println(console, "1. found ${ada.name}, hp ${ada.hp}")
    val names: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    addPlatform(names, ada.name)
    println(console, "1. copied out ${toStr(names, { __i0 -> __i0 })}")
    val w = window(roster)
    w.at = 1
    println(console, "1. window at ${w.at}: ${(peek(w) ?: throw AssertionError("salvo: value is absent at main:206:38")).name}")
    val pass = iter(roster)
    val standing = filter(pass, { f: Fighter ->
    f.hp > 10
}, ::next__ListYield)
    println(console, "1. ${sizePlatform(standing)} of ${sizePlatform(roster)} still standing")
    val bench: salvo.platform.core.list.MutList<Fighter> = mutableListOf<Fighter>(Fighter(name = "Cy", hp = 12, energy = 2))
    addPlatform(bench, Fighter(name = "Dee", hp = 6, energy = 7))
    println(console, "2. bench ${sizePlatform(bench)}, front ${(getPlatform(bench, 0) ?: throw AssertionError("salvo: value is absent at main:221:47")).name} (a reading, not a handle)")
    val squad: List<Fighter> = listOf<Fighter>(Fighter(name = "Ada", hp = 30, energy = 4), Fighter(name = "Bo", hp = 8, energy = 9))
    val boss = (getPlatform(squad, 0) ?: throw AssertionError("salvo: value is absent at main:232:16"))
    boss.hp = boss.hp + 1
    val n = sizePlatform(squad)
    boss.hp = boss.hp + n
    println(console, "2. ${(getPlatform(squad, 0) ?: throw AssertionError("salvo: value is absent at main:236:19")).name} at ${(getPlatform(squad, 0) ?: throw AssertionError("salvo: value is absent at main:236:45")).hp} after a read in the middle")
    heal((wounded(squad) ?: throw AssertionError("salvo: value is absent at main:239:10")))
    rallyAt(squad, 1, ::at)
    println(console, "2. after the searches: ${(getPlatform(squad, 0) ?: throw AssertionError("salvo: value is absent at main:244:39")).hp} ${(getPlatform(squad, 1) ?: throw AssertionError("salvo: value is absent at main:244:60")).hp}")
    val i = 0
    val j = 1
    if (NotEq_qualifies(j, i)) {
        val a = (getPlatform(squad, i) ?: throw AssertionError("salvo: value is absent at main:260:17"))
        val d = (getPlatform(squad, j) ?: throw AssertionError("salvo: value is absent at main:261:17"))
        a.hp = a.hp + 1
        d.hp = d.hp + 1
        duel(a, d)
    }
    if (Idx_qualifies(i, squad, ::sizePlatform)) {
        if (Idx_qualifies(j, squad, ::sizePlatform)) {
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
    if (Idx_qualifies(i, squad, ::sizePlatform)) {
        if (Idx_qualifies(j, squad, ::sizePlatform)) {
            strike(get(squad, i), get(squad, j))
            strike(get(squad, i), get(squad, i))
        }
    }
    println(console, "4. ${(getPlatform(squad, 0) ?: throw AssertionError("salvo: value is absent at main:294:19")).hp} hp / ${(getPlatform(squad, 0) ?: throw AssertionError("salvo: value is absent at main:294:45")).energy} energy after striking itself")
    val team = Squad(banner = "Red", members = listOf<Fighter>(Fighter(name = "Cy", hp = 12, energy = 2), Fighter(name = "Dee", hp = 6, energy = 7)))
    rotate(team, (getPlatform(team.members, i) ?: throw AssertionError("salvo: value is absent at main:301:18")), (getPlatform(team.members, j) ?: throw AssertionError("salvo: value is absent at main:301:41")))
    println(console, "4. ${team.banner}: ${(getPlatform(team.members, 0) ?: throw AssertionError("salvo: value is absent at main:302:35")).energy} ${(getPlatform(team.members, 1) ?: throw AssertionError("salvo: value is absent at main:302:67")).energy}")
    val camp = Camp(supplies = 10, banners = mutableListOf<String>("red"))
    val banners = camp.banners
    spend(camp, 3)
    hoist(camp, "blue")
    println(console, "5. supplies ${camp.supplies}, banners ${toStr(banners, { __i0 -> __i0 })}")
}
