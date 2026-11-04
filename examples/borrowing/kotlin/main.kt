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
    for (f in salvo.platform.std.core.list.each(roster)) {
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
    return salvo.std.core.list.getPlatform(w.roster, w.at)
}

fun heal(f: Fighter) {
    f.hp = (f.hp + 10)
    null
    return
}

fun wounded(squad: List<Fighter>): Fighter? {
    for (f in salvo.platform.std.core.list.each(squad)) {
        if ((f.hp < 10)) {
            return f
        }
    }
    return null
}

fun<L> rallyAt(squad: List<Fighter>, l: L, at: (List<Fighter>, L) -> Fighter?) {
    salvo.main.heal(kotlin.run {
        val __pick1: Fighter? = at(squad, l)
        when {
            (__pick1 != null) -> {
                val __pick1_1: Fighter = __pick1!!
                __pick1_1
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:102:26")
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

fun strike(c: List<Fighter>, a: Fighter, d: Fighter) {
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

fun rotate(members: List<Fighter>, from: Fighter, to: Fighter) {
    from.energy = (from.energy - 1)
    to.energy = (to.energy + 1)
    null
    return
}

data class Camp(
    var supplies: Int,
    var banners: salvo.platform.std.core.list.MutList<String>,
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
    salvo.std.core.list.addPlatform((camp.banners as salvo.platform.std.core.list.MutList<String>), banner)
    null
    return
}

fun main() {
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val roster: List<Fighter> = salvo.std.core.list.List__T_TArray(Fighter(name = "Ada", hp = 30, energy = 4), arrayOf<Fighter>(Fighter(name = "Bo", hp = 8, energy = 9)))
    val ada: Fighter = kotlin.run {
        val __pick2: Fighter? = salvo.main.named(roster, "Ada")
        when {
            (__pick2 != null) -> {
                val __pick2_3: Fighter = __pick2!!
                __pick2_3
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:191:39")
            }
        }
    }
    salvo.std.core.console.println(__handle_2, "1. found ${ada.name}, hp ${ada.hp}")
    val names: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    salvo.std.core.list.addPlatform(names, ada.name)
    salvo.std.core.console.println(__handle_2, "1. copied out ${salvo.std.core.list.toStr(names, { __a0 -> __a0 })}")
    var w: Window = salvo.main.window(roster)
    w.at = 1
    salvo.std.core.console.println(__handle_2, "1. window at ${w.at}: ${kotlin.run {
        val __proj_5: Fighter = kotlin.run {
            val __pick3: Fighter? = salvo.main.peek(w)
            when {
                (__pick3 != null) -> {
                    val __pick3_4: Fighter = __pick3!!
                    __pick3_4
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:205:50")
                }
            }
        }
        __proj_5.name
    }}")
    val pass: salvo.std.core.list.ListIter<Fighter> = salvo.std.core.list.iter(roster)
    val standing: salvo.platform.std.core.list.MutList<Fighter> = salvo.std.core.seq.filter(pass, fun(f: Fighter): Boolean {
        return (f.hp > 10)
    }, { __a0 -> salvo.std.core.list.next__ListIter(__a0) })
    salvo.std.core.console.println(__handle_2, "1. ${salvo.std.core.list.sizePlatform(standing)} of ${salvo.std.core.list.sizePlatform(roster)} still standing")
    val bench: salvo.platform.std.core.list.MutList<Fighter> = salvo.std.core.list.Mut_List(Fighter(name = "Cy", hp = 12, energy = 2), arrayOf<Fighter>())
    salvo.std.core.list.addPlatform(bench, Fighter(name = "Dee", hp = 6, energy = 7))
    salvo.std.core.console.println(__handle_2, "2. bench ${salvo.std.core.list.sizePlatform(bench)}, front ${kotlin.run {
        val __proj_7: Fighter = kotlin.run {
            val __pick4: Fighter? = salvo.std.core.list.getPlatform(bench, 0)
            when {
                (__pick4 != null) -> {
                    val __pick4_6: Fighter = __pick4!!
                    __pick4_6
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:220:65")
                }
            }
        }
        __proj_7.name
    }} (a reading, not a handle)")
    val squad: List<Fighter> = salvo.std.core.list.List__T_TArray(Fighter(name = "Ada", hp = 30, energy = 4), arrayOf<Fighter>(Fighter(name = "Bo", hp = 8, energy = 9)))
    var boss: Fighter = kotlin.run {
        val __pick5: Fighter? = salvo.std.core.list.atPlatform(squad, 0)
        when {
            (__pick5 != null) -> {
                val __pick5_8: Fighter = __pick5!!
                __pick5_8
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:231:32")
            }
        }
    }
    boss.hp = (boss.hp + 1)
    val n: Int = salvo.std.core.list.sizePlatform(squad)
    boss.hp = (boss.hp + n)
    salvo.std.core.console.println(__handle_2, "2. ${kotlin.run {
        val __proj_10: Fighter = kotlin.run {
            val __pick6: Fighter? = salvo.std.core.list.getPlatform(squad, 0)
            when {
                (__pick6 != null) -> {
                    val __pick6_9: Fighter = __pick6!!
                    __pick6_9
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:235:37")
                }
            }
        }
        __proj_10.name
    }} at ${kotlin.run {
        val __proj_12: Fighter = kotlin.run {
            val __pick7: Fighter? = salvo.std.core.list.getPlatform(squad, 0)
            when {
                (__pick7 != null) -> {
                    val __pick7_11: Fighter = __pick7!!
                    __pick7_11
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:235:82")
                }
            }
        }
        __proj_12.hp
    }} after a read in the middle")
    salvo.main.heal(kotlin.run {
        val __pick8: Fighter? = salvo.main.wounded(squad)
        when {
            (__pick8 != null) -> {
                val __pick8_13: Fighter = __pick8!!
                __pick8_13
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:238:28")
            }
        }
    })
    salvo.main.rallyAt(squad, 1, { __a0, __a1 -> salvo.std.core.list.atPlatform(__a0, __a1) })
    salvo.std.core.console.println(__handle_2, "2. after the searches: ${kotlin.run {
        val __proj_15: Fighter = kotlin.run {
            val __pick9: Fighter? = salvo.std.core.list.getPlatform(squad, 0)
            when {
                (__pick9 != null) -> {
                    val __pick9_14: Fighter = __pick9!!
                    __pick9_14
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:243:57")
                }
            }
        }
        __proj_15.hp
    }} ${kotlin.run {
        val __proj_17: Fighter = kotlin.run {
            val __pick10: Fighter? = salvo.std.core.list.getPlatform(squad, 1)
            when {
                (__pick10 != null) -> {
                    val __pick10_16: Fighter = __pick10!!
                    __pick10_16
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:243:97")
                }
            }
        }
        __proj_17.hp
    }}")
    val i: Int = 0
    val j: Int = 1
    var a: Fighter = kotlin.run {
        val __pick11: Fighter? = salvo.std.core.list.atPlatform(squad, i)
        when {
            (__pick11 != null) -> {
                val __pick11_18: Fighter = __pick11!!
                __pick11_18
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:259:29")
            }
        }
    }
    var d: Fighter = kotlin.run {
        val __pick12: Fighter? = salvo.std.core.list.atPlatform(squad, j)
        when {
            (__pick12 != null) -> {
                val __pick12_19: Fighter = __pick12!!
                __pick12_19
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:260:29")
            }
        }
    }
    if (salvo.std.core.ref.NotSame_qualifies(d, a)) {
        a.hp = (a.hp + 1)
        d.hp = (d.hp + 1)
        salvo.main.duel(a, d)
        a.energy = (a.energy + 100)
        d.energy = (d.energy + 200)
    }
    if (salvo.std.core.index.Idx_qualifies(i, squad, { __a0 -> salvo.std.core.list.sizePlatform(__a0) })) {
        if (salvo.std.core.index.Idx_qualifies(j, squad, { __a0 -> salvo.std.core.list.sizePlatform(__a0) })) {
            salvo.std.core.list.update(squad, i, fun(f: Fighter) {
                f.energy = (f.energy + 1)
            })
            salvo.std.core.console.println(__handle_2, "3. ${kotlin.run {
                val __proj_20: Fighter = salvo.std.core.list.get(squad, i)
                __proj_20.energy
            }} ${kotlin.run {
                val __proj_21: Fighter = salvo.std.core.list.get(squad, j)
                __proj_21.energy
            }} (total reads: `Idx` survived)")
        }
    }
    salvo.main.strike(squad, kotlin.run {
        val __pick13: Fighter? = salvo.std.core.list.atPlatform(squad, i)
        when {
            (__pick13 != null) -> {
                val __pick13_22: Fighter = __pick13!!
                __pick13_22
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:282:35")
            }
        }
    }, kotlin.run {
        val __pick14: Fighter? = salvo.std.core.list.atPlatform(squad, j)
        when {
            (__pick14 != null) -> {
                val __pick14_23: Fighter = __pick14!!
                __pick14_23
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:282:67")
            }
        }
    })
    salvo.main.strike(squad, kotlin.run {
        val __pick15: Fighter? = salvo.std.core.list.atPlatform(squad, i)
        when {
            (__pick15 != null) -> {
                val __pick15_24: Fighter = __pick15!!
                __pick15_24
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:285:35")
            }
        }
    }, kotlin.run {
        val __pick16: Fighter? = salvo.std.core.list.atPlatform(squad, i)
        when {
            (__pick16 != null) -> {
                val __pick16_25: Fighter = __pick16!!
                __pick16_25
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:285:67")
            }
        }
    })
    salvo.std.core.console.println(__handle_2, "4. ${kotlin.run {
        val __proj_27: Fighter = kotlin.run {
            val __pick17: Fighter? = salvo.std.core.list.getPlatform(squad, 0)
            when {
                (__pick17 != null) -> {
                    val __pick17_26: Fighter = __pick17!!
                    __pick17_26
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:286:37")
                }
            }
        }
        __proj_27.hp
    }} hp / ${kotlin.run {
        val __proj_29: Fighter = kotlin.run {
            val __pick18: Fighter? = salvo.std.core.list.getPlatform(squad, 0)
            when {
                (__pick18 != null) -> {
                    val __pick18_28: Fighter = __pick18!!
                    __pick18_28
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:286:82")
                }
            }
        }
        __proj_29.energy
    }} energy after striking itself")
    val team: Squad = Squad(banner = "Red", members = salvo.std.core.list.List__T_TArray(Fighter(name = "Cy", hp = 12, energy = 2), arrayOf<Fighter>(Fighter(name = "Dee", hp = 6, energy = 7))))
    salvo.main.rotate(team.members, kotlin.run {
        val __pick19: Fighter? = salvo.std.core.list.atPlatform(team.members, i)
        when {
            (__pick19 != null) -> {
                val __pick19_30: Fighter = __pick19!!
                __pick19_30
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:293:49")
            }
        }
    }, kotlin.run {
        val __pick20: Fighter? = salvo.std.core.list.atPlatform(team.members, j)
        when {
            (__pick20 != null) -> {
                val __pick20_31: Fighter = __pick20!!
                __pick20_31
            }
            else -> {
                throw AssertionError("salvo: unreachable at main:293:88")
            }
        }
    })
    salvo.std.core.console.println(__handle_2, "4. ${team.banner}: ${kotlin.run {
        val __proj_33: Fighter = kotlin.run {
            val __pick21: Fighter? = salvo.std.core.list.getPlatform(team.members, 0)
            when {
                (__pick21 != null) -> {
                    val __pick21_32: Fighter = __pick21!!
                    __pick21_32
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:294:60")
                }
            }
        }
        __proj_33.energy
    }} ${kotlin.run {
        val __proj_35: Fighter = kotlin.run {
            val __pick22: Fighter? = salvo.std.core.list.getPlatform(team.members, 1)
            when {
                (__pick22 != null) -> {
                    val __pick22_34: Fighter = __pick22!!
                    __pick22_34
                }
                else -> {
                    throw AssertionError("salvo: unreachable at main:294:111")
                }
            }
        }
        __proj_35.energy
    }}")
    val camp: Camp = Camp(supplies = 10, banners = salvo.std.core.list.Mut_List("red", arrayOf<String>()))
    val banners: salvo.platform.std.core.list.MutList<String> = camp.banners
    salvo.main.spend(camp, 3)
    salvo.main.hoist(camp, "blue")
    salvo.std.core.console.println(__handle_2, "5. supplies ${camp.supplies}, banners ${salvo.std.core.list.toStr(banners, { __a0 -> __a0 })}")
}

