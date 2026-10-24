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
    heal(kotlin.run {
        val __pick_1: Fighter? = at(squad, l)
        (if ((__pick_1 != null)) {
            val __picked_2: Fighter = __pick_1!!
            __picked_2
        } else {
            throw AssertionError("salvo: unreachable at main:102:26")
        })
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

fun main() {
    val __use_1: salvo.core.console.__Platform_StdConsole = salvo.core.console.__Platform_StdConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val roster: List<Fighter> = listOf<Fighter>(Fighter(name = "Ada", hp = 30, energy = 4), Fighter(name = "Bo", hp = 8, energy = 9))
    val ada: Fighter = kotlin.run {
        val __pick_3: Fighter? = named(roster, "Ada")
        (if ((__pick_3 != null)) {
            val __picked_4: Fighter = __pick_3!!
            __picked_4
        } else {
            throw AssertionError("salvo: unreachable at main:191:39")
        })
    }
    salvo.core.console.println(__handle_2, "1. found ${ada.name}, hp ${ada.hp}")
    val names: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    salvo.core.list.addPlatform(names, ada.name)
    salvo.core.console.println(__handle_2, "1. copied out ${salvo.core.list.toStr(names, { __a0 -> __a0 })}")
    var w: Window = window(roster)
    w.at = 1
    salvo.core.console.println(__handle_2, "1. window at ${w.at}: ${kotlin.run {
        val __proj_9: Fighter = kotlin.run {
            val __pick_6: Fighter? = peek(w)
            (if ((__pick_6 != null)) {
                val __picked_7: Fighter = __pick_6!!
                __picked_7
            } else {
                throw AssertionError("salvo: unreachable at main:205:50")
            })
        }
        __proj_9.name
    }}")
    val pass: salvo.core.list.ListYield<Fighter> = salvo.core.list.iter(roster)
    val standing: salvo.platform.core.list.MutList<Fighter> = salvo.core.seq.filter(pass, fun(f: Fighter): Boolean {
        return (f.hp > 10)
    }, { __a0 -> salvo.core.list.next__ListYield(__a0) })
    salvo.core.console.println(__handle_2, "1. ${salvo.core.list.sizePlatform(standing)} of ${salvo.core.list.sizePlatform(roster)} still standing")
    val bench: salvo.platform.core.list.MutList<Fighter> = mutableListOf<Fighter>(Fighter(name = "Cy", hp = 12, energy = 2))
    salvo.core.list.addPlatform(bench, Fighter(name = "Dee", hp = 6, energy = 7))
    salvo.core.console.println(__handle_2, "2. bench ${salvo.core.list.sizePlatform(bench)}, front ${kotlin.run {
        val __proj_13: Fighter = kotlin.run {
            val __pick_10: Fighter? = salvo.core.list.getPlatform(bench, 0)
            (if ((__pick_10 != null)) {
                val __picked_11: Fighter = __pick_10!!
                __picked_11
            } else {
                throw AssertionError("salvo: unreachable at main:220:65")
            })
        }
        __proj_13.name
    }} (a reading, not a handle)")
    val squad: List<Fighter> = listOf<Fighter>(Fighter(name = "Ada", hp = 30, energy = 4), Fighter(name = "Bo", hp = 8, energy = 9))
    var boss: Fighter = kotlin.run {
        val __pick_14: Fighter? = salvo.core.list.atPlatform(squad, 0)
        (if ((__pick_14 != null)) {
            val __picked_15: Fighter = __pick_14!!
            __picked_15
        } else {
            throw AssertionError("salvo: unreachable at main:231:32")
        })
    }
    boss.hp = (boss.hp + 1)
    val n: Int = salvo.core.list.sizePlatform(squad)
    boss.hp = (boss.hp + n)
    salvo.core.console.println(__handle_2, "2. ${kotlin.run {
        val __proj_20: Fighter = kotlin.run {
            val __pick_17: Fighter? = salvo.core.list.getPlatform(squad, 0)
            (if ((__pick_17 != null)) {
                val __picked_18: Fighter = __pick_17!!
                __picked_18
            } else {
                throw AssertionError("salvo: unreachable at main:235:37")
            })
        }
        __proj_20.name
    }} at ${kotlin.run {
        val __proj_24: Fighter = kotlin.run {
            val __pick_21: Fighter? = salvo.core.list.getPlatform(squad, 0)
            (if ((__pick_21 != null)) {
                val __picked_22: Fighter = __pick_21!!
                __picked_22
            } else {
                throw AssertionError("salvo: unreachable at main:235:82")
            })
        }
        __proj_24.hp
    }} after a read in the middle")
    heal(kotlin.run {
        val __pick_25: Fighter? = wounded(squad)
        (if ((__pick_25 != null)) {
            val __picked_26: Fighter = __pick_25!!
            __picked_26
        } else {
            throw AssertionError("salvo: unreachable at main:238:28")
        })
    })
    rallyAt(squad, 1, { __a0, __a1 -> salvo.core.list.atPlatform(__a0, __a1) })
    salvo.core.console.println(__handle_2, "2. after the searches: ${kotlin.run {
        val __proj_31: Fighter = kotlin.run {
            val __pick_28: Fighter? = salvo.core.list.getPlatform(squad, 0)
            (if ((__pick_28 != null)) {
                val __picked_29: Fighter = __pick_28!!
                __picked_29
            } else {
                throw AssertionError("salvo: unreachable at main:243:57")
            })
        }
        __proj_31.hp
    }} ${kotlin.run {
        val __proj_35: Fighter = kotlin.run {
            val __pick_32: Fighter? = salvo.core.list.getPlatform(squad, 1)
            (if ((__pick_32 != null)) {
                val __picked_33: Fighter = __pick_32!!
                __picked_33
            } else {
                throw AssertionError("salvo: unreachable at main:243:97")
            })
        }
        __proj_35.hp
    }}")
    val i: Int = 0
    val j: Int = 1
    var a: Fighter = kotlin.run {
        val __pick_36: Fighter? = salvo.core.list.atPlatform(squad, i)
        (if ((__pick_36 != null)) {
            val __picked_37: Fighter = __pick_36!!
            __picked_37
        } else {
            throw AssertionError("salvo: unreachable at main:259:29")
        })
    }
    var d: Fighter = kotlin.run {
        val __pick_39: Fighter? = salvo.core.list.atPlatform(squad, j)
        (if ((__pick_39 != null)) {
            val __picked_40: Fighter = __pick_39!!
            __picked_40
        } else {
            throw AssertionError("salvo: unreachable at main:260:29")
        })
    }
    if (salvo.core.ref.NotSame_qualifies(d, a)) {
        a.hp = (a.hp + 1)
        d.hp = (d.hp + 1)
        duel(a, d)
        a.energy = (a.energy + 100)
        d.energy = (d.energy + 200)
    }
    if (salvo.core.index.Idx_qualifies(i, squad, { __a0 -> salvo.core.list.sizePlatform(__a0) })) {
        if (salvo.core.index.Idx_qualifies(j, squad, { __a0 -> salvo.core.list.sizePlatform(__a0) })) {
            salvo.core.list.update(squad, i, fun(f: Fighter) {
                f.energy = (f.energy + 1)
            })
            salvo.core.console.println(__handle_2, "3. ${kotlin.run {
                val __proj_42: Fighter = salvo.core.list.get(squad, i)
                __proj_42.energy
            }} ${kotlin.run {
                val __proj_43: Fighter = salvo.core.list.get(squad, j)
                __proj_43.energy
            }} (total reads: `Idx` survived)")
        }
    }
    strike(squad, kotlin.run {
        val __pick_44: Fighter? = salvo.core.list.atPlatform(squad, i)
        (if ((__pick_44 != null)) {
            val __picked_45: Fighter = __pick_44!!
            __picked_45
        } else {
            throw AssertionError("salvo: unreachable at main:282:35")
        })
    }, kotlin.run {
        val __pick_47: Fighter? = salvo.core.list.atPlatform(squad, j)
        (if ((__pick_47 != null)) {
            val __picked_48: Fighter = __pick_47!!
            __picked_48
        } else {
            throw AssertionError("salvo: unreachable at main:282:67")
        })
    })
    strike(squad, kotlin.run {
        val __pick_50: Fighter? = salvo.core.list.atPlatform(squad, i)
        (if ((__pick_50 != null)) {
            val __picked_51: Fighter = __pick_50!!
            __picked_51
        } else {
            throw AssertionError("salvo: unreachable at main:285:35")
        })
    }, kotlin.run {
        val __pick_53: Fighter? = salvo.core.list.atPlatform(squad, i)
        (if ((__pick_53 != null)) {
            val __picked_54: Fighter = __pick_53!!
            __picked_54
        } else {
            throw AssertionError("salvo: unreachable at main:285:67")
        })
    })
    salvo.core.console.println(__handle_2, "4. ${kotlin.run {
        val __proj_59: Fighter = kotlin.run {
            val __pick_56: Fighter? = salvo.core.list.getPlatform(squad, 0)
            (if ((__pick_56 != null)) {
                val __picked_57: Fighter = __pick_56!!
                __picked_57
            } else {
                throw AssertionError("salvo: unreachable at main:286:37")
            })
        }
        __proj_59.hp
    }} hp / ${kotlin.run {
        val __proj_63: Fighter = kotlin.run {
            val __pick_60: Fighter? = salvo.core.list.getPlatform(squad, 0)
            (if ((__pick_60 != null)) {
                val __picked_61: Fighter = __pick_60!!
                __picked_61
            } else {
                throw AssertionError("salvo: unreachable at main:286:82")
            })
        }
        __proj_63.energy
    }} energy after striking itself")
    val team: Squad = Squad(banner = "Red", members = listOf<Fighter>(Fighter(name = "Cy", hp = 12, energy = 2), Fighter(name = "Dee", hp = 6, energy = 7)))
    rotate(team.members, kotlin.run {
        val __pick_64: Fighter? = salvo.core.list.atPlatform(team.members, i)
        (if ((__pick_64 != null)) {
            val __picked_65: Fighter = __pick_64!!
            __picked_65
        } else {
            throw AssertionError("salvo: unreachable at main:293:49")
        })
    }, kotlin.run {
        val __pick_67: Fighter? = salvo.core.list.atPlatform(team.members, j)
        (if ((__pick_67 != null)) {
            val __picked_68: Fighter = __pick_67!!
            __picked_68
        } else {
            throw AssertionError("salvo: unreachable at main:293:88")
        })
    })
    salvo.core.console.println(__handle_2, "4. ${team.banner}: ${kotlin.run {
        val __proj_73: Fighter = kotlin.run {
            val __pick_70: Fighter? = salvo.core.list.getPlatform(team.members, 0)
            (if ((__pick_70 != null)) {
                val __picked_71: Fighter = __pick_70!!
                __picked_71
            } else {
                throw AssertionError("salvo: unreachable at main:294:60")
            })
        }
        __proj_73.energy
    }} ${kotlin.run {
        val __proj_77: Fighter = kotlin.run {
            val __pick_74: Fighter? = salvo.core.list.getPlatform(team.members, 1)
            (if ((__pick_74 != null)) {
                val __picked_75: Fighter = __pick_74!!
                __picked_75
            } else {
                throw AssertionError("salvo: unreachable at main:294:111")
            })
        }
        __proj_77.energy
    }}")
    val camp: Camp = Camp(supplies = 10, banners = mutableListOf<String>("red"))
    val banners: salvo.platform.core.list.MutList<String> = camp.banners
    spend(camp, 3)
    hoist(camp, "blue")
    salvo.core.console.println(__handle_2, "5. supplies ${camp.supplies}, banners ${salvo.core.list.toStr(banners, { __a0 -> __a0 })}")
}

