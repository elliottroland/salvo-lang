package salvo.args

import salvo.*

interface Args {
    fun args(): List<String>
}

class __Mon_Args(
    private val inner: Args,
    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),
) : Args {
    override fun args(): List<String> {
        check(!lock.isHeldByCurrentThread) { "salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]" }
        lock.lock()
        try { return inner.args() } finally { lock.unlock() }
    }
}

interface ArgsPlatform {
    fun args(): List<String>
}

open class __Platform_Args(private val impl: ArgsPlatform) : Args {
    override fun args(): List<String> = impl.args()
}


class __Platform_HostArgs() : __Platform_Args(salvo.platform.args.HostArgs())

data class UnknownOption(
    val option: String,
)

object __Codec_UnknownOption : salvo.WireCodec<UnknownOption> {
    override fun enc(v: UnknownOption, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.option, out)
    }
    override fun dec(inp: salvo.WireIn): UnknownOption = UnknownOption(salvo.StrCodec.dec(inp))
}

data class MissingValue(
    val option: String,
)

object __Codec_MissingValue : salvo.WireCodec<MissingValue> {
    override fun enc(v: MissingValue, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.option, out)
    }
    override fun dec(inp: salvo.WireIn): MissingValue = MissingValue(salvo.StrCodec.dec(inp))
}

data class Missing(
    val what: String,
)

object __Codec_Missing : salvo.WireCodec<Missing> {
    override fun enc(v: Missing, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.what, out)
    }
    override fun dec(inp: salvo.WireIn): Missing = Missing(salvo.StrCodec.dec(inp))
}

data class Invalid(
    val what: String,
    val value: String,
    val reason: String,
)

object __Codec_Invalid : salvo.WireCodec<Invalid> {
    override fun enc(v: Invalid, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.what, out)
        salvo.StrCodec.enc(v.value, out)
        salvo.StrCodec.enc(v.reason, out)
    }
    override fun dec(inp: salvo.WireIn): Invalid = Invalid(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class Unexpected(
    val value: String,
)

object __Codec_Unexpected : salvo.WireCodec<Unexpected> {
    override fun enc(v: Unexpected, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): Unexpected = Unexpected(salvo.StrCodec.dec(inp))
}

data class Repeated(
    val option: String,
)

object __Codec_Repeated : salvo.WireCodec<Repeated> {
    override fun enc(v: Repeated, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.option, out)
    }
    override fun dec(inp: salvo.WireIn): Repeated = Repeated(salvo.StrCodec.dec(inp))
}

data class Requires(
    val option: String,
    val needs: String,
)

object __Codec_Requires : salvo.WireCodec<Requires> {
    override fun enc(v: Requires, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.option, out)
        salvo.StrCodec.enc(v.needs, out)
    }
    override fun dec(inp: salvo.WireIn): Requires = Requires(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class Conflict(
    val option: String,
    val other: String,
)

object __Codec_Conflict : salvo.WireCodec<Conflict> {
    override fun enc(v: Conflict, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.option, out)
        salvo.StrCodec.enc(v.other, out)
    }
    override fun dec(inp: salvo.WireIn): Conflict = Conflict(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class MissingOne(
    val options: List<String>,
)

object __Codec_MissingOne : salvo.WireCodec<MissingOne> {
    override fun enc(v: MissingOne, out: salvo.WireOut) {
        salvo.ListCodec(salvo.StrCodec).enc(v.options, out)
    }
    override fun dec(inp: salvo.WireIn): MissingOne = MissingOne(salvo.ListCodec(salvo.StrCodec).dec(inp))
}

data class UnknownCommand(
    val name: String,
)

object __Codec_UnknownCommand : salvo.WireCodec<UnknownCommand> {
    override fun enc(v: UnknownCommand, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.name, out)
    }
    override fun dec(inp: salvo.WireIn): UnknownCommand = UnknownCommand(salvo.StrCodec.dec(inp))
}

data class HelpRequested(
    val text: String,
)

object __Codec_HelpRequested : salvo.WireCodec<HelpRequested> {
    override fun enc(v: HelpRequested, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.text, out)
    }
    override fun dec(inp: salvo.WireIn): HelpRequested = HelpRequested(salvo.StrCodec.dec(inp))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun message(e: Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>): String {
    return when {
        (e is Union11.U1<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_1: UnknownOption = ((e as Union11.U1<*, *, *, *, *, *, *, *, *, *, *>).value as UnknownOption)
            return "unknown option '${e_1.option}'"
        }
        (e is Union11.U2<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_2: MissingValue = ((e as Union11.U2<*, *, *, *, *, *, *, *, *, *, *>).value as MissingValue)
            return "option '${e_2.option}' needs a value"
        }
        (e is Union11.U3<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_3: Missing = ((e as Union11.U3<*, *, *, *, *, *, *, *, *, *, *>).value as Missing)
            return "'${e_3.what}' is required"
        }
        (e is Union11.U4<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_4: Invalid = ((e as Union11.U4<*, *, *, *, *, *, *, *, *, *, *>).value as Invalid)
            return "invalid value '${e_4.value}' for '${e_4.what}': ${e_4.reason}"
        }
        (e is Union11.U5<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_5: Unexpected = ((e as Union11.U5<*, *, *, *, *, *, *, *, *, *, *>).value as Unexpected)
            return "unexpected argument '${e_5.value}'"
        }
        (e is Union11.U6<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_6: Repeated = ((e as Union11.U6<*, *, *, *, *, *, *, *, *, *, *>).value as Repeated)
            return "option '${e_6.option}' was given more than once"
        }
        (e is Union11.U7<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_7: Requires = ((e as Union11.U7<*, *, *, *, *, *, *, *, *, *, *>).value as Requires)
            return "'${e_7.option}' requires '${e_7.needs}'"
        }
        (e is Union11.U8<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_8: Conflict = ((e as Union11.U8<*, *, *, *, *, *, *, *, *, *, *>).value as Conflict)
            return "'${e_8.option}' cannot be used with '${e_8.other}'"
        }
        (e is Union11.U9<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_9: MissingOne = ((e as Union11.U9<*, *, *, *, *, *, *, *, *, *, *>).value as MissingOne)
            return "one of '${salvo.core.string.joinPlatform(e_9.options, "', '")}' is required"
        }
        (e is Union11.U10<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_10: UnknownCommand = ((e as Union11.U10<*, *, *, *, *, *, *, *, *, *, *>).value as UnknownCommand)
            return "unrecognized command '${e_10.name}'"
        }
        (e is Union11.U11<*, *, *, *, *, *, *, *, *, *, *>) -> {
            val e_11: HelpRequested = ((e as Union11.U11<*, *, *, *, *, *, *, *, *, *, *>).value as HelpRequested)
            return "help was requested"
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at args:121:5"))
        }
    }
}

data class OptSpec(
    val field: String,
    val short: Char? = null,
    val takesValue: Boolean = true,
    val repeats: Boolean = false,
    val required: Boolean = false,
    val help: String? = null,
    val defaultText: String? = null,
    val choices: List<String> = listOf<String>(),
    val group: String? = null,
)

object __Codec_OptSpec : salvo.WireCodec<OptSpec> {
    override fun enc(v: OptSpec, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.field, out)
        salvo.OptCodec(salvo.CharCodec).enc(v.short, out)
        salvo.BoolCodec.enc(v.takesValue, out)
        salvo.BoolCodec.enc(v.repeats, out)
        salvo.BoolCodec.enc(v.required, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.help, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.defaultText, out)
        salvo.ListCodec(salvo.StrCodec).enc(v.choices, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.group, out)
    }
    override fun dec(inp: salvo.WireIn): OptSpec = OptSpec(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.CharCodec).dec(inp), salvo.BoolCodec.dec(inp), salvo.BoolCodec.dec(inp), salvo.BoolCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.ListCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class GroupSpec(
    val key: String,
    val kind: String,
    val required: Boolean = false,
    val heading: String? = null,
)

object __Codec_GroupSpec : salvo.WireCodec<GroupSpec> {
    override fun enc(v: GroupSpec, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.key, out)
        salvo.StrCodec.enc(v.kind, out)
        salvo.BoolCodec.enc(v.required, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.heading, out)
    }
    override fun dec(inp: salvo.WireIn): GroupSpec = GroupSpec(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.BoolCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

data class ArgSpec(
    val field: String,
    val help: String? = null,
    val required: Boolean = true,
    val many: Boolean = false,
)

object __Codec_ArgSpec : salvo.WireCodec<ArgSpec> {
    override fun enc(v: ArgSpec, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.field, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.help, out)
        salvo.BoolCodec.enc(v.required, out)
        salvo.BoolCodec.enc(v.many, out)
    }
    override fun dec(inp: salvo.WireIn): ArgSpec = ArgSpec(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.BoolCodec.dec(inp), salvo.BoolCodec.dec(inp))
}

data class Spec(
    val name: String,
    val about: String? = null,
    val opts: List<OptSpec>,
    val args: List<ArgSpec>,
    val groups: List<GroupSpec> = listOf<GroupSpec>(),
    val commands: List<CmdSpec> = listOf<CmdSpec>(),
    val commandRequired: Boolean = false,
    val commandFields: Int = 0,
)

object __Codec_Spec : salvo.WireCodec<Spec> {
    override fun enc(v: Spec, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.name, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.about, out)
        salvo.ListCodec(__Codec_OptSpec).enc(v.opts, out)
        salvo.ListCodec(__Codec_ArgSpec).enc(v.args, out)
        salvo.ListCodec(__Codec_GroupSpec).enc(v.groups, out)
        salvo.ListCodec(__Codec_CmdSpec).enc(v.commands, out)
        salvo.BoolCodec.enc(v.commandRequired, out)
        salvo.IntCodec.enc(v.commandFields, out)
    }
    override fun dec(inp: salvo.WireIn): Spec = Spec(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.ListCodec(__Codec_OptSpec).dec(inp), salvo.ListCodec(__Codec_ArgSpec).dec(inp), salvo.ListCodec(__Codec_GroupSpec).dec(inp), salvo.ListCodec(__Codec_CmdSpec).dec(inp), salvo.BoolCodec.dec(inp), salvo.IntCodec.dec(inp))
}

