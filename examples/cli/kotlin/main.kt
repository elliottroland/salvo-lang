package salvo.main

import salvo.*

data class Tool(
    val verbose: Boolean,
    val command: Union2<Serve, Fetch>?,
)

object __Codec_Tool : salvo.WireCodec<Tool> {
    override fun enc(v: Tool, out: salvo.WireOut) {
        salvo.BoolCodec.enc(v.verbose, out)
        salvo.OptCodec(salvo.Union2Codec(__Codec_Serve, __Codec_Fetch)).enc(v.command, out)
    }
    override fun dec(inp: salvo.WireIn): Tool = Tool(salvo.BoolCodec.dec(inp), salvo.OptCodec(salvo.Union2Codec(__Codec_Serve, __Codec_Fetch)).dec(inp))
}

data class Serve(
    val net: Net,
    val port: Int = 8080,
    val output: String = "text",
    val tls: Tls?,
    val files: List<String>,
)

object __Codec_Serve : salvo.WireCodec<Serve> {
    override fun enc(v: Serve, out: salvo.WireOut) {
        __Codec_Net.enc(v.net, out)
        salvo.IntCodec.enc(v.port, out)
        salvo.StrCodec.enc(v.output, out)
        salvo.OptCodec(__Codec_Tls).enc(v.tls, out)
        salvo.ListCodec(salvo.StrCodec).enc(v.files, out)
    }
    override fun dec(inp: salvo.WireIn): Serve = Serve(__Codec_Net.dec(inp), salvo.IntCodec.dec(inp), salvo.StrCodec.dec(inp), salvo.OptCodec(__Codec_Tls).dec(inp), salvo.ListCodec(salvo.StrCodec).dec(inp))
}

data class Net(
    val host: String = "127.0.0.1",
    val workers: Int?,
)

object __Codec_Net : salvo.WireCodec<Net> {
    override fun enc(v: Net, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.host, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.workers, out)
    }
    override fun dec(inp: salvo.WireIn): Net = Net(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp))
}

data class Tls(
    val cert: String,
    val key: String,
)

object __Codec_Tls : salvo.WireCodec<Tls> {
    override fun enc(v: Tls, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.cert, out)
        salvo.StrCodec.enc(v.key, out)
    }
    override fun dec(inp: salvo.WireIn): Tls = Tls(salvo.StrCodec.dec(inp), salvo.StrCodec.dec(inp))
}

data class Fetch(
    val source: Union3<FromUrl, FromFile, FromStdin>,
    val header: List<String>,
)

object __Codec_Fetch : salvo.WireCodec<Fetch> {
    override fun enc(v: Fetch, out: salvo.WireOut) {
        salvo.Union3Codec(__Codec_FromUrl, __Codec_FromFile, __Codec_FromStdin).enc(v.source, out)
        salvo.ListCodec(salvo.StrCodec).enc(v.header, out)
    }
    override fun dec(inp: salvo.WireIn): Fetch = Fetch(salvo.Union3Codec(__Codec_FromUrl, __Codec_FromFile, __Codec_FromStdin).dec(inp), salvo.ListCodec(salvo.StrCodec).dec(inp))
}

data class FromUrl(
    val url: String,
    val timeout: Int = 30,
)

object __Codec_FromUrl : salvo.WireCodec<FromUrl> {
    override fun enc(v: FromUrl, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.url, out)
        salvo.IntCodec.enc(v.timeout, out)
    }
    override fun dec(inp: salvo.WireIn): FromUrl = FromUrl(salvo.StrCodec.dec(inp), salvo.IntCodec.dec(inp))
}

data class FromFile(
    val file: String,
)

object __Codec_FromFile : salvo.WireCodec<FromFile> {
    override fun enc(v: FromFile, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.file, out)
    }
    override fun dec(inp: salvo.WireIn): FromFile = FromFile(salvo.StrCodec.dec(inp))
}

class FromStdin

object __Codec_FromStdin : salvo.WireCodec<FromStdin> {
    override fun enc(v: FromStdin, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): FromStdin = FromStdin()
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun describe(t: Tool): String {
    val c: Union2<Serve, Fetch>? = t.command
    return when {
        (c is Union2.U1<*, *>) -> {
            val c_1: Serve = ((c as Union2.U1<*, *>).value as Serve)
            val tls: Tls? = c_1.tls
            val secure: String = (if ((tls == null)) {
                "plain"
            } else {
                val tls_2: Tls = tls!!
                "tls ${tls_2.cert}"
            })
            return "serve ${salvo.core.string.joinPlatform(c_1.files, ",")} on ${c_1.net.host}:${c_1.port} as ${c_1.output}, ${secure}, ${run {
                val __pick_3: Int? = c_1.net.workers
                (if ((__pick_3 != null)) {
                    val __picked_4: Int = __pick_3!!
                    __picked_4
                } else {
                    1
                })
            }} worker(s)"
        }
        (c is Union2.U2<*, *>) -> {
            val c_6: Fetch = ((c as Union2.U2<*, *>).value as Fetch)
            val s: Union3<FromUrl, FromFile, FromStdin> = c_6.source
            val from: String = when {
                (s is Union3.U1<*, *, *>) -> {
                    val s_7: FromUrl = ((s as Union3.U1<*, *, *>).value as FromUrl)
                    "${s_7.url} (timeout ${s_7.timeout}s)"
                }
                (s is Union3.U2<*, *, *>) -> {
                    val s_8: FromFile = ((s as Union3.U2<*, *, *>).value as FromFile)
                    "file ${s_8.file}"
                }
                (s is Union3.U3<*, *, *>) -> {
                    val s_9: FromStdin = ((s as Union3.U3<*, *, *>).value as FromStdin)
                    "standard input"
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:104:24"))
                }
            }
            return "fetch ${from} with ${salvo.core.list.sizePlatform(c_6.header)} header(s)"
        }
        (c == null) -> {
            return "nothing to do"
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:96:5"))
        }
    }
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun show(console: salvo.core.console.Console, line: List<String>) {
    salvo.core.console.println(console, "\$ tool ${salvo.core.string.joinPlatform(line, " ")}")
    val r: Union2<Tool, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> = salvo.args.parseArgs(line, ::argSpec__SpecTool, ::fromFound__Found_OkToolOrErrsNonEmptyListArgError)
    when {
        (r is Union2.U1<*, *>) -> {
            val r_1: Tool = ((r as Union2.U1<*, *>).value as Tool)
            val loud: String = (if (r_1.verbose) {
                " (verbose)"
            } else {
                ""
            })
            salvo.core.console.println(console, "  ${describe(r_1)}${loud}")
        }
        (r is Union2.U2<*, *>) -> {
            val r_2: List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>> = ((r as Union2.U2<*, *>).value as List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>)
            for (e in salvo.platform.core.list.each(r_2)) {
                val one: Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested> = e
                if ((one is Union11.U11<*, *, *, *, *, *, *, *, *, *, *>)) {
                    val one_3: salvo.args.HelpRequested = ((one as Union11.U11<*, *, *, *, *, *, *, *, *, *, *>).value as salvo.args.HelpRequested)
                    console.print(one_3.text)
                } else {
                    val one_4: Union10<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand> = one.let { when (it) { is Union11.U1<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U1<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.UnknownOption); is Union11.U2<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U2<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.MissingValue); is Union11.U3<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U3<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.Missing); is Union11.U4<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U4<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.Invalid); is Union11.U5<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U5<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.Unexpected); is Union11.U6<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U6<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.Repeated); is Union11.U7<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U7<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.Requires); is Union11.U8<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U8<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.Conflict); is Union11.U9<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U9<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.MissingOne); is Union11.U10<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U10<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand>(it.value as salvo.args.UnknownCommand); else -> throw IllegalStateException("salvo: unreachable union arm") } }
                    salvo.core.console.println(console, "  error: ${salvo.args.message(one_4.let { when (it) { is Union10.U1<*, *, *, *, *, *, *, *, *, *> -> Union11.U1<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.UnknownOption); is Union10.U2<*, *, *, *, *, *, *, *, *, *> -> Union11.U2<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.MissingValue); is Union10.U3<*, *, *, *, *, *, *, *, *, *> -> Union11.U3<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.Missing); is Union10.U4<*, *, *, *, *, *, *, *, *, *> -> Union11.U4<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.Invalid); is Union10.U5<*, *, *, *, *, *, *, *, *, *> -> Union11.U5<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.Unexpected); is Union10.U6<*, *, *, *, *, *, *, *, *, *> -> Union11.U6<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.Repeated); is Union10.U7<*, *, *, *, *, *, *, *, *, *> -> Union11.U7<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.Requires); is Union10.U8<*, *, *, *, *, *, *, *, *, *> -> Union11.U8<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.Conflict); is Union10.U9<*, *, *, *, *, *, *, *, *, *> -> Union11.U9<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.MissingOne); is Union10.U10<*, *, *, *, *, *, *, *, *, *> -> Union11.U10<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>(it.value as salvo.args.UnknownCommand); else -> throw IllegalStateException("salvo: unreachable union arm") } })}")
                }
            }
        }
        else -> {
            throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:120:5"))
        }
    }
}

