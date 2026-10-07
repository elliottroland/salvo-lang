package salvo.main

import salvo.*

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
        if (((f.name) == (name))) {
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
    return salvo.core.list.getPlatform(w.roster, w.at)
}

fun heal(f: Fighter) {
    f.hp = (f.hp + 10)
    null
    return
}

fun wounded(squad: List<Fighter>): Fighter? {
    for (f in salvo.platform.core.list.each(squad)) {
        if ((f.hp < 10)) {
            return f
        }
    }
    return null
}

fun<L> rallyAt(squad: List<Fighter>, l: L, at: (List<Fighter>, L) -> Fighter?) {
    heal(run {
        val __nn_1: Fighter? = at(squad, l)
        when {
            (__nn_1 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:100:10"))
            }
            else -> {
                val __some_2: Fighter = __nn_1!!
                __some_2
            }
        }
    })
    null
    return
}

fun duel(a: Fighter, d: Fighter) {
    a.hp = (a.hp - 1)
    d.hp = (d.hp - 2)
    null
    return
}

fun strike(a: Fighter, d: Fighter) {
    a.energy = (a.energy - 1)
    d.hp = (d.hp - 2)
    null
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
    from.energy = (from.energy - 1)
    to.energy = (to.energy + 1)
    null
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
    camp.supplies = (camp.supplies - n)
    null
    return
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun hoist(camp: Camp, banner: String) {
    salvo.core.list.addPlatform((camp.banners as salvo.platform.core.list.MutList<String>), banner)
    null
    return
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun main() {
    val __use_1: salvo.core.console.__Platform_StdOutConsole = salvo.core.console.__Platform_StdOutConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val roster: List<Fighter> = listOf<Fighter>(Fighter(name = "Ada", hp = 30, energy = 4), Fighter(name = "Bo", hp = 8, energy = 9))
    val ada: Fighter = run {
        val __nn_3: Fighter? = named(roster, "Ada")
        when {
            (__nn_3 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:192:15"))
            }
            else -> {
                val __some_4: Fighter = __nn_3!!
                __some_4
            }
        }
    }
    salvo.core.console.println(__handle_2, "1. found ${ada.name}, hp ${ada.hp}")
    val names: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    salvo.core.list.addPlatform(names, ada.name)
    salvo.core.console.println(__handle_2, "1. copied out ${salvo.core.list.toStr(names, { __a0 -> __a0 })}")
    var w: Window = window(roster)
    w.at = 1
    salvo.core.console.println(__handle_2, "1. window at ${w.at}: ${run {
        val __proj_7: Fighter = run {
            val __nn_5: Fighter? = peek(w)
            when {
                (__nn_5 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:206:38"))
                }
                else -> {
                    val __some_6: Fighter = __nn_5!!
                    __some_6
                }
            }
        }
        __proj_7.name
    }}")
    val pass: salvo.core.list.ListYield<Fighter> = salvo.core.list.iter(roster)
    val standing: salvo.platform.core.list.MutList<Fighter> = salvo.core.seq.filter(pass, fun(f: Fighter): Boolean {
        return (f.hp > 10)
    }, { __a0 -> salvo.core.list.next__ListYield(__a0) })
    salvo.core.console.println(__handle_2, "1. ${salvo.core.list.sizePlatform(standing)} of ${salvo.core.list.sizePlatform(roster)} still standing")
    val bench: salvo.platform.core.list.MutList<Fighter> = mutableListOf<Fighter>(Fighter(name = "Cy", hp = 12, energy = 2))
    salvo.core.list.addPlatform(bench, Fighter(name = "Dee", hp = 6, energy = 7))
    salvo.core.console.println(__handle_2, "2. bench ${salvo.core.list.sizePlatform(bench)}, front ${run {
        val __proj_10: Fighter = run {
            val __nn_8: Fighter? = salvo.core.list.getPlatform(bench, 0)
            when {
                (__nn_8 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:221:47"))
                }
                else -> {
                    val __some_9: Fighter = __nn_8!!
                    __some_9
                }
            }
        }
        __proj_10.name
    }} (a reading, not a handle)")
    val squad: List<Fighter> = listOf<Fighter>(Fighter(name = "Ada", hp = 30, energy = 4), Fighter(name = "Bo", hp = 8, energy = 9))
    var boss: Fighter = run {
        val __nn_11: Fighter? = salvo.core.list.getPlatform(squad, 0)
        when {
            (__nn_11 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:232:16"))
            }
            else -> {
                val __some_12: Fighter = __nn_11!!
                __some_12
            }
        }
    }
    boss.hp = (boss.hp + 1)
    val n: Int = salvo.core.list.sizePlatform(squad)
    boss.hp = (boss.hp + n)
    salvo.core.console.println(__handle_2, "2. ${run {
        val __proj_15: Fighter = run {
            val __nn_13: Fighter? = salvo.core.list.getPlatform(squad, 0)
            when {
                (__nn_13 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:236:19"))
                }
                else -> {
                    val __some_14: Fighter = __nn_13!!
                    __some_14
                }
            }
        }
        __proj_15.name
    }} at ${run {
        val __proj_18: Fighter = run {
            val __nn_16: Fighter? = salvo.core.list.getPlatform(squad, 0)
            when {
                (__nn_16 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:236:45"))
                }
                else -> {
                    val __some_17: Fighter = __nn_16!!
                    __some_17
                }
            }
        }
        __proj_18.hp
    }} after a read in the middle")
    heal(run {
        val __nn_19: Fighter? = wounded(squad)
        when {
            (__nn_19 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:239:10"))
            }
            else -> {
                val __some_20: Fighter = __nn_19!!
                __some_20
            }
        }
    })
    rallyAt(squad, 1, { __a0, __a1 -> salvo.core.list.at(__a0, __a1) })
    salvo.core.console.println(__handle_2, "2. after the searches: ${run {
        val __proj_23: Fighter = run {
            val __nn_21: Fighter? = salvo.core.list.getPlatform(squad, 0)
            when {
                (__nn_21 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:244:39"))
                }
                else -> {
                    val __some_22: Fighter = __nn_21!!
                    __some_22
                }
            }
        }
        __proj_23.hp
    }} ${run {
        val __proj_26: Fighter = run {
            val __nn_24: Fighter? = salvo.core.list.getPlatform(squad, 1)
            when {
                (__nn_24 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:244:60"))
                }
                else -> {
                    val __some_25: Fighter = __nn_24!!
                    __some_25
                }
            }
        }
        __proj_26.hp
    }}")
    val i: Int = 0
    val j: Int = 1
    if (salvo.core.index.NotEq_qualifies(j, i)) {
        var a: Fighter = run {
            val __nn_27: Fighter? = salvo.core.list.getPlatform(squad, i)
            when {
                (__nn_27 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:260:17"))
                }
                else -> {
                    val __some_28: Fighter = __nn_27!!
                    __some_28
                }
            }
        }
        var d: Fighter = run {
            val __nn_29: Fighter? = salvo.core.list.getPlatform(squad, j)
            when {
                (__nn_29 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:261:17"))
                }
                else -> {
                    val __some_30: Fighter = __nn_29!!
                    __some_30
                }
            }
        }
        a.hp = (a.hp + 1)
        d.hp = (d.hp + 1)
        duel(a, d)
    }
    if (salvo.core.index.Idx_qualifies(i, squad, { __a0 -> salvo.core.list.sizePlatform(__a0) })) {
        if (salvo.core.index.Idx_qualifies(j, squad, { __a0 -> salvo.core.list.sizePlatform(__a0) })) {
            salvo.core.list.update(squad, i, fun(f: Fighter) {
                f.energy = (f.energy + 1)
            })
            if (salvo.core.index.NotEq_qualifies(j, i)) {
                salvo.core.list.update2(squad, i, j, fun(a: Fighter, b: Fighter) {
                    a.energy = (a.energy + 100)
                    b.energy = (b.energy + 200)
                })
            }
            salvo.core.console.println(__handle_2, "3. ${run {
                val __proj_31: Fighter = salvo.core.list.get(squad, i)
                __proj_31.energy
            }} ${run {
                val __proj_32: Fighter = salvo.core.list.get(squad, j)
                __proj_32.energy
            }} (total reads: `Idx` survived)")
        }
    }
    if (salvo.core.index.Idx_qualifies(i, squad, { __a0 -> salvo.core.list.sizePlatform(__a0) })) {
        if (salvo.core.index.Idx_qualifies(j, squad, { __a0 -> salvo.core.list.sizePlatform(__a0) })) {
            strike(salvo.core.list.get(squad, i), salvo.core.list.get(squad, j))
            strike(salvo.core.list.get(squad, i), salvo.core.list.get(squad, i))
        }
    }
    salvo.core.console.println(__handle_2, "4. ${run {
        val __proj_35: Fighter = run {
            val __nn_33: Fighter? = salvo.core.list.getPlatform(squad, 0)
            when {
                (__nn_33 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:294:19"))
                }
                else -> {
                    val __some_34: Fighter = __nn_33!!
                    __some_34
                }
            }
        }
        __proj_35.hp
    }} hp / ${run {
        val __proj_38: Fighter = run {
            val __nn_36: Fighter? = salvo.core.list.getPlatform(squad, 0)
            when {
                (__nn_36 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:294:45"))
                }
                else -> {
                    val __some_37: Fighter = __nn_36!!
                    __some_37
                }
            }
        }
        __proj_38.energy
    }} energy after striking itself")
    val team: Squad = Squad(banner = "Red", members = listOf<Fighter>(Fighter(name = "Cy", hp = 12, energy = 2), Fighter(name = "Dee", hp = 6, energy = 7)))
    rotate(team, run {
        val __nn_39: Fighter? = salvo.core.list.getPlatform(team.members, i)
        when {
            (__nn_39 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:301:18"))
            }
            else -> {
                val __some_40: Fighter = __nn_39!!
                __some_40
            }
        }
    }, run {
        val __nn_41: Fighter? = salvo.core.list.getPlatform(team.members, j)
        when {
            (__nn_41 == null) -> {
                throw AssertionError(("salvo: " + ("value is absent") + " at main:301:41"))
            }
            else -> {
                val __some_42: Fighter = __nn_41!!
                __some_42
            }
        }
    })
    salvo.core.console.println(__handle_2, "4. ${team.banner}: ${run {
        val __proj_45: Fighter = run {
            val __nn_43: Fighter? = salvo.core.list.getPlatform(team.members, 0)
            when {
                (__nn_43 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:302:35"))
                }
                else -> {
                    val __some_44: Fighter = __nn_43!!
                    __some_44
                }
            }
        }
        __proj_45.energy
    }} ${run {
        val __proj_48: Fighter = run {
            val __nn_46: Fighter? = salvo.core.list.getPlatform(team.members, 1)
            when {
                (__nn_46 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at main:302:67"))
                }
                else -> {
                    val __some_47: Fighter = __nn_46!!
                    __some_47
                }
            }
        }
        __proj_48.energy
    }}")
    val camp: Camp = Camp(supplies = 10, banners = mutableListOf<String>("red"))
    val banners: salvo.platform.core.list.MutList<String> = (camp.banners as salvo.platform.core.list.MutList<String>)
    spend(camp, 3)
    hoist(camp, "blue")
    salvo.core.console.println(__handle_2, "5. supplies ${camp.supplies}, banners ${salvo.core.list.toStr(banners, { __a0 -> __a0 })}")
}