data class CmdSpec(
    val name: String,
    val about: String? = null,
)

object __Codec_CmdSpec : salvo.WireCodec<CmdSpec> {
    override fun enc(v: CmdSpec, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.name, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.about, out)
    }
    override fun dec(inp: salvo.WireIn): CmdSpec = CmdSpec(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp))
}

fun mergeGroup(opts: salvo.platform.core.list.MutList<OptSpec>, groups: salvo.platform.core.list.MutList<GroupSpec>, child: Spec, key: String, kind: String, heading: String?) {
    if (!(salvo.core.list.isEmpty(child.args))) {
        throw AssertionError(("salvo: " + ("`${key}` is a group of options, and `${child.name}` takes positionals") + " at args:217:9"))
    }
    salvo.core.list.addPlatform(groups, GroupSpec(key = key, kind = kind, required = false, heading = run {
        val __pick_1: String? = heading
        (if ((__pick_1 != null)) {
            val __picked_2: String = __pick_1!!
            __picked_2
        } else {
            child.about
        })
    }))
    for (g in salvo.platform.core.list.each(child.groups)) {
        salvo.core.list.addPlatform(groups, GroupSpec(key = "${key}/${g.key}", kind = g.kind, required = g.required, heading = g.heading))
    }
    for (o in salvo.platform.core.list.each(child.opts)) {
        val group: String = (if ((o.group == null)) {
            key
        } else {
            val __narrowed_4: String = o.group!!
            "${key}/${__narrowed_4}"
        })
        salvo.core.list.addPlatform(opts, OptSpec(field = o.field, short = o.short, takesValue = o.takesValue, repeats = o.repeats, required = o.required, help = o.help, defaultText = o.defaultText, choices = o.choices, group = group))
    }
    if ((((kind) == ("arm")) && salvo.core.list.isEmpty(child.opts))) {
        salvo.core.list.addPlatform(opts, OptSpec(field = salvo.core.string.replacePlatform(commandName(child.name), "-", "_"), short = null, takesValue = false, repeats = false, required = false, help = child.about, defaultText = null, choices = listOf<String>(), group = key))
    }
}

fun addOne(groups: salvo.platform.core.list.MutList<GroupSpec>, key: String, required: Boolean, heading: String?) {
    salvo.core.list.addPlatform(groups, GroupSpec(key = key, kind = "one", required = required, heading = heading))
}

fun anyGiven(found: Found, spec: Spec): Boolean {
    if (salvo.core.list.isEmpty(spec.opts)) {
        return given(found, salvo.core.string.replacePlatform(commandName(spec.name), "-", "_"))
    }
    for (o in salvo.platform.core.list.each(spec.opts)) {
        if (given(found, o.field)) {
            return true
        }
    }
    return false
}

fun longName(field: String): String {
    return "--${salvo.core.string.replacePlatform(field, "_", "-")}"
}

fun argName(field: String): String {
    return "<${salvo.core.string.replacePlatform(field, "_", "-")}>"
}

fun commandName(typeName: String): String {
    var last: String = typeName
    val cut: Pair<String, String>? = salvo.core.string.splitLast(typeName, ".")
    if (!((cut == null))) {
        val cut_1: Pair<String, String> = cut!!
        val __destructured_2: Pair<String, String> = cut_1
        val __unused_3: String = __destructured_2.first
        val tail: String = __destructured_2.second
        last = tail
    }
    val out: salvo.platform.core.string.MutStr = salvo.core.string.Mut_Str(arrayOf<String>(""))
    var first: Boolean = true
    for (c in salvo.platform.core.string.each(last)) {
        val lower: String = salvo.core.string.toLowerPlatform("${c}")
        if ((!(first) && !(((lower) == ("${c}"))))) {
            salvo.core.string.appendPlatform(out, "-")
        }
        salvo.core.string.appendPlatform(out, lower)
        first = false
    }
    return out.toString()
}