fun main(args: Array<String>) {
    salvo.SalvoArgs.set(args)
    val __use_1: salvo.core.console.__Platform_StdConsole = salvo.core.console.__Platform_StdConsole()
    val __handle_2: salvo.core.console.Console = __use_1
    val __use_3: salvo.args.__Platform_HostArgs = salvo.args.__Platform_HostArgs()
    val __handle_4: salvo.args.Args = __use_3
    val __use_5: salvo.process.__Platform_HostProcess = salvo.process.__Platform_HostProcess()
    val __handle_6: salvo.process.Process = __use_5
    show(__handle_2, listOf<String>("serve", "-p", "80", "index.html", "app.js"))
    show(__handle_2, listOf<String>("-v", "serve", "--host=0.0.0.0", "--workers", "4", "--cert", "c.pem", "--key", "k.pem", "--output", "json", "site"))
    show(__handle_2, listOf<String>("fetch", "--url", "https://example.com", "--header", "Accept: text/html", "--header", "X-Id: 7"))
    show(__handle_2, listOf<String>("fetch", "--from-stdin"))
    show(__handle_2, listOf<String>("serve", "-p", "0", "--output", "xml", "--cert", "c.pem"))
    show(__handle_2, listOf<String>("fetch", "--url", "u", "--file", "f"))
    show(__handle_2, listOf<String>("deploy"))
    show(__handle_2, listOf<String>("--help"))
    show(__handle_2, listOf<String>("serve", "--help"))
    val tool: Tool = salvo.args.argsOrExit(__handle_2, __handle_6, __handle_4.args(), ::argSpec__SpecTool, ::fromFound__Found_OkToolOrErrsNonEmptyListArgError)
    salvo.core.console.println(__handle_2, "argv: ${describe(tool)}")
}