data class Hit(
    val field: String,
    val value: String,
)

object __Codec_Hit : salvo.WireCodec<Hit> {
    override fun enc(v: Hit, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.field, out)
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): Hit = Hit(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class Found(
    val hits: List<Hit>,
    val errors: List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>,
    val help: Boolean,
    val command: String? = null,
    val rest: List<String> = listOf<String>(),
    val path: String = "",
)

object __Codec_Found : salvo.WireCodec<Found> {
    override fun enc(v: Found, out: salvo.WireOut) {
        salvo.ListCodec(__Codec_Hit).enc(v.hits, out)
        salvo.ListCodec(salvo.Union11Codec(__Codec_UnknownOption, __Codec_MissingValue, __Codec_Missing, __Codec_Invalid, __Codec_Unexpected, __Codec_Repeated, __Codec_Requires, __Codec_Conflict, __Codec_MissingOne, __Codec_UnknownCommand, __Codec_HelpRequested)).enc(v.errors, out)
        salvo.BoolCodec.enc(v.help, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.command, out)
        salvo.ListCodec(salvo.StrCodec).enc(v.rest, out)
        salvo.StrCodec.enc(v.path, out)
    }
    override fun dec(inp: salvo.WireIn): Found = Found(salvo.ListCodec(__Codec_Hit).dec(inp), salvo.ListCodec(salvo.Union11Codec(__Codec_UnknownOption, __Codec_MissingValue, __Codec_Missing, __Codec_Invalid, __Codec_Unexpected, __Codec_Repeated, __Codec_Requires, __Codec_Conflict, __Codec_MissingOne, __Codec_UnknownCommand, __Codec_HelpRequested)).dec(inp), salvo.BoolCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.ListCodec(salvo.StrCodec).dec(inp), salvo.StrCodec.dec(inp))
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun scan(spec: Spec, args: List<String>): Found {
    val tokens: List<Union4<salvo.args.lex.LongToken, salvo.args.lex.ShortToken, salvo.args.lex.WordToken, salvo.args.lex.EndToken>> = salvo.args.lex.lex(args)
    val hits: salvo.platform.core.list.MutList<Hit> = mutableListOf<Hit>()
    val errors: salvo.platform.core.list.MutList<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>> = mutableListOf<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>()
    val words: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    var help: Boolean = false
    var command: String? = null
    var failedCommand: Boolean = false
    val rest: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val n: Int = salvo.core.list.sizePlatform(tokens)
    var i: Int = 0
    while (true) {
        if (!((i < n))) {
            break
        }
        val t: Union4<salvo.args.lex.LongToken, salvo.args.lex.ShortToken, salvo.args.lex.WordToken, salvo.args.lex.EndToken> = run {
            val __nn_1: Union4<salvo.args.lex.LongToken, salvo.args.lex.ShortToken, salvo.args.lex.WordToken, salvo.args.lex.EndToken>? = salvo.core.list.getPlatform(tokens, i)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at args:334:22"))
                }
                else -> {
                    val __some_2: Union4<salvo.args.lex.LongToken, salvo.args.lex.ShortToken, salvo.args.lex.WordToken, salvo.args.lex.EndToken> = __nn_1!!
                    __some_2
                }
            }
        }
        i = (i + 1)
        when {
            (t is Union4.U4<*, *, *, *>) -> {
                val t_3: salvo.args.lex.EndToken = ((t as Union4.U4<*, *, *, *>).value as salvo.args.lex.EndToken)
            }
            (t is Union4.U3<*, *, *, *>) -> {
                val t_4: salvo.args.lex.WordToken = ((t as Union4.U3<*, *, *, *>).value as salvo.args.lex.WordToken)
                if (((!(salvo.core.list.isEmpty(spec.commands)) && (command == null)) && !(failedCommand))) {
                    if (hasCommand(spec, t_4.value)) {
                        command = t_4.value
                        while (true) {
                            if (!((i < n))) {
                                break
                            }
                            salvo.core.list.addPlatform(rest, run {
                                val __nn_5: String? = salvo.core.list.getPlatform(args, i)
                                when {
                                    (__nn_5 == null) -> {
                                        throw AssertionError(("salvo: " + ("value is absent") + " at args:345:44"))
                                    }
                                    else -> {
                                        val __some_6: String = __nn_5!!
                                        __some_6
                                    }
                                }
                            })
                            i = (i + 1)
                        }
                    } else {
                        salvo.core.list.addPlatform(errors, Union11.U10<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(UnknownCommand(name = t_4.value)))
                        failedCommand = true
                        i = n
                    }
                } else {
                    salvo.core.list.addPlatform(words, t_4.value)
                }
            }
            (t is Union4.U1<*, *, *, *>) -> {
                val t_7: salvo.args.lex.LongToken = ((t as Union4.U1<*, *, *, *>).value as salvo.args.lex.LongToken)
                if (((t_7.name) == ("help"))) {
                    help = true
                } else {
                    val o: OptSpec? = longOpt(spec, t_7.name)
                    if ((o == null)) {
                        salvo.core.list.addPlatform(errors, Union11.U1<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(UnknownOption(option = "--${t_7.name}")))
                    } else {
                        val o_8: OptSpec = o!!
                        val flag: String = longName(o_8.field)
                        val attached: String? = t_7.value
                        if ((!(o_8.takesValue) && (attached == null))) {
                            salvo.core.list.addPlatform(hits, Hit(field = o_8.field, value = ""))
                        } else if (!(o_8.takesValue)) {
                            salvo.core.list.addPlatform(errors, Union11.U4<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Invalid(what = flag, value = run {
                                val __pick_9: String? = attached
                                (if ((__pick_9 != null)) {
                                    val __picked_10: String = __pick_9!!
                                    __picked_10
                                } else {
                                    ""
                                })
                            }, reason = "the option takes no value")))
                        } else if (!((attached == null))) {
                            val attached_12: String = attached!!
                            salvo.core.list.addPlatform(hits, Hit(field = o_8.field, value = attached_12))
                        } else {
                            val v: String? = wordAt(tokens, i)
                            if ((v == null)) {
                                salvo.core.list.addPlatform(errors, Union11.U2<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(MissingValue(option = flag)))
                            } else {
                                val v_13: String = v!!
                                salvo.core.list.addPlatform(hits, Hit(field = o_8.field, value = v_13))
                                i = (i + 1)
                            }
                        }
                    }
                }
            }
            (t is Union4.U2<*, *, *, *>) -> {
                val t_14: salvo.args.lex.ShortToken = ((t as Union4.U2<*, *, *, *>).value as salvo.args.lex.ShortToken)
                val letters: String = t_14.letters
                var k: Int = 0
                val len: Int = salvo.core.string.sizePlatform(letters)
                while (true) {
                    if (!((k < len))) {
                        break
                    }
                    val c: Char = run {
                        val __nn_15: Char? = salvo.core.string.charAtPlatform(letters, k)
                        when {
                            (__nn_15 == null) -> {
                                throw AssertionError(("salvo: " + ("value is absent") + " at args:395:29"))
                            }
                            else -> {
                                val __some_16: Char = __nn_15!!
                                __some_16
                            }
                        }
                    }
                    k = (k + 1)
                    if ((c == 'h')) {
                        help = true
                    } else {
                        val o: OptSpec? = shortOpt(spec, c)
                        if ((o == null)) {
                            salvo.core.list.addPlatform(errors, Union11.U1<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(UnknownOption(option = "-${c}")))
                        } else {
                            val o_17: OptSpec = o!!
                            if (!(o_17.takesValue)) {
                                val o_18: OptSpec = o!!
                                salvo.core.list.addPlatform(hits, Hit(field = o_18.field, value = ""))
                            } else {
                                if ((k < len)) {
                                    val o_19: OptSpec = o!!
                                    salvo.core.list.addPlatform(hits, Hit(field = o_19.field, value = run {
                                        val __nn_20: String? = salvo.core.string.substrPlatform(letters, k, len)
                                        when {
                                            (__nn_20 == null) -> {
                                                throw AssertionError(("salvo: " + ("value is absent") + " at args:411:78"))
                                            }
                                            else -> {
                                                val __some_21: String = __nn_20!!
                                                __some_21
                                            }
                                        }
                                    }))
                                    k = len
                                } else {
                                    val v: String? = wordAt(tokens, i)
                                    if ((v == null)) {
                                        salvo.core.list.addPlatform(errors, Union11.U2<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(MissingValue(option = "-${c}")))
                                    } else {
                                        val o_22: OptSpec = o!!
                                        val v_23: String = v!!
                                        salvo.core.list.addPlatform(hits, Hit(field = o_22.field, value = v_23))
                                        i = (i + 1)
                                    }
                                }
                            }
                        }
                    }
                }
            }
            else -> {
                throw AssertionError(("salvo: " + ("unhandled union arm") + " at args:336:9"))
            }
        }
    }
    for (o in salvo.platform.core.list.each(spec.opts)) {
        val flag: String = longName(o.field)
        var count: Int = 0
        for (h in salvo.platform.core.list.each(hits)) {
            if (((h.field) == (o.field))) {
                count = (count + 1)
                if ((!(salvo.core.list.isEmpty(o.choices)) && !(salvo.core.list.contains(o.choices, h.value, { __a0, __a1 -> ((__a0) == (__a1)) })))) {
                    salvo.core.list.addPlatform(errors, Union11.U4<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Invalid(what = flag, value = h.value, reason = "expected one of ${salvo.core.string.joinPlatform(o.choices, ", ")}")))
                }
            }
        }
        if (((count > 1) && !(o.repeats))) {
            salvo.core.list.addPlatform(errors, Union11.U6<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Repeated(option = flag)))
        }
        if (((count == 0) && o.required)) {
            val why: String? = unmetGroup(spec, hits, o.group)
            if ((why != null)) {
                val why_24: String = why!!
                salvo.core.list.addPlatform(errors, Union11.U7<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Requires(option = why_24, needs = flag)))
            } else if (live(spec, hits, o.group)) {
                salvo.core.list.addPlatform(errors, Union11.U3<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Missing(what = flag)))
            } else {
            }
        }
    }
    for (g in salvo.platform.core.list.each(spec.groups)) {
        if ((((g.kind) == ("one")) && live(spec, hits, g.key))) {
            var first: String? = null
            val names: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
            for (a in salvo.platform.core.list.each(spec.groups)) {
                if ((((a.kind) == ("arm")) && parentIs(a.key, g.key))) {
                    salvo.core.list.addPlatform(names, firstOption(spec, a.key))
                    val here: String? = givenIn(spec, hits, a.key)
                    if (!((here == null))) {
                        if ((first == null)) {
                            val here_25: String = here!!
                            first = here_25
                        } else {
                            val here_26: String = here!!
                            val first_27: String = first!!
                            salvo.core.list.addPlatform(errors, Union11.U8<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Conflict(option = here_26, other = first_27)))
                        }
                    }
                }
            }
            if (((first == null) && g.required)) {
                salvo.core.list.addPlatform(errors, Union11.U9<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(MissingOne(options = names)))
            }
        }
    }
    var w: Int = 0
    val nw: Int = salvo.core.list.sizePlatform(words)
    for (a in salvo.platform.core.list.each(spec.args)) {
        if (a.many) {
            if (((w == nw) && a.required)) {
                salvo.core.list.addPlatform(errors, Union11.U3<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Missing(what = argName(a.field))))
            }
            while (true) {
                if (!((w < nw))) {
                    break
                }
                salvo.core.list.addPlatform(hits, Hit(field = a.field, value = run {
                    val __nn_28: String? = salvo.core.list.getPlatform(words, w)
                    when {
                        (__nn_28 == null) -> {
                            throw AssertionError(("salvo: " + ("value is absent") + " at args:487:67"))
                        }
                        else -> {
                            val __some_29: String = __nn_28!!
                            __some_29
                        }
                    }
                }))
                w = (w + 1)
            }
        } else {
            if ((w < nw)) {
                salvo.core.list.addPlatform(hits, Hit(field = a.field, value = run {
                    val __nn_30: String? = salvo.core.list.getPlatform(words, w)
                    when {
                        (__nn_30 == null) -> {
                            throw AssertionError(("salvo: " + ("value is absent") + " at args:493:71"))
                        }
                        else -> {
                            val __some_31: String = __nn_30!!
                            __some_31
                        }
                    }
                }))
                w = (w + 1)
            } else if (a.required) {
                salvo.core.list.addPlatform(errors, Union11.U3<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Missing(what = argName(a.field))))
            } else {
            }
        }
    }
    while (true) {
        if (!((w < nw))) {
            break
        }
        salvo.core.list.addPlatform(errors, Union11.U5<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Unexpected(value = run {
            val __nn_32: String? = salvo.core.list.getPlatform(words, w)
            when {
                (__nn_32 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at args:504:46"))
                }
                else -> {
                    val __some_33: String = __nn_32!!
                    __some_33
                }
            }
        })))
        w = (w + 1)
    }
    if ((((!(salvo.core.list.isEmpty(spec.commands)) && (command == null)) && !(failedCommand)) && spec.commandRequired)) {
        salvo.core.list.addPlatform(errors, Union11.U3<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Missing(what = "<command>")))
    }
    return Found(hits = hits, errors = errors, help = help, command = command, rest = rest, path = spec.name)
}

fun parentKey(key: String): String? {
    val cut: Pair<String, String>? = salvo.core.string.splitLast(key, "/")
    if ((cut == null)) {
        return null
    }
    val cut_1: Pair<String, String> = cut!!
    val __destructured_2: Pair<String, String> = cut_1
    val parent: String = __destructured_2.first
    val __unused_3: String = __destructured_2.second
    return parent
}

fun parentIs(key: String, parent: String): Boolean {
    val p: String? = parentKey(key)
    if ((p == null)) {
        return false
    }
    val p_1: String = p!!
    return ((p_1) == (parent))
}

fun inGroup(group: String?, key: String): Boolean {
    if ((group == null)) {
        return false
    }
    val group_1: String = group!!
    return ((group_1) == (key))
}

fun givenIn(spec: Spec, hits: List<Hit>, key: String): String? {
    for (h in salvo.platform.core.list.each(hits)) {
        for (o in salvo.platform.core.list.each(spec.opts)) {
            if ((((o.field) == (h.field)) && !((o.group == null)))) {
                val __narrowed_1: String = o.group!!
                val g: String = __narrowed_1
                if ((((g) == (key)) || salvo.core.string.startsWithPlatform(g, "${key}/"))) {
                    return longName(o.field)
                }
            }
        }
    }
    return null
}