fun argSpec__SpecTool(): salvo.args.Spec {
    val opts: salvo.platform.core.list.MutList<salvo.args.OptSpec> = mutableListOf<salvo.args.OptSpec>()
    val args: salvo.platform.core.list.MutList<salvo.args.ArgSpec> = mutableListOf<salvo.args.ArgSpec>()
    val groups: salvo.platform.core.list.MutList<salvo.args.GroupSpec> = mutableListOf<salvo.args.GroupSpec>()
    val commands: salvo.platform.core.list.MutList<salvo.args.CmdSpec> = mutableListOf<salvo.args.CmdSpec>()
    val commandRequired: Boolean = false
    var commandFields: Int = 0
    val required__c1__c2__c3__c4: Boolean = false
    val many__c1__c2__c3__c4: Boolean = false
    var switch__c1__c2__c3__c4: Boolean = false
    val defaultText__c1__c2__c3__c4: String? = null
    val choices__c1__c2__c3__c4: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c1__c2__c3__c4: Boolean = false
    switch__c1__c2__c3__c4 = true
    if (exclusive__c1__c2__c3__c4) {
        salvo.args.addOne(groups, "verbose", required__c1__c2__c3__c4, "Print more.")
    }
    var short__c1__c2__c3__c4: Char? = null
    short__c1__c2__c3__c4 = 'v'
    val short__c1__c2__c3__c4_1: Char = short__c1__c2__c3__c4!!
    val opt__c1__c2__c3__c4: salvo.args.OptSpec = salvo.args.OptSpec(field = "verbose", short = short__c1__c2__c3__c4_1, takesValue = !(switch__c1__c2__c3__c4), repeats = many__c1__c2__c3__c4, required = (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help = "Print more.", defaultText = defaultText__c1__c2__c3__c4, choices = choices__c1__c2__c3__c4, group = null)
    if ((!(exclusive__c1__c2__c3__c4) || !(salvo.core.list.isEmpty(opt__c1__c2__c3__c4.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c1__c2__c3__c4)
    }
    commandFields = (commandFields + 1)
    val child__c8__c9__c11__c12__c13: salvo.args.Spec = argSpec__SpecServe()
    salvo.core.list.addPlatform(commands, salvo.args.CmdSpec(name = child__c8__c9__c11__c12__c13.name, about = child__c8__c9__c11__c12__c13.about))
    val child__c8__c9__c11__c14__c15: salvo.args.Spec = argSpec__SpecFetch()
    salvo.core.list.addPlatform(commands, salvo.args.CmdSpec(name = child__c8__c9__c11__c14__c15.name, about = child__c8__c9__c11__c14__c15.about))
    val name: String = salvo.args.commandName("Tool")
    return salvo.args.Spec(name = name, about = "A toy file server.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkToolOrErrsNonEmptyListArgError(found: salvo.args.Found): Union2<Tool, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> {
    val errors: salvo.platform.core.list.MutList<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>> = mutableListOf<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>()
    for (e in salvo.platform.core.list.each(found.errors)) {
        salvo.core.list.addPlatform(errors, e)
    }
    val s__c8__c9__c10__c11__c12: salvo.args.Spec = argSpec__SpecServe()
    if (salvo.args.isCommand(found, s__c8__c9__c10__c11__c12.name)) {
        val r__c8__c9__c10__c11__c12: Union2<Serve, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> = salvo.args.parseNamed(found.rest, found.path, ::argSpec__SpecServe, ::fromFound__Found_OkServeOrErrsNonEmptyListArgError)
        salvo.args.groupErrors(errors, r__c8__c9__c10__c11__c12, 0)
    }
    val s__c8__c9__c10__c13__c14: salvo.args.Spec = argSpec__SpecFetch()
    if (salvo.args.isCommand(found, s__c8__c9__c10__c13__c14.name)) {
        val r__c8__c9__c10__c13__c14: Union2<Fetch, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> = salvo.args.parseNamed(found.rest, found.path, ::argSpec__SpecFetch, ::fromFound__Found_OkFetchOrErrsNonEmptyListArgError)
        salvo.args.groupErrors(errors, r__c8__c9__c10__c13__c14, 0)
    }
    if (salvo.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Tool, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Errs(errors))
    }
    return Union2.U1<Tool, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Ok(Tool(verbose = salvo.args.eval(fun(): Boolean {
        return salvo.args.given(found, "verbose")
    }), command = salvo.args.eval(fun(): Union2<Serve, Fetch>? {
        val s__c24__c25__c26__c27__c28: salvo.args.Spec = argSpec__SpecServe()
        if (salvo.args.isCommand(found, s__c24__c25__c26__c27__c28.name)) {
            return Union2.U1<Serve, Fetch>(salvo.args.built(salvo.args.parseNamed(found.rest, found.path, ::argSpec__SpecServe, ::fromFound__Found_OkServeOrErrsNonEmptyListArgError)))
        }
        val s__c24__c25__c26__c29__c30: salvo.args.Spec = argSpec__SpecFetch()
        if (salvo.args.isCommand(found, s__c24__c25__c26__c29__c30.name)) {
            return Union2.U2<Serve, Fetch>(salvo.args.built(salvo.args.parseNamed(found.rest, found.path, ::argSpec__SpecFetch, ::fromFound__Found_OkFetchOrErrsNonEmptyListArgError)))
        }
        return null
    }))))
}

fun argSpec__SpecServe(): salvo.args.Spec {
    val opts: salvo.platform.core.list.MutList<salvo.args.OptSpec> = mutableListOf<salvo.args.OptSpec>()
    val args: salvo.platform.core.list.MutList<salvo.args.ArgSpec> = mutableListOf<salvo.args.ArgSpec>()
    val groups: salvo.platform.core.list.MutList<salvo.args.GroupSpec> = mutableListOf<salvo.args.GroupSpec>()
    val commands: salvo.platform.core.list.MutList<salvo.args.CmdSpec> = mutableListOf<salvo.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    val child__c1__c2__c3__c4: salvo.args.Spec = argSpec__SpecNet()
    salvo.args.mergeGroup(opts, groups, child__c1__c2__c3__c4, "net", "flat", null)
    val required__c5__c6__c7__c8: Boolean = false
    val many__c5__c6__c7__c8: Boolean = false
    val switch__c5__c6__c7__c8: Boolean = false
    var defaultText__c5__c6__c7__c8: String? = null
    val choices__c5__c6__c7__c8: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c5__c6__c7__c8: Boolean = false
    defaultText__c5__c6__c7__c8 = "${run {
        val __default_1: Int = 8080
        (if (!(salvo.core.range.InRange_qualifies(__default_1, 1, 65535))) throw AssertionError(("salvo: " + ("the default for field `port` is not `InRange(1, 65535) Int`") + " at main:157:1")) else Unit)
        __default_1
    }}"
    if (exclusive__c5__c6__c7__c8) {
        salvo.args.addOne(groups, "port", required__c5__c6__c7__c8, "Port to listen on.")
    }
    var short__c5__c6__c7__c8: Char? = null
    short__c5__c6__c7__c8 = 'p'
    val short__c5__c6__c7__c8_2: Char = short__c5__c6__c7__c8!!
    val defaultText__c5__c6__c7__c8_3: String = defaultText__c5__c6__c7__c8!!
    val opt__c5__c6__c7__c8: salvo.args.OptSpec = salvo.args.OptSpec(field = "port", short = short__c5__c6__c7__c8_2, takesValue = !(switch__c5__c6__c7__c8), repeats = many__c5__c6__c7__c8, required = (required__c5__c6__c7__c8 && !(exclusive__c5__c6__c7__c8)), help = "Port to listen on.", defaultText = defaultText__c5__c6__c7__c8_3, choices = choices__c5__c6__c7__c8, group = null)
    if ((!(exclusive__c5__c6__c7__c8) || !(salvo.core.list.isEmpty(opt__c5__c6__c7__c8.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c5__c6__c7__c8)
    }
    val required__c16__c17__c18__c19: Boolean = false
    val many__c16__c17__c18__c19: Boolean = false
    val switch__c16__c17__c18__c19: Boolean = false
    var defaultText__c16__c17__c18__c19: String? = null
    val choices__c16__c17__c18__c19: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c16__c17__c18__c19: Boolean = false
    defaultText__c16__c17__c18__c19 = "${"text"}"
    salvo.core.list.addPlatform(choices__c16__c17__c18__c19, salvo.args.literalText("\"json\""))
    salvo.core.list.addPlatform(choices__c16__c17__c18__c19, salvo.args.literalText("\"text\""))
    if (exclusive__c16__c17__c18__c19) {
        salvo.args.addOne(groups, "output", required__c16__c17__c18__c19, "Output format.")
    }
    val short__c16__c17__c18__c19: Char? = null
    val defaultText__c16__c17__c18__c19_4: String = defaultText__c16__c17__c18__c19!!
    val opt__c16__c17__c18__c19: salvo.args.OptSpec = salvo.args.OptSpec(field = "output", short = short__c16__c17__c18__c19, takesValue = !(switch__c16__c17__c18__c19), repeats = many__c16__c17__c18__c19, required = (required__c16__c17__c18__c19 && !(exclusive__c16__c17__c18__c19)), help = "Output format.", defaultText = defaultText__c16__c17__c18__c19_4, choices = choices__c16__c17__c18__c19, group = null)
    if ((!(exclusive__c16__c17__c18__c19) || !(salvo.core.list.isEmpty(opt__c16__c17__c18__c19.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c16__c17__c18__c19)
    }
    val child__c33__c34__c35: salvo.args.Spec = argSpec__SpecTls()
    salvo.args.mergeGroup(opts, groups, child__c33__c34__c35, "tls", "together", "TLS (give both or neither).")
    var required__c36__c37__c38__c39: Boolean = false
    var many__c36__c37__c38__c39: Boolean = false
    val switch__c36__c37__c38__c39: Boolean = false
    val defaultText__c36__c37__c38__c39: String? = null
    val choices__c36__c37__c38__c39: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c36__c37__c38__c39: Boolean = false
    many__c36__c37__c38__c39 = true
    required__c36__c37__c38__c39 = true
    if (exclusive__c36__c37__c38__c39) {
        salvo.args.addOne(groups, "files", required__c36__c37__c38__c39, "Files to serve.")
    }
    val short__c36__c37__c38__c39: Char? = null
    val opt__c36__c37__c38__c39: salvo.args.OptSpec = salvo.args.OptSpec(field = "files", short = short__c36__c37__c38__c39, takesValue = !(switch__c36__c37__c38__c39), repeats = many__c36__c37__c38__c39, required = (required__c36__c37__c38__c39 && !(exclusive__c36__c37__c38__c39)), help = "Files to serve.", defaultText = defaultText__c36__c37__c38__c39, choices = choices__c36__c37__c38__c39, group = null)
    if (exclusive__c36__c37__c38__c39) {
        throw AssertionError(("salvo: " + ("positional `files` is a choice of option groups") + " at main:157:1"))
    }
    salvo.core.list.addPlatform(args, salvo.args.ArgSpec(field = opt__c36__c37__c38__c39.field, help = opt__c36__c37__c38__c39.help, required = opt__c36__c37__c38__c39.required, many = opt__c36__c37__c38__c39.repeats))
    val name: String = salvo.args.commandName("Serve")
    return salvo.args.Spec(name = name, about = "Serve files over HTTP.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun fromFound__Found_OkServeOrErrsNonEmptyListArgError(found: salvo.args.Found): Union2<Serve, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> {
    val errors: salvo.platform.core.list.MutList<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>> = mutableListOf<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>()
    for (e in salvo.platform.core.list.each(found.errors)) {
        salvo.core.list.addPlatform(errors, e)
    }
    val r__c1__c2__c3__c4: Union2<Net, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> = fromFound__Found_OkNetOrErrsNonEmptyListArgError(found)
    salvo.args.groupErrors(errors, r__c1__c2__c3__c4, salvo.core.list.sizePlatform(found.errors))
    val positional__c5__c6__c7__c8__c9__c10__c11: Boolean = false
    val shown__c5__c6__c7__c8__c9__c10__c11: String = salvo.args.shownName("port", positional__c5__c6__c7__c8__c9__c10__c11)
    var what__c5__c6__c7__c8__c9__c10__c11: String = "a valid value"
    what__c5__c6__c7__c8__c9__c10__c11 = "an integer"
    salvo.args.checkValue(errors, found, "port", shown__c5__c6__c7__c8__c9__c10__c11, what__c5__c6__c7__c8__c9__c10__c11, fun(v: Int): String? {
        return (if (!(salvo.core.range.InRange_qualifies(v, 1, 65535))) {
            "${v} is not in 1..65535"
        } else {
            null
        })
    }, { __a0 -> salvo.core.string.fromStr__Str_IntOpt(__a0) })
    val s__c25__c26__c27: salvo.args.Spec = argSpec__SpecTls()
    if (salvo.args.anyGiven(found, s__c25__c26__c27)) {
        val r__c25__c26__c27: Union2<Tls, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> = fromFound__Found_OkTlsOrErrsNonEmptyListArgError(found)
        salvo.args.groupErrors(errors, r__c25__c26__c27, salvo.core.list.sizePlatform(found.errors))
    }
    var positional__c28__c29__c30__c31__c32__c33__c34: Boolean = false
    positional__c28__c29__c30__c31__c32__c33__c34 = true
    val shown__c28__c29__c30__c31__c32__c33__c34: String = salvo.args.shownName("files", positional__c28__c29__c30__c31__c32__c33__c34)
    val what__c28__c29__c30__c31__c32__c33__c34: String = "a valid value"
    salvo.args.checkValue(errors, found, "files", shown__c28__c29__c30__c31__c32__c33__c34, what__c28__c29__c30__c31__c32__c33__c34, fun(_v: String): String? {
        return null
    }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    if (salvo.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Serve, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Errs(errors))
    }
    return Union2.U1<Serve, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Ok(Serve(net = salvo.args.eval(fun(): Net {
        return salvo.args.built(fromFound__Found_OkNetOrErrsNonEmptyListArgError(found))
    }), port = salvo.args.eval(fun(): Int {
        return salvo.args.readValue(found, "port", run {
            val __default_1: Int = 8080
            (if (!(salvo.core.range.InRange_qualifies(__default_1, 1, 65535))) throw AssertionError(("salvo: " + ("the default for field `port` is not `InRange(1, 65535) Int`") + " at main:157:1")) else Unit)
            __default_1
        }, fun(v__c41__c42__c43__c44__c45__c46__c47__c48: Int): Int? {
            return (if (salvo.core.range.InRange_qualifies(v__c41__c42__c43__c44__c45__c46__c47__c48, 1, 65535)) {
                v__c41__c42__c43__c44__c45__c46__c47__c48
            } else {
                null
            })
        }, { __a0 -> salvo.core.string.fromStr__Str_IntOpt(__a0) })
    }), output = salvo.args.eval(fun(): String {
        val v__c52__c53__c54__c55__c56__c57: String? = salvo.args.valueOf(found, "output")
        if (!((v__c52__c53__c54__c55__c56__c57 == null))) {
            if ((v__c52__c53__c54__c55__c56__c57 != null && (v__c52__c53__c54__c55__c56__c57 == "json"))) {
                val v__c52__c53__c54__c55__c56__c57_2: String = v__c52__c53__c54__c55__c56__c57!!
                return v__c52__c53__c54__c55__c56__c57_2
            }
            if ((v__c52__c53__c54__c55__c56__c57 != null && (v__c52__c53__c54__c55__c56__c57 == "text"))) {
                val v__c52__c53__c54__c55__c56__c57_3: String = v__c52__c53__c54__c55__c56__c57!!
                return v__c52__c53__c54__c55__c56__c57_3
            }
        }
        return "text"
    }), tls = salvo.args.eval(fun(): Tls? {
        val s__c65__c66__c67: salvo.args.Spec = argSpec__SpecTls()
        if (salvo.args.anyGiven(found, s__c65__c66__c67)) {
            return salvo.args.built(fromFound__Found_OkTlsOrErrsNonEmptyListArgError(found))
        }
        return null
    }), files = salvo.args.eval(fun(): List<String> {
        val all__c68__c69__c70__c71__c72__c73__c74__c75: List<String> = salvo.args.readAll(found, "files", { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
        if (salvo.core.list.NonEmpty_qualifies(all__c68__c69__c70__c71__c72__c73__c74__c75)) {
            return all__c68__c69__c70__c71__c72__c73__c74__c75
        }
        return throw AssertionError(("salvo: " + ("`files` was checked as required") + " at main:157:1"))
    }))))
}

fun argSpec__SpecNet(): salvo.args.Spec {
    val opts: salvo.platform.core.list.MutList<salvo.args.OptSpec> = mutableListOf<salvo.args.OptSpec>()
    val args: salvo.platform.core.list.MutList<salvo.args.ArgSpec> = mutableListOf<salvo.args.ArgSpec>()
    val groups: salvo.platform.core.list.MutList<salvo.args.GroupSpec> = mutableListOf<salvo.args.GroupSpec>()
    val commands: salvo.platform.core.list.MutList<salvo.args.CmdSpec> = mutableListOf<salvo.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    val required__c1__c2__c3__c4: Boolean = false
    val many__c1__c2__c3__c4: Boolean = false
    val switch__c1__c2__c3__c4: Boolean = false
    var defaultText__c1__c2__c3__c4: String? = null
    val choices__c1__c2__c3__c4: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c1__c2__c3__c4: Boolean = false
    defaultText__c1__c2__c3__c4 = "127.0.0.1"
    if (exclusive__c1__c2__c3__c4) {
        salvo.args.addOne(groups, "host", required__c1__c2__c3__c4, "Address to bind.")
    }
    val short__c1__c2__c3__c4: Char? = null
    val defaultText__c1__c2__c3__c4_1: String = defaultText__c1__c2__c3__c4!!
    val opt__c1__c2__c3__c4: salvo.args.OptSpec = salvo.args.OptSpec(field = "host", short = short__c1__c2__c3__c4, takesValue = !(switch__c1__c2__c3__c4), repeats = many__c1__c2__c3__c4, required = (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help = "Address to bind.", defaultText = defaultText__c1__c2__c3__c4_1, choices = choices__c1__c2__c3__c4, group = null)
    if ((!(exclusive__c1__c2__c3__c4) || !(salvo.core.list.isEmpty(opt__c1__c2__c3__c4.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c1__c2__c3__c4)
    }
    val required__c11__c12__c13__c14: Boolean = false
    val many__c11__c12__c13__c14: Boolean = false
    val switch__c11__c12__c13__c14: Boolean = false
    val defaultText__c11__c12__c13__c14: String? = null
    val choices__c11__c12__c13__c14: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c11__c12__c13__c14: Boolean = false
    if (exclusive__c11__c12__c13__c14) {
        salvo.args.addOne(groups, "workers", required__c11__c12__c13__c14, "Connections served at once.")
    }
    val short__c11__c12__c13__c14: Char? = null
    val opt__c11__c12__c13__c14: salvo.args.OptSpec = salvo.args.OptSpec(field = "workers", short = short__c11__c12__c13__c14, takesValue = !(switch__c11__c12__c13__c14), repeats = many__c11__c12__c13__c14, required = (required__c11__c12__c13__c14 && !(exclusive__c11__c12__c13__c14)), help = "Connections served at once.", defaultText = defaultText__c11__c12__c13__c14, choices = choices__c11__c12__c13__c14, group = null)
    if ((!(exclusive__c11__c12__c13__c14) || !(salvo.core.list.isEmpty(opt__c11__c12__c13__c14.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c11__c12__c13__c14)
    }
    val name: String = salvo.args.commandName("Net")
    return salvo.args.Spec(name = name, about = "Network options.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkNetOrErrsNonEmptyListArgError(found: salvo.args.Found): Union2<Net, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> {
    val errors: salvo.platform.core.list.MutList<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>> = mutableListOf<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>()
    for (e in salvo.platform.core.list.each(found.errors)) {
        salvo.core.list.addPlatform(errors, e)
    }
    val positional__c1__c2__c3__c4__c5__c6__c7: Boolean = false
    val shown__c1__c2__c3__c4__c5__c6__c7: String = salvo.args.shownName("host", positional__c1__c2__c3__c4__c5__c6__c7)
    val what__c1__c2__c3__c4__c5__c6__c7: String = "a valid value"
    salvo.args.checkValue(errors, found, "host", shown__c1__c2__c3__c4__c5__c6__c7, what__c1__c2__c3__c4__c5__c6__c7, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    val positional__c11__c12__c13__c14__c15__c16__c17: Boolean = false
    val shown__c11__c12__c13__c14__c15__c16__c17: String = salvo.args.shownName("workers", positional__c11__c12__c13__c14__c15__c16__c17)
    var what__c11__c12__c13__c14__c15__c16__c17: String = "a valid value"
    what__c11__c12__c13__c14__c15__c16__c17 = "an integer"
    salvo.args.checkValue(errors, found, "workers", shown__c11__c12__c13__c14__c15__c16__c17, what__c11__c12__c13__c14__c15__c16__c17, fun(_v: Int): String? {
        return null
    }, { __a0 -> salvo.core.string.fromStr__Str_IntOpt(__a0) })
    if (salvo.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Net, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Errs(errors))
    }
    return Union2.U1<Net, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Ok(Net(host = salvo.args.eval(fun(): String {
        return salvo.args.readValue(found, "host", "127.0.0.1", fun(v__c21__c22__c23__c24__c25__c26__c27__c28: String): String? {
            return (if (true) {
                v__c21__c22__c23__c24__c25__c26__c27__c28
            } else {
                null
            })
        }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    }), workers = salvo.args.eval(fun(): Int? {
        return salvo.args.readOptional(found, "workers", { __a0 -> salvo.core.string.fromStr__Str_IntOpt(__a0) })
    }))))
}

fun argSpec__SpecTls(): salvo.args.Spec {
    val opts: salvo.platform.core.list.MutList<salvo.args.OptSpec> = mutableListOf<salvo.args.OptSpec>()
    val args: salvo.platform.core.list.MutList<salvo.args.ArgSpec> = mutableListOf<salvo.args.ArgSpec>()
    val groups: salvo.platform.core.list.MutList<salvo.args.GroupSpec> = mutableListOf<salvo.args.GroupSpec>()
    val commands: salvo.platform.core.list.MutList<salvo.args.CmdSpec> = mutableListOf<salvo.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    var required__c1__c2__c3__c4: Boolean = false
    val many__c1__c2__c3__c4: Boolean = false
    val switch__c1__c2__c3__c4: Boolean = false
    val defaultText__c1__c2__c3__c4: String? = null
    val choices__c1__c2__c3__c4: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c1__c2__c3__c4: Boolean = false
    required__c1__c2__c3__c4 = true
    if (exclusive__c1__c2__c3__c4) {
        salvo.args.addOne(groups, "cert", required__c1__c2__c3__c4, "Certificate file.")
    }
    val short__c1__c2__c3__c4: Char? = null
    val opt__c1__c2__c3__c4: salvo.args.OptSpec = salvo.args.OptSpec(field = "cert", short = short__c1__c2__c3__c4, takesValue = !(switch__c1__c2__c3__c4), repeats = many__c1__c2__c3__c4, required = (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help = "Certificate file.", defaultText = defaultText__c1__c2__c3__c4, choices = choices__c1__c2__c3__c4, group = null)
    if ((!(exclusive__c1__c2__c3__c4) || !(salvo.core.list.isEmpty(opt__c1__c2__c3__c4.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c1__c2__c3__c4)
    }
    var required__c11__c12__c13__c14: Boolean = false
    val many__c11__c12__c13__c14: Boolean = false
    val switch__c11__c12__c13__c14: Boolean = false
    val defaultText__c11__c12__c13__c14: String? = null
    val choices__c11__c12__c13__c14: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c11__c12__c13__c14: Boolean = false
    required__c11__c12__c13__c14 = true
    if (exclusive__c11__c12__c13__c14) {
        salvo.args.addOne(groups, "key", required__c11__c12__c13__c14, "Private key file.")
    }
    val short__c11__c12__c13__c14: Char? = null
    val opt__c11__c12__c13__c14: salvo.args.OptSpec = salvo.args.OptSpec(field = "key", short = short__c11__c12__c13__c14, takesValue = !(switch__c11__c12__c13__c14), repeats = many__c11__c12__c13__c14, required = (required__c11__c12__c13__c14 && !(exclusive__c11__c12__c13__c14)), help = "Private key file.", defaultText = defaultText__c11__c12__c13__c14, choices = choices__c11__c12__c13__c14, group = null)
    if ((!(exclusive__c11__c12__c13__c14) || !(salvo.core.list.isEmpty(opt__c11__c12__c13__c14.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c11__c12__c13__c14)
    }
    val name: String = salvo.args.commandName("Tls")
    return salvo.args.Spec(name = name, about = null, opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkTlsOrErrsNonEmptyListArgError(found: salvo.args.Found): Union2<Tls, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> {
    val errors: salvo.platform.core.list.MutList<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>> = mutableListOf<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>()
    for (e in salvo.platform.core.list.each(found.errors)) {
        salvo.core.list.addPlatform(errors, e)
    }
    val positional__c1__c2__c3__c4__c5__c6__c7: Boolean = false
    val shown__c1__c2__c3__c4__c5__c6__c7: String = salvo.args.shownName("cert", positional__c1__c2__c3__c4__c5__c6__c7)
    val what__c1__c2__c3__c4__c5__c6__c7: String = "a valid value"
    salvo.args.checkValue(errors, found, "cert", shown__c1__c2__c3__c4__c5__c6__c7, what__c1__c2__c3__c4__c5__c6__c7, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    val positional__c11__c12__c13__c14__c15__c16__c17: Boolean = false
    val shown__c11__c12__c13__c14__c15__c16__c17: String = salvo.args.shownName("key", positional__c11__c12__c13__c14__c15__c16__c17)
    val what__c11__c12__c13__c14__c15__c16__c17: String = "a valid value"
    salvo.args.checkValue(errors, found, "key", shown__c11__c12__c13__c14__c15__c16__c17, what__c11__c12__c13__c14__c15__c16__c17, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    if (salvo.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Tls, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Errs(errors))
    }
    return Union2.U1<Tls, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Ok(Tls(cert = salvo.args.eval(fun(): String {
        return salvo.args.readValue(found, "cert", null, fun(v__c21__c22__c23__c24__c25__c26__c27__c28: String): String? {
            return (if (true) {
                v__c21__c22__c23__c24__c25__c26__c27__c28
            } else {
                null
            })
        }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    }), key = salvo.args.eval(fun(): String {
        return salvo.args.readValue(found, "key", null, fun(v__c31__c32__c33__c34__c35__c36__c37__c38: String): String? {
            return (if (true) {
                v__c31__c32__c33__c34__c35__c36__c37__c38
            } else {
                null
            })
        }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    }))))
}

fun argSpec__SpecFetch(): salvo.args.Spec {
    val opts: salvo.platform.core.list.MutList<salvo.args.OptSpec> = mutableListOf<salvo.args.OptSpec>()
    val args: salvo.platform.core.list.MutList<salvo.args.ArgSpec> = mutableListOf<salvo.args.ArgSpec>()
    val groups: salvo.platform.core.list.MutList<salvo.args.GroupSpec> = mutableListOf<salvo.args.GroupSpec>()
    val commands: salvo.platform.core.list.MutList<salvo.args.CmdSpec> = mutableListOf<salvo.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    var required__c1__c2__c3__c4: Boolean = false
    val many__c1__c2__c3__c4: Boolean = false
    val switch__c1__c2__c3__c4: Boolean = false
    val defaultText__c1__c2__c3__c4: String? = null
    val choices__c1__c2__c3__c4: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    var exclusive__c1__c2__c3__c4: Boolean = false
    required__c1__c2__c3__c4 = true
    val child__c1__c2__c3__c4__c5__c6__c9__c10__c11__c12: salvo.args.Spec = argSpec__SpecFromUrl()
    salvo.args.mergeGroup(opts, groups, child__c1__c2__c3__c4__c5__c6__c9__c10__c11__c12, "source/FromUrl", "arm", null)
    exclusive__c1__c2__c3__c4 = true
    val child__c1__c2__c3__c4__c5__c6__c9__c10__c13__c14: salvo.args.Spec = argSpec__SpecFromFile()
    salvo.args.mergeGroup(opts, groups, child__c1__c2__c3__c4__c5__c6__c9__c10__c13__c14, "source/FromFile", "arm", null)
    exclusive__c1__c2__c3__c4 = true
    val child__c1__c2__c3__c4__c5__c6__c9__c10__c15__c16: salvo.args.Spec = argSpec__SpecFromStdin()
    salvo.args.mergeGroup(opts, groups, child__c1__c2__c3__c4__c5__c6__c9__c10__c15__c16, "source/FromStdin", "arm", null)
    exclusive__c1__c2__c3__c4 = true
    if (exclusive__c1__c2__c3__c4) {
        salvo.args.addOne(groups, "source", required__c1__c2__c3__c4, "Where to read.")
    }
    val short__c1__c2__c3__c4: Char? = null
    val opt__c1__c2__c3__c4: salvo.args.OptSpec = salvo.args.OptSpec(field = "source", short = short__c1__c2__c3__c4, takesValue = !(switch__c1__c2__c3__c4), repeats = many__c1__c2__c3__c4, required = (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help = "Where to read.", defaultText = defaultText__c1__c2__c3__c4, choices = choices__c1__c2__c3__c4, group = null)
    if ((!(exclusive__c1__c2__c3__c4) || !(salvo.core.list.isEmpty(opt__c1__c2__c3__c4.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c1__c2__c3__c4)
    }
    val required__c18__c19__c20__c21: Boolean = false
    var many__c18__c19__c20__c21: Boolean = false
    val switch__c18__c19__c20__c21: Boolean = false
    val defaultText__c18__c19__c20__c21: String? = null
    val choices__c18__c19__c20__c21: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c18__c19__c20__c21: Boolean = false
    many__c18__c19__c20__c21 = true
    if (exclusive__c18__c19__c20__c21) {
        salvo.args.addOne(groups, "header", required__c18__c19__c20__c21, "A header line to send; repeatable.")
    }
    val short__c18__c19__c20__c21: Char? = null
    val opt__c18__c19__c20__c21: salvo.args.OptSpec = salvo.args.OptSpec(field = "header", short = short__c18__c19__c20__c21, takesValue = !(switch__c18__c19__c20__c21), repeats = many__c18__c19__c20__c21, required = (required__c18__c19__c20__c21 && !(exclusive__c18__c19__c20__c21)), help = "A header line to send; repeatable.", defaultText = defaultText__c18__c19__c20__c21, choices = choices__c18__c19__c20__c21, group = null)
    if ((!(exclusive__c18__c19__c20__c21) || !(salvo.core.list.isEmpty(opt__c18__c19__c20__c21.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c18__c19__c20__c21)
    }
    val name: String = salvo.args.commandName("Fetch")
    return salvo.args.Spec(name = name, about = "Fetch a document.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun fromFound__Found_OkFetchOrErrsNonEmptyListArgError(found: salvo.args.Found): Union2<Fetch, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> {
    val errors: salvo.platform.core.list.MutList<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>> = mutableListOf<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>()
    for (e in salvo.platform.core.list.each(found.errors)) {
        salvo.core.list.addPlatform(errors, e)
    }
    val s__c1__c2__c3__c4__c5__c6__c7__c8: salvo.args.Spec = argSpec__SpecFromUrl()
    if (salvo.args.anyGiven(found, s__c1__c2__c3__c4__c5__c6__c7__c8)) {
        val r__c1__c2__c3__c4__c5__c6__c7__c8: Union2<FromUrl, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> = fromFound__Found_OkFromUrlOrErrsNonEmptyListArgError(found)
        salvo.args.groupErrors(errors, r__c1__c2__c3__c4__c5__c6__c7__c8, salvo.core.list.sizePlatform(found.errors))
    }
    val s__c1__c2__c3__c4__c5__c6__c9__c10: salvo.args.Spec = argSpec__SpecFromFile()
    if (salvo.args.anyGiven(found, s__c1__c2__c3__c4__c5__c6__c9__c10)) {
        val r__c1__c2__c3__c4__c5__c6__c9__c10: Union2<FromFile, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> = fromFound__Found_OkFromFileOrErrsNonEmptyListArgError(found)
        salvo.args.groupErrors(errors, r__c1__c2__c3__c4__c5__c6__c9__c10, salvo.core.list.sizePlatform(found.errors))
    }
    val s__c1__c2__c3__c4__c5__c6__c11__c12: salvo.args.Spec = argSpec__SpecFromStdin()
    if (salvo.args.anyGiven(found, s__c1__c2__c3__c4__c5__c6__c11__c12)) {
        val r__c1__c2__c3__c4__c5__c6__c11__c12: Union2<FromStdin, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> = fromFound__Found_OkFromStdinOrErrsNonEmptyListArgError(found)
        salvo.args.groupErrors(errors, r__c1__c2__c3__c4__c5__c6__c11__c12, salvo.core.list.sizePlatform(found.errors))
    }
    val positional__c13__c14__c15__c16__c17__c18__c19: Boolean = false
    val shown__c13__c14__c15__c16__c17__c18__c19: String = salvo.args.shownName("header", positional__c13__c14__c15__c16__c17__c18__c19)
    val what__c13__c14__c15__c16__c17__c18__c19: String = "a valid value"
    salvo.args.checkValue(errors, found, "header", shown__c13__c14__c15__c16__c17__c18__c19, what__c13__c14__c15__c16__c17__c18__c19, fun(_v: String): String? {
        return null
    }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    if (salvo.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Fetch, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Errs(errors))
    }
    return Union2.U1<Fetch, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Ok(Fetch(source = salvo.args.eval(fun(): Union3<FromUrl, FromFile, FromStdin> {
        val s__c21__c22__c23__c24__c25__c26__c27__c28: salvo.args.Spec = argSpec__SpecFromUrl()
        if (salvo.args.anyGiven(found, s__c21__c22__c23__c24__c25__c26__c27__c28)) {
            return Union3.U1<FromUrl, FromFile, FromStdin>(salvo.args.built(fromFound__Found_OkFromUrlOrErrsNonEmptyListArgError(found)))
        }
        val s__c21__c22__c23__c24__c25__c26__c29__c30: salvo.args.Spec = argSpec__SpecFromFile()
        if (salvo.args.anyGiven(found, s__c21__c22__c23__c24__c25__c26__c29__c30)) {
            return Union3.U2<FromUrl, FromFile, FromStdin>(salvo.args.built(fromFound__Found_OkFromFileOrErrsNonEmptyListArgError(found)))
        }
        val s__c21__c22__c23__c24__c25__c26__c31__c32: salvo.args.Spec = argSpec__SpecFromStdin()
        if (salvo.args.anyGiven(found, s__c21__c22__c23__c24__c25__c26__c31__c32)) {
            return Union3.U3<FromUrl, FromFile, FromStdin>(salvo.args.built(fromFound__Found_OkFromStdinOrErrsNonEmptyListArgError(found)))
        }
        val v__c21__c22__c23__c24__c25__c26: String? = salvo.args.valueOf(found, "source")
        if (!((v__c21__c22__c23__c24__c25__c26 == null))) {
        }
        return throw AssertionError(("salvo: " + ("`source` was checked as required") + " at main:157:1"))
    }), header = salvo.args.eval(fun(): List<String> {
        val all__c41__c42__c43__c44__c45__c46__c47__c48: List<String> = salvo.args.readAll(found, "header", { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
        run {
            return all__c41__c42__c43__c44__c45__c46__c47__c48
        }
        return throw AssertionError(("salvo: " + ("`header` was checked as required") + " at main:157:1"))
    }))))
}

fun argSpec__SpecFromUrl(): salvo.args.Spec {
    val opts: salvo.platform.core.list.MutList<salvo.args.OptSpec> = mutableListOf<salvo.args.OptSpec>()
    val args: salvo.platform.core.list.MutList<salvo.args.ArgSpec> = mutableListOf<salvo.args.ArgSpec>()
    val groups: salvo.platform.core.list.MutList<salvo.args.GroupSpec> = mutableListOf<salvo.args.GroupSpec>()
    val commands: salvo.platform.core.list.MutList<salvo.args.CmdSpec> = mutableListOf<salvo.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    var required__c1__c2__c3__c4: Boolean = false
    val many__c1__c2__c3__c4: Boolean = false
    val switch__c1__c2__c3__c4: Boolean = false
    val defaultText__c1__c2__c3__c4: String? = null
    val choices__c1__c2__c3__c4: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c1__c2__c3__c4: Boolean = false
    required__c1__c2__c3__c4 = true
    if (exclusive__c1__c2__c3__c4) {
        salvo.args.addOne(groups, "url", required__c1__c2__c3__c4, "Where to fetch from.")
    }
    val short__c1__c2__c3__c4: Char? = null
    val opt__c1__c2__c3__c4: salvo.args.OptSpec = salvo.args.OptSpec(field = "url", short = short__c1__c2__c3__c4, takesValue = !(switch__c1__c2__c3__c4), repeats = many__c1__c2__c3__c4, required = (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help = "Where to fetch from.", defaultText = defaultText__c1__c2__c3__c4, choices = choices__c1__c2__c3__c4, group = null)
    if ((!(exclusive__c1__c2__c3__c4) || !(salvo.core.list.isEmpty(opt__c1__c2__c3__c4.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c1__c2__c3__c4)
    }
    val required__c11__c12__c13__c14: Boolean = false
    val many__c11__c12__c13__c14: Boolean = false
    val switch__c11__c12__c13__c14: Boolean = false
    var defaultText__c11__c12__c13__c14: String? = null
    val choices__c11__c12__c13__c14: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c11__c12__c13__c14: Boolean = false
    defaultText__c11__c12__c13__c14 = "${30}"
    if (exclusive__c11__c12__c13__c14) {
        salvo.args.addOne(groups, "timeout", required__c11__c12__c13__c14, "Seconds to wait.")
    }
    val short__c11__c12__c13__c14: Char? = null
    val defaultText__c11__c12__c13__c14_1: String = defaultText__c11__c12__c13__c14!!
    val opt__c11__c12__c13__c14: salvo.args.OptSpec = salvo.args.OptSpec(field = "timeout", short = short__c11__c12__c13__c14, takesValue = !(switch__c11__c12__c13__c14), repeats = many__c11__c12__c13__c14, required = (required__c11__c12__c13__c14 && !(exclusive__c11__c12__c13__c14)), help = "Seconds to wait.", defaultText = defaultText__c11__c12__c13__c14_1, choices = choices__c11__c12__c13__c14, group = null)
    if ((!(exclusive__c11__c12__c13__c14) || !(salvo.core.list.isEmpty(opt__c11__c12__c13__c14.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c11__c12__c13__c14)
    }
    val name: String = salvo.args.commandName("FromUrl")
    return salvo.args.Spec(name = name, about = null, opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkFromUrlOrErrsNonEmptyListArgError(found: salvo.args.Found): Union2<FromUrl, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> {
    val errors: salvo.platform.core.list.MutList<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>> = mutableListOf<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>()
    for (e in salvo.platform.core.list.each(found.errors)) {
        salvo.core.list.addPlatform(errors, e)
    }
    val positional__c1__c2__c3__c4__c5__c6__c7: Boolean = false
    val shown__c1__c2__c3__c4__c5__c6__c7: String = salvo.args.shownName("url", positional__c1__c2__c3__c4__c5__c6__c7)
    val what__c1__c2__c3__c4__c5__c6__c7: String = "a valid value"
    salvo.args.checkValue(errors, found, "url", shown__c1__c2__c3__c4__c5__c6__c7, what__c1__c2__c3__c4__c5__c6__c7, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    val positional__c11__c12__c13__c14__c15__c16__c17: Boolean = false
    val shown__c11__c12__c13__c14__c15__c16__c17: String = salvo.args.shownName("timeout", positional__c11__c12__c13__c14__c15__c16__c17)
    var what__c11__c12__c13__c14__c15__c16__c17: String = "a valid value"
    what__c11__c12__c13__c14__c15__c16__c17 = "an integer"
    salvo.args.checkValue(errors, found, "timeout", shown__c11__c12__c13__c14__c15__c16__c17, what__c11__c12__c13__c14__c15__c16__c17, fun(v: Int): String? {
        return null
    }, { __a0 -> salvo.core.string.fromStr__Str_IntOpt(__a0) })
    if (salvo.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<FromUrl, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Errs(errors))
    }
    return Union2.U1<FromUrl, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Ok(FromUrl(url = salvo.args.eval(fun(): String {
        return salvo.args.readValue(found, "url", null, fun(v__c23__c24__c25__c26__c27__c28__c29__c30: String): String? {
            return (if (true) {
                v__c23__c24__c25__c26__c27__c28__c29__c30
            } else {
                null
            })
        }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    }), timeout = salvo.args.eval(fun(): Int {
        return salvo.args.readValue(found, "timeout", 30, fun(v__c33__c34__c35__c36__c37__c38__c39__c40: Int): Int? {
            return (if (true) {
                v__c33__c34__c35__c36__c37__c38__c39__c40
            } else {
                null
            })
        }, { __a0 -> salvo.core.string.fromStr__Str_IntOpt(__a0) })
    }))))
}

fun argSpec__SpecFromFile(): salvo.args.Spec {
    val opts: salvo.platform.core.list.MutList<salvo.args.OptSpec> = mutableListOf<salvo.args.OptSpec>()
    val args: salvo.platform.core.list.MutList<salvo.args.ArgSpec> = mutableListOf<salvo.args.ArgSpec>()
    val groups: salvo.platform.core.list.MutList<salvo.args.GroupSpec> = mutableListOf<salvo.args.GroupSpec>()
    val commands: salvo.platform.core.list.MutList<salvo.args.CmdSpec> = mutableListOf<salvo.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    var required__c1__c2__c3__c4: Boolean = false
    val many__c1__c2__c3__c4: Boolean = false
    val switch__c1__c2__c3__c4: Boolean = false
    val defaultText__c1__c2__c3__c4: String? = null
    val choices__c1__c2__c3__c4: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    val exclusive__c1__c2__c3__c4: Boolean = false
    required__c1__c2__c3__c4 = true
    if (exclusive__c1__c2__c3__c4) {
        salvo.args.addOne(groups, "file", required__c1__c2__c3__c4, "A file to read instead.")
    }
    val short__c1__c2__c3__c4: Char? = null
    val opt__c1__c2__c3__c4: salvo.args.OptSpec = salvo.args.OptSpec(field = "file", short = short__c1__c2__c3__c4, takesValue = !(switch__c1__c2__c3__c4), repeats = many__c1__c2__c3__c4, required = (required__c1__c2__c3__c4 && !(exclusive__c1__c2__c3__c4)), help = "A file to read instead.", defaultText = defaultText__c1__c2__c3__c4, choices = choices__c1__c2__c3__c4, group = null)
    if ((!(exclusive__c1__c2__c3__c4) || !(salvo.core.list.isEmpty(opt__c1__c2__c3__c4.choices)))) {
        salvo.core.list.addPlatform(opts, opt__c1__c2__c3__c4)
    }
    val name: String = salvo.args.commandName("FromFile")
    return salvo.args.Spec(name = name, about = null, opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkFromFileOrErrsNonEmptyListArgError(found: salvo.args.Found): Union2<FromFile, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> {
    val errors: salvo.platform.core.list.MutList<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>> = mutableListOf<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>()
    for (e in salvo.platform.core.list.each(found.errors)) {
        salvo.core.list.addPlatform(errors, e)
    }
    val positional__c1__c2__c3__c4__c5__c6__c7: Boolean = false
    val shown__c1__c2__c3__c4__c5__c6__c7: String = salvo.args.shownName("file", positional__c1__c2__c3__c4__c5__c6__c7)
    val what__c1__c2__c3__c4__c5__c6__c7: String = "a valid value"
    salvo.args.checkValue(errors, found, "file", shown__c1__c2__c3__c4__c5__c6__c7, what__c1__c2__c3__c4__c5__c6__c7, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    if (salvo.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<FromFile, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Errs(errors))
    }
    return Union2.U1<FromFile, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Ok(FromFile(file = salvo.args.eval(fun(): String {
        return salvo.args.readValue(found, "file", null, fun(v__c11__c12__c13__c14__c15__c16__c17__c18: String): String? {
            return (if (true) {
                v__c11__c12__c13__c14__c15__c16__c17__c18
            } else {
                null
            })
        }, { __a0 -> salvo.core.string.fromStr__Str_StrOpt(__a0) })
    }))))
}

fun argSpec__SpecFromStdin(): salvo.args.Spec {
    val opts: salvo.platform.core.list.MutList<salvo.args.OptSpec> = mutableListOf<salvo.args.OptSpec>()
    val args: salvo.platform.core.list.MutList<salvo.args.ArgSpec> = mutableListOf<salvo.args.ArgSpec>()
    val groups: salvo.platform.core.list.MutList<salvo.args.GroupSpec> = mutableListOf<salvo.args.GroupSpec>()
    val commands: salvo.platform.core.list.MutList<salvo.args.CmdSpec> = mutableListOf<salvo.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    val name: String = salvo.args.commandName("FromStdin")
    return salvo.args.Spec(name = name, about = "Read standard input.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkFromStdinOrErrsNonEmptyListArgError(found: salvo.args.Found): Union2<FromStdin, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>> {
    val errors: salvo.platform.core.list.MutList<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>> = mutableListOf<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>()
    for (e in salvo.platform.core.list.each(found.errors)) {
        salvo.core.list.addPlatform(errors, e)
    }
    if (salvo.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<FromStdin, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Errs(errors))
    }
    return Union2.U1<FromStdin, List<Union11<salvo.args.UnknownOption, salvo.args.MissingValue, salvo.args.Missing, salvo.args.Invalid, salvo.args.Unexpected, salvo.args.Repeated, salvo.args.Requires, salvo.args.Conflict, salvo.args.MissingOne, salvo.args.UnknownCommand, salvo.args.HelpRequested>>>(salvo.core.result.Ok(FromStdin()))
}