fun firstOption(spec: Spec, key: String): String {
    for (o in salvo.platform.core.list.each(spec.opts)) {
        if ((o.required && inGroup(o.group, key))) {
            return longName(o.field)
        }
    }
    for (o in salvo.platform.core.list.each(spec.opts)) {
        if (inGroup(o.group, key)) {
            return longName(o.field)
        }
    }
    return key
}

fun live(spec: Spec, hits: List<Hit>, key: String?): Boolean {
    var k: String? = key
    while (true) {
        if (!(!((k == null)))) {
            break
        }
        val k_1: String = k!!
        val here: String = k_1
        for (g in salvo.platform.core.list.each(spec.groups)) {
            if (((((g.key) == (here)) && (((g.kind) == ("together")) || ((g.kind) == ("arm")))) && (givenIn(spec, hits, here) == null))) {
                return false
            }
        }
        k = parentKey(here)
    }
    return true
}

fun unmetGroup(spec: Spec, hits: List<Hit>, key: String?): String? {
    var k: String? = key
    while (true) {
        if (!(!((k == null)))) {
            break
        }
        val k_1: String = k!!
        val here: String = k_1
        for (g in salvo.platform.core.list.each(spec.groups)) {
            if ((((g.key) == (here)) && (((g.kind) == ("together")) || ((g.kind) == ("arm"))))) {
                val why: String? = givenIn(spec, hits, here)
                if ((!((why == null)) && live(spec, hits, parentKey(here)))) {
                    val why_2: String = why!!
                    return why_2
                }
                return null
            }
        }
        k = parentKey(here)
    }
    return null
}

fun hasCommand(spec: Spec, name: String): Boolean {
    for (c in salvo.platform.core.list.each(spec.commands)) {
        if (((c.name) == (name))) {
            return true
        }
    }
    return false
}

fun isCommand(found: Found, name: String): Boolean {
    val c: String? = found.command
    if ((c == null)) {
        return false
    }
    val c_1: String = c!!
    return ((c_1) == (name))
}

fun longOpt(spec: Spec, name: String): OptSpec? {
    for (o in salvo.platform.core.list.each(spec.opts)) {
        if (((longName(o.field)) == ("--${name}"))) {
            return o
        }
    }
    return null
}

fun shortOpt(spec: Spec, c: Char): OptSpec? {
    for (o in salvo.platform.core.list.each(spec.opts)) {
        if (run {
            val __cmp_1: Char? = o.short
            (if ((__cmp_1 != null)) {
                val __arm_2: Char = __cmp_1!!
                (__arm_2 == c)
            } else {
                false
            })
        }) {
            return o
        }
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun wordAt(tokens: List<Union4<salvo.args.lex.LongToken, salvo.args.lex.ShortToken, salvo.args.lex.WordToken, salvo.args.lex.EndToken>>, i: Int): String? {
    val t: Union4<salvo.args.lex.LongToken, salvo.args.lex.ShortToken, salvo.args.lex.WordToken, salvo.args.lex.EndToken>? = salvo.core.list.getPlatform(tokens, i)
    if ((t == null)) {
        return null
    }
    val t_1: Union4<salvo.args.lex.LongToken, salvo.args.lex.ShortToken, salvo.args.lex.WordToken, salvo.args.lex.EndToken> = t!!
    val w: Union4<salvo.args.lex.LongToken, salvo.args.lex.ShortToken, salvo.args.lex.WordToken, salvo.args.lex.EndToken> = t_1
    if ((w is Union4.U3<*, *, *, *>)) {
        val w_2: salvo.args.lex.WordToken = ((w as Union4.U3<*, *, *, *>).value as salvo.args.lex.WordToken)
        return w_2.value
    }
    return null
}

fun given(found: Found, field: String): Boolean {
    for (h in salvo.platform.core.list.each(found.hits)) {
        if (((h.field) == (field))) {
            return true
        }
    }
    return false
}

fun valueOf(found: Found, field: String): String? {
    for (h in salvo.platform.core.list.each(found.hits)) {
        if (((h.field) == (field))) {
            return h.value
        }
    }
    return null
}

fun valuesOf(found: Found, field: String): List<String> {
    val out: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    for (h in salvo.platform.core.list.each(found.hits)) {
        if (((h.field) == (field))) {
            salvo.core.list.addPlatform(out, h.value)
        }
    }
    return out
}

fun usage(spec: Spec): String {
    val out: salvo.platform.core.string.MutStr = salvo.core.string.Mut_Str(arrayOf<String>("Usage: ${spec.name}"))
    if (!(salvo.core.list.isEmpty(spec.opts))) {
        salvo.core.string.appendPlatform(out, " [OPTIONS]")
    }
    val none: List<Hit> = listOf<Hit>()
    for (o in salvo.platform.core.list.each(spec.opts)) {
        if ((o.required && live(spec, none, o.group))) {
            salvo.core.string.appendPlatform(out, " ${longName(o.field)} <${valueName(o.field)}>")
        }
    }
    for (a in salvo.platform.core.list.each(spec.args)) {
        val name: String = argName(a.field)
        val shown: String = (if (a.many) {
            "${name}..."
        } else {
            name
        })
        salvo.core.string.appendPlatform(out, (if (a.required) {
            " ${shown}"
        } else {
            " [${shown}]"
        }))
    }
    if (!(salvo.core.list.isEmpty(spec.commands))) {
        salvo.core.string.appendPlatform(out, (if (spec.commandRequired) {
            " <COMMAND>"
        } else {
            " [COMMAND]"
        }))
    }
    return out.toString()
}

fun valueName(field: String): String {
    return salvo.core.string.toUpperPlatform(salvo.core.string.replacePlatform(field, "_", "-"))
}

fun helpText(spec: Spec): String {
    val out: salvo.platform.core.string.MutStr = salvo.core.string.Mut_Str(arrayOf<String>(""))
    if (!((spec.about == null))) {
        val __narrowed_1: String = spec.about!!
        salvo.core.string.appendPlatform(out, "${__narrowed_1}\n\n")
    }
    salvo.core.string.appendPlatform(out, "${usage(spec)}\n")
    val lefts: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val rights: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    for (a in salvo.platform.core.list.each(spec.args)) {
        salvo.core.list.addPlatform(lefts, (if (a.many) {
            "${argName(a.field)}..."
        } else {
            argName(a.field)
        }))
        salvo.core.list.addPlatform(rights, oneLine(a.help))
    }
    val nargs: Int = salvo.core.list.sizePlatform(lefts)
    for (c in salvo.platform.core.list.each(spec.commands)) {
        salvo.core.list.addPlatform(lefts, c.name)
        salvo.core.list.addPlatform(rights, oneLine(c.about))
    }
    val ncommands: Int = (salvo.core.list.sizePlatform(lefts) - nargs)
    val sections: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val sectionOf: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    for (o in salvo.platform.core.list.each(spec.opts)) {
        val heading: String = section(spec, o.group)
        if ((!(((heading) == (""))) && !(salvo.core.list.contains(sections, heading, { __a0, __a1 -> ((__a0) == (__a1)) })))) {
            salvo.core.list.addPlatform(sections, heading)
        }
        salvo.core.list.addPlatform(sectionOf, heading)
    }
    for (o in salvo.platform.core.list.each(spec.opts)) {
        val left: salvo.platform.core.string.MutStr = salvo.core.string.Mut_Str(arrayOf<String>((if ((o.short == null)) {
            "    "
        } else {
            val __narrowed_2: Char = o.short!!
            "-${__narrowed_2}, "
        })))
        salvo.core.string.appendPlatform(left, longName(o.field))
        if (o.takesValue) {
            salvo.core.string.appendPlatform(left, " <${valueName(o.field)}>")
        }
        salvo.core.list.addPlatform(lefts, StringBuilder(left).toString())
        val right: salvo.platform.core.string.MutStr = salvo.core.string.Mut_Str(arrayOf<String>(oneLine(o.help)))
        if (!(salvo.core.list.isEmpty(o.choices))) {
            salvo.core.string.appendPlatform(right, " [possible values: ${salvo.core.string.joinPlatform(o.choices, ", ")}]")
        }
        if (!((o.defaultText == null))) {
            val __narrowed_3: String = o.defaultText!!
            salvo.core.string.appendPlatform(right, " [default: ${__narrowed_3}]")
        }
        salvo.core.list.addPlatform(rights, salvo.core.string.trimPlatform(right.toString()))
    }
    salvo.core.list.addPlatform(lefts, "-h, --help")
    salvo.core.list.addPlatform(rights, "Print help")
    var width: Int = 0
    for (l in salvo.platform.core.list.each(lefts)) {
        if ((salvo.core.string.sizePlatform(l) > width)) {
            width = salvo.core.string.sizePlatform(l)
        }
    }
    val helpRow: Int = (salvo.core.list.sizePlatform(lefts) - 1)
    val rows: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    if ((nargs > 0)) {
        salvo.core.string.appendPlatform(out, "\nArguments:\n")
        var i: Int = 0
        while (true) {
            if (!((i < nargs))) {
                break
            }
            salvo.core.list.addPlatform(rows, (i + 0))
            i = (i + 1)
        }
        helpRows(out, lefts, rights, rows, width)
    }
    if ((ncommands > 0)) {
        salvo.core.string.appendPlatform(out, "\nCommands:\n")
        val commands: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
        var c: Int = 0
        while (true) {
            if (!((c < ncommands))) {
                break
            }
            salvo.core.list.addPlatform(commands, ((nargs + c) + 0))
            c = (c + 1)
        }
        helpRows(out, lefts, rights, commands, width)
    }
    val firstOpt: Int = (nargs + ncommands)
    val mine: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
    var k: Int = 0
    for (s in salvo.platform.core.list.each(sectionOf)) {
        if (((s) == (""))) {
            salvo.core.list.addPlatform(mine, (firstOpt + k))
        }
        k = (k + 1)
    }
    salvo.core.list.addPlatform(mine, helpRow)
    salvo.core.string.appendPlatform(out, "\nOptions:\n")
    helpRows(out, lefts, rights, mine, width)
    for (heading in salvo.platform.core.list.each(sections)) {
        val these: salvo.platform.core.list.MutList<Int> = mutableListOf<Int>()
        var j: Int = 0
        for (s in salvo.platform.core.list.each(sectionOf)) {
            if (((s) == (heading))) {
                salvo.core.list.addPlatform(these, (firstOpt + j))
            }
            j = (j + 1)
        }
        salvo.core.string.appendPlatform(out, "\n${heading}:\n")
        helpRows(out, lefts, rights, these, width)
    }
    return out.toString()
}

fun oneLine(text: String?): String {
    if ((text == null)) {
        return ""
    }
    val text_1: String = text!!
    return salvo.core.string.replacePlatform(text_1, "\n", " ")
}

fun helpRows(out: salvo.platform.core.string.MutStr, lefts: List<String>, rights: List<String>, rows: List<Int>, width: Int) {
    for (i in salvo.platform.core.list.each(rows)) {
        val l: String = run {
            val __nn_1: String? = salvo.core.list.getPlatform(lefts, i)
            when {
                (__nn_1 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at args:843:17"))
                }
                else -> {
                    val __some_2: String = __nn_1!!
                    __some_2
                }
            }
        }
        val r: String = run {
            val __nn_3: String? = salvo.core.list.getPlatform(rights, i)
            when {
                (__nn_3 == null) -> {
                    throw AssertionError(("salvo: " + ("value is absent") + " at args:844:17"))
                }
                else -> {
                    val __some_4: String = __nn_3!!
                    __some_4
                }
            }
        }
        if (salvo.core.string.isEmpty(r)) {
            salvo.core.string.appendPlatform(out, "  ${l}\n")
        } else {
            salvo.core.string.appendPlatform(out, "  ${l}${salvo.core.string.repeat(" ", (width - salvo.core.string.sizePlatform(l)))}  ${r}\n")
        }
    }
}

fun section(spec: Spec, group: String?): String {
    if ((group == null)) {
        return ""
    }
    val group_1: String = group!!
    var top: String = group_1
    val cut: Pair<String, String>? = salvo.core.string.splitOnce(top, "/")
    if (!((cut == null))) {
        val cut_2: Pair<String, String> = cut!!
        val __destructured_3: Pair<String, String> = cut_2
        val head: String = __destructured_3.first
        val __unused_4: String = __destructured_3.second
        top = head
    }
    for (g in salvo.platform.core.list.each(spec.groups)) {
        if ((((g.key) == (top)) && !((g.heading == null)))) {
            val __narrowed_5: String = g.heading!!
            return salvo.core.string.trimSuffixPlatform(oneLine(__narrowed_5), ".")
        }
    }
    return ""
}

fun<T> parseArgs(args: List<String>, argSpec: () -> Spec, fromFound: (Found) -> Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>): Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>> {
    return parseNamed(args, null, argSpec, fromFound)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<T> parseNamed(args: List<String>, prefix: String?, argSpec: () -> Spec, fromFound: (Found) -> Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>): Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>> {
    val base: Spec = argSpec()
    checkSpec(base)
    val spec: Spec = (if ((prefix == null)) {
        base
    } else {
        val prefix_1: String = prefix!!
        run {
            val __spread_2: Spec = base
            Spec(name = "${prefix_1} ${base.name}", about = __spread_2.about, opts = __spread_2.opts, args = __spread_2.args, groups = __spread_2.groups, commands = __spread_2.commands, commandRequired = __spread_2.commandRequired, commandFields = __spread_2.commandFields)
        }
    })
    val found: Found = scan(spec, args)
    if (found.help) {
        val help: Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested> = Union11.U11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(HelpRequested(text = helpText(spec)))
        return Union2.U2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>(salvo.core.result.errs(help))
    }
    val r: Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>> = fromFound(found)
    if ((r is Union2.U2<*, *>)) {
        val r_3: List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>> = ((r as Union2.U2<*, *>).value as List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>)
        for (e in salvo.platform.core.list.each(r_3)) {
            val one: Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested> = e
            if ((one is Union11.U11<*, *, *, *, *, *, *, *, *, *, *>)) {
                val one_4: HelpRequested = ((one as Union11.U11<*, *, *, *, *, *, *, *, *, *, *>).value as HelpRequested)
                val help: Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested> = Union11.U11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(one_4)
                return Union2.U2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>(salvo.core.result.errs(help))
            }
        }
    }
    return r
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<T> argsOrExit(console: salvo.core.console.Console, process: salvo.process.Process, args: List<String>, argSpec: () -> Spec, fromFound: (Found) -> Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>): T {
    val parsed: Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>> = parseArgs(args, argSpec, fromFound)
    return when {
        (parsed is Union2.U1<*, *>) -> {
            val parsed_1: T = ((parsed as Union2.U1<*, *>).value as T)
            return parsed_1
        }
        (parsed is Union2.U2<*, *>) -> {
            val parsed_2: List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>> = ((parsed as Union2.U2<*, *>).value as List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>)
            exitWith(console, process, parsed_2, argSpec, fromFound)
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at args:977:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<T, V> parseValid(args: List<String>, validate: (T) -> V, argSpec: () -> Spec, fromFound: (Found) -> Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>): Union2<V, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>> {
    val parsed: Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>> = parseArgs(args, argSpec, fromFound)
    return when {
        (parsed is Union2.U2<*, *>) -> {
            val parsed_1: List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>> = ((parsed as Union2.U2<*, *>).value as List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>)
            return Union2.U2<V, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>(parsed_1)
        }
        (parsed is Union2.U1<*, *>) -> {
            val parsed_2: T = ((parsed as Union2.U1<*, *>).value as T)
            val checked: Union2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>> = try {
                Union2.U1<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(validate(parsed_2))
            } catch (__signal: ThrowSignal) {
                when (__signal.tag) {
                    "UnknownOption" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U1<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as UnknownOption))
                    "MissingValue" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U2<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as MissingValue))
                    "Missing" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U3<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as Missing))
                    "Invalid" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U4<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as Invalid))
                    "Unexpected" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U5<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as Unexpected))
                    "Repeated" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U6<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as Repeated))
                    "Requires" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U7<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as Requires))
                    "Conflict" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U8<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as Conflict))
                    "MissingOne" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U9<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as MissingOne))
                    "UnknownCommand" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U10<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as UnknownCommand))
                    "HelpRequested" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(Union11.U11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(__signal.payload as HelpRequested))
                    "UnknownOption | MissingValue | Missing | Invalid | Unexpected | Repeated | Requires | Conflict | MissingOne | UnknownCommand | HelpRequested" -> Union2.U2<V, Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>(__signal.payload as Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>)
                    else -> throw __signal
                }
            }
            when {
                (checked is Union2.U1<*, *>) -> {
                    val checked_3: V = ((checked as Union2.U1<*, *>).value as V)
                    return Union2.U1<V, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>(salvo.core.result.Ok(checked_3))
                }
                (checked is Union2.U2<*, *>) -> {
                    val checked_4: Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested> = ((checked as Union2.U2<*, *>).value as Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>)
                    val e: Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested> = checked_4
                    return Union2.U2<V, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>(salvo.core.result.errs(e))
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at args:1009:13"))
                }
            }
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at args:1003:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<T, V> validArgsOrExit(console: salvo.core.console.Console, process: salvo.process.Process, args: List<String>, validate: (T) -> V, argSpec: () -> Spec, fromFound: (Found) -> Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>): V {
    val parsed: Union2<V, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>> = parseValid(args, validate, argSpec, fromFound)
    return when {
        (parsed is Union2.U1<*, *>) -> {
            val parsed_1: V = ((parsed as Union2.U1<*, *>).value as V)
            return parsed_1
        }
        (parsed is Union2.U2<*, *>) -> {
            val parsed_2: List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>> = ((parsed as Union2.U2<*, *>).value as List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>)
            exitWith(console, process, parsed_2, argSpec, fromFound)
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at args:1025:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<T> exitWith(console: salvo.core.console.Console, process: salvo.process.Process, errors: List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>, argSpec: () -> Spec, fromFound: (Found) -> Union2<T, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>): Nothing {
    for (e in salvo.platform.core.list.each(errors)) {
        val one: Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested> = e
        if ((one is Union11.U11<*, *, *, *, *, *, *, *, *, *, *>)) {
            val one_1: HelpRequested = ((one as Union11.U11<*, *, *, *, *, *, *, *, *, *, *>).value as HelpRequested)
            console.print(one_1.text)
            process.exit(0)
        }
    }
    for (e in salvo.platform.core.list.each(errors)) {
        console.printerr("error: ${message(e)}\n")
    }
    val spec: Spec = argSpec()
    console.printerr("\n${usage(spec)}\n\nFor more information, try '--help'.\n")
    return process.exit(2)
}

fun checkSpec(spec: Spec) {
    val shorts: salvo.platform.core.list.MutList<Char> = mutableListOf<Char>()
    val fields: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    for (o in salvo.platform.core.list.each(spec.opts)) {
        if (salvo.core.list.contains(fields, o.field, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            throw AssertionError(("salvo: " + ("two options of `${spec.name}` are spelled `${longName(o.field)}`") + " at args:1061:13"))
        }
        salvo.core.list.addPlatform(fields, o.field)
        if (((o.field) == ("help"))) {
            throw AssertionError(("salvo: " + ("`--help` is the parser's own: name the field of `${spec.name}` something else") + " at args:1065:13"))
        }
        val s: Char? = o.short
        if (!((s == null))) {
            val s_1: Char = s!!
            if ((s_1 == 'h')) {
                throw AssertionError(("salvo: " + ("`Short('h')` on `${o.field}` takes help's short name") + " at args:1070:17"))
            }
            val s_2: Char = s!!
            if (salvo.core.list.contains(shorts, s_2, { __a0, __a1 -> ((__a0) == (__a1)) })) {
                throw AssertionError(("salvo: " + ("two options of `${spec.name}` have the short name `-${s_2}`") + " at args:1073:17"))
            }
            val s_3: Char = s!!
            salvo.core.list.addPlatform(shorts, s_3)
        }
    }
    for (g in salvo.platform.core.list.each(spec.groups)) {
        if (((g.kind) == ("arm"))) {
            var count: Int = 0
            var required: Boolean = false
            for (o in salvo.platform.core.list.each(spec.opts)) {
                if (inGroup(o.group, g.key)) {
                    count = (count + 1)
                    required = (required || o.required)
                }
            }
            if ((((count > 0) && !(required)) && !(((count == 1) && !(optTakesValue(spec, g.key)))))) {
                throw AssertionError(("salvo: " + ("every option of `${g.key}` is optional, so nothing chooses that arm: make one required") + " at args:1091:17"))
            }
        }
    }
    if ((!(salvo.core.list.isEmpty(spec.commands)) && !(salvo.core.list.isEmpty(spec.args)))) {
        throw AssertionError(("salvo: " + ("`${spec.name}` takes a sub-command and positionals: whether a word is one or the other would depend on its spelling") + " at args:1097:9"))
    }
    if ((spec.commandFields > 1)) {
        throw AssertionError(("salvo: " + ("`${spec.name}` has more than one `Subcommand` field") + " at args:1100:9"))
    }
    val names: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    for (c in salvo.platform.core.list.each(spec.commands)) {
        if (salvo.core.list.contains(names, c.name, { __a0, __a1 -> ((__a0) == (__a1)) })) {
            throw AssertionError(("salvo: " + ("two sub-commands of `${spec.name}` are named `${c.name}`") + " at args:1105:13"))
        }
        salvo.core.list.addPlatform(names, c.name)
    }
    var optional: Boolean = false
    var many: Boolean = false
    for (a in salvo.platform.core.list.each(spec.args)) {
        if (many) {
            throw AssertionError(("salvo: " + ("positional `${a.field}` follows one that takes every remaining word") + " at args:1113:13"))
        }
        if ((a.required && optional)) {
            throw AssertionError(("salvo: " + ("required positional `${a.field}` follows an optional one") + " at args:1116:13"))
        }
        optional = (optional || !(a.required))
        many = a.many
    }
}

fun optTakesValue(spec: Spec, key: String): Boolean {
    for (o in salvo.platform.core.list.each(spec.opts)) {
        if (inGroup(o.group, key)) {
            return o.takesValue
        }
    }
    return true
}

fun<Q> eval(f: () -> Q): Q {
    return f()
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<B, Q> readValue(found: Found, field: String, default: Q?, refine: (B) -> Q?, fromStr: (String) -> B?): Q {
    val s: String? = valueOf(found, field)
    if (!((s == null))) {
        val s_1: String = s!!
        val v: B? = fromStr(s_1)
        if (!((v == null))) {
            val v_2: B = v!!
            val q: Q? = refine(v_2)
            if (!((q == null))) {
                val q_3: Q = q!!
                return q_3
            }
        }
        throw AssertionError(("salvo: " + ("`${field}`'s value was checked") + " at args:1155:9"))
    }
    if (!((default == null))) {
        val default_4: Q = default!!
        return default_4
    }
    return throw AssertionError(("salvo: " + ("`${field}` was checked as required") + " at args:1160:5"))
}

fun<E> readOptional(found: Found, field: String, fromStr: (String) -> E?): E? {
    val s: String? = valueOf(found, field)
    if ((s == null)) {
        return null
    }
    val s_1: String = s!!
    return fromStr(s_1)
}

fun<E> readAll(found: Found, field: String, fromStr: (String) -> E?): List<E> {
    val out: salvo.platform.core.list.MutList<E> = mutableListOf<E>()
    for (s in salvo.platform.core.list.each(valuesOf(found, field))) {
        val v: E? = fromStr(s)
        if (!((v == null))) {
            val v_1: E = v!!
            salvo.core.list.addPlatform(out, v_1)
        }
    }
    return out
}

fun<B> checkValue(errors: salvo.platform.core.list.MutList<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>, found: Found, field: String, shown: String, what: String, test: (B) -> String?, fromStr: (String) -> B?) {
    for (s in salvo.platform.core.list.each(valuesOf(found, field))) {
        val v: B? = fromStr(s)
        if ((v == null)) {
            salvo.core.list.addPlatform(errors, Union11.U4<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Invalid(what = shown, value = s, reason = "expected ${what}")))
        } else {
            val v_1: B = v!!
            val why: String? = test(v_1)
            if (!((why == null))) {
                val why_2: String = why!!
                salvo.core.list.addPlatform(errors, Union11.U4<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>(Invalid(what = shown, value = s, reason = why_2)))
            }
        }
    }
}

fun rejected(): String? {
    return "the value is not accepted"
}

fun shownName(field: String, positional: Boolean): String {
    if (positional) {
        return argName(field)
    }
    return longName(field)
}

fun literalText(name: String): String {
    return salvo.core.string.trimSuffixPlatform(salvo.core.string.trimPrefixPlatform(name, "\""), "\"")
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<C> groupErrors(errors: salvo.platform.core.list.MutList<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>, r: Union2<C, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>, skip: Int) {
    if ((r is Union2.U2<*, *>)) {
        val r_1: List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>> = ((r as Union2.U2<*, *>).value as List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>)
        var i: Int = 0
        for (e in salvo.platform.core.list.each(r_1)) {
            if ((i >= skip)) {
                salvo.core.list.addPlatform(errors, e)
            }
            i = (i + 1)
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun<C> built(r: Union2<C, List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>>): C {
    return when {
        (r is Union2.U1<*, *>) -> {
            val r_1: C = ((r as Union2.U1<*, *>).value as C)
            return r_1
        }
        (r is Union2.U2<*, *>) -> {
            val r_2: List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>> = ((r as Union2.U2<*, *>).value as List<Union11<UnknownOption, MissingValue, Missing, Invalid, Unexpected, Repeated, Requires, Conflict, MissingOne, UnknownCommand, HelpRequested>>)
            throw AssertionError(("salvo: " + ("a group failed after its checks passed") + " at args:1241:19"))
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at args:1239:5"))
        }
    }
}

