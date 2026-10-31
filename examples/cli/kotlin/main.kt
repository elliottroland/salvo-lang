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
            return "serve ${salvo.std.core.string.joinPlatform(c_1.files, ",")} on ${c_1.net.host}:${c_1.port} as ${c_1.output}, ${secure}, ${when {
                (c_1.net.workers != null) -> {
                    val __narrowed_3: Int = c_1.net.workers!!
                    __narrowed_3
                }
                else -> {
                    1
                }
            }} worker(s)"
        }
        (c is Union2.U2<*, *>) -> {
            val c_4: Fetch = ((c as Union2.U2<*, *>).value as Fetch)
            val s: Union3<FromUrl, FromFile, FromStdin> = c_4.source
            val from: String = when {
                (s is Union3.U1<*, *, *>) -> {
                    val s_5: FromUrl = ((s as Union3.U1<*, *, *>).value as FromUrl)
                    "${s_5.url} (timeout ${s_5.timeout}s)"
                }
                (s is Union3.U2<*, *, *>) -> {
                    val s_6: FromFile = ((s as Union3.U2<*, *, *>).value as FromFile)
                    "file ${s_6.file}"
                }
                (s is Union3.U3<*, *, *>) -> {
                    val s_7: FromStdin = ((s as Union3.U3<*, *, *>).value as FromStdin)
                    "standard input"
                }
                else -> {
                    throw AssertionError(("salvo: " + ("unhandled union arm") + " at main:104:24"))
                }
            }
            return "fetch ${from} with ${salvo.std.core.list.sizePlatform(c_4.header)} header(s)"
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
fun show(console: salvo.std.core.console.Console, line: List<String>) {
    salvo.std.core.console.println(console, "\$ tool ${salvo.std.core.string.joinPlatform(line, " ")}")
    val r: Union2<Tool, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> = salvo.std.args.parseArgs(line, ::argSpec__SpecTool, ::fromFound__Found_OkToolOrErrsNonEmptyListArgError)
    when {
        (r is Union2.U1<*, *>) -> {
            val r_1: Tool = ((r as Union2.U1<*, *>).value as Tool)
            val loud: String = (if (r_1.verbose) {
                " (verbose)"
            } else {
                ""
            })
            salvo.std.core.console.println(console, "  ${salvo.main.describe(r_1)}${loud}")
        }
        (r is Union2.U2<*, *>) -> {
            val r_2: List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>> = ((r as Union2.U2<*, *>).value as List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>)
            for (e in salvo.platform.std.core.list.each(r_2)) {
                val one: Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested> = e
                if ((one is Union11.U11<*, *, *, *, *, *, *, *, *, *, *>)) {
                    val one_3: salvo.std.args.HelpRequested = ((one as Union11.U11<*, *, *, *, *, *, *, *, *, *, *>).value as salvo.std.args.HelpRequested)
                    console.print(one_3.text)
                } else {
                    val one_4: Union10<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand> = one.let { when (it) { is Union11.U1<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U1<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.UnknownOption); is Union11.U2<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U2<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.MissingValue); is Union11.U3<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U3<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.Missing); is Union11.U4<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U4<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.Invalid); is Union11.U5<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U5<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.Unexpected); is Union11.U6<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U6<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.Repeated); is Union11.U7<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U7<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.Requires); is Union11.U8<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U8<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.Conflict); is Union11.U9<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U9<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.MissingOne); is Union11.U10<*, *, *, *, *, *, *, *, *, *, *> -> Union10.U10<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand>(it.value as salvo.std.args.UnknownCommand); else -> throw IllegalStateException("salvo: unreachable union arm") } }
                    salvo.std.core.console.println(console, "  error: ${salvo.std.args.message(one_4.let { when (it) { is Union10.U1<*, *, *, *, *, *, *, *, *, *> -> Union11.U1<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.UnknownOption); is Union10.U2<*, *, *, *, *, *, *, *, *, *> -> Union11.U2<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.MissingValue); is Union10.U3<*, *, *, *, *, *, *, *, *, *> -> Union11.U3<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.Missing); is Union10.U4<*, *, *, *, *, *, *, *, *, *> -> Union11.U4<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.Invalid); is Union10.U5<*, *, *, *, *, *, *, *, *, *> -> Union11.U5<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.Unexpected); is Union10.U6<*, *, *, *, *, *, *, *, *, *> -> Union11.U6<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.Repeated); is Union10.U7<*, *, *, *, *, *, *, *, *, *> -> Union11.U7<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.Requires); is Union10.U8<*, *, *, *, *, *, *, *, *, *> -> Union11.U8<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.Conflict); is Union10.U9<*, *, *, *, *, *, *, *, *, *> -> Union11.U9<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.MissingOne); is Union10.U10<*, *, *, *, *, *, *, *, *, *> -> Union11.U10<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>(it.value as salvo.std.args.UnknownCommand); else -> throw IllegalStateException("salvo: unreachable union arm") } })}")
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
    val __use_1: salvo.std.core.console.__Platform_StdConsole = salvo.std.core.console.__Platform_StdConsole()
    val __handle_2: salvo.std.core.console.Console = __use_1
    val __use_3: salvo.std.args.__Platform_HostArgs = salvo.std.args.__Platform_HostArgs()
    val __handle_4: salvo.std.args.Args = __use_3
    val __use_5: salvo.std.process.__Platform_HostProcess = salvo.std.process.__Platform_HostProcess()
    val __handle_6: salvo.std.process.Process = __use_5
    salvo.main.show(__handle_2, listOf<String>("serve", "-p", "80", "index.html", "app.js"))
    salvo.main.show(__handle_2, listOf<String>("-v", "serve", "--host=0.0.0.0", "--workers", "4", "--cert", "c.pem", "--key", "k.pem", "--output", "json", "site"))
    salvo.main.show(__handle_2, listOf<String>("fetch", "--url", "https://example.com", "--header", "Accept: text/html", "--header", "X-Id: 7"))
    salvo.main.show(__handle_2, listOf<String>("fetch", "--from-stdin"))
    salvo.main.show(__handle_2, listOf<String>("serve", "-p", "0", "--output", "xml", "--cert", "c.pem"))
    salvo.main.show(__handle_2, listOf<String>("fetch", "--url", "u", "--file", "f"))
    salvo.main.show(__handle_2, listOf<String>("deploy"))
    salvo.main.show(__handle_2, listOf<String>("--help"))
    salvo.main.show(__handle_2, listOf<String>("serve", "--help"))
    val tool: Tool = salvo.std.args.argsOrExit(__handle_2, __handle_6, __handle_4.args(), ::argSpec__SpecTool, ::fromFound__Found_OkToolOrErrsNonEmptyListArgError)
    salvo.std.core.console.println(__handle_2, "argv: ${salvo.main.describe(tool)}")
}

fun argSpec__SpecTool(): salvo.std.args.Spec {
    val opts: salvo.platform.std.core.list.MutList<salvo.std.args.OptSpec> = mutableListOf<salvo.std.args.OptSpec>()
    val args: salvo.platform.std.core.list.MutList<salvo.std.args.ArgSpec> = mutableListOf<salvo.std.args.ArgSpec>()
    val groups: salvo.platform.std.core.list.MutList<salvo.std.args.GroupSpec> = mutableListOf<salvo.std.args.GroupSpec>()
    val commands: salvo.platform.std.core.list.MutList<salvo.std.args.CmdSpec> = mutableListOf<salvo.std.args.CmdSpec>()
    val commandRequired: Boolean = false
    var commandFields: Int = 0
    val required: Boolean = false
    val many: Boolean = false
    var switch: Boolean = false
    val defaultText: String? = null
    val choices: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive: Boolean = false
    switch = true
    if (exclusive) {
        salvo.std.args.addOne(groups, "verbose", required, "Print more.")
    }
    var short: Char? = null
    short = 'v'
    val short_1: Char = short!!
    val opt: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "verbose", short = short_1, takesValue = !(switch), repeats = many, required = (required && !(exclusive)), help = "Print more.", defaultText = defaultText, choices = choices, group = null)
    if ((!(exclusive) || !(salvo.std.core.list.isEmpty(opt.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt)
    }
    commandFields = (commandFields + 1)
    val child: salvo.std.args.Spec = salvo.main.argSpec__SpecServe()
    salvo.std.core.list.addPlatform(commands, salvo.std.args.CmdSpec(name = child.name, about = child.about))
    val child_2: salvo.std.args.Spec = salvo.main.argSpec__SpecFetch()
    salvo.std.core.list.addPlatform(commands, salvo.std.args.CmdSpec(name = child_2.name, about = child_2.about))
    val name: String = salvo.std.args.commandName("Tool")
    return salvo.std.args.Spec(name = name, about = "A toy file server.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkToolOrErrsNonEmptyListArgError(found: salvo.std.args.Found): Union2<Tool, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> {
    val errors: salvo.platform.std.core.list.MutList<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>> = mutableListOf<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>()
    for (e in salvo.platform.std.core.list.each(found.errors)) {
        salvo.std.core.list.addPlatform(errors, e)
    }
    val s: salvo.std.args.Spec = salvo.main.argSpec__SpecServe()
    if (salvo.std.args.isCommand(found, s.name)) {
        val r: Union2<Serve, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> = salvo.std.args.parseNamed(found.rest, found.path, ::argSpec__SpecServe, ::fromFound__Found_OkServeOrErrsNonEmptyListArgError)
        salvo.std.args.groupErrors(errors, r, 0)
    }
    val s_1: salvo.std.args.Spec = salvo.main.argSpec__SpecFetch()
    if (salvo.std.args.isCommand(found, s_1.name)) {
        val r: Union2<Fetch, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> = salvo.std.args.parseNamed(found.rest, found.path, ::argSpec__SpecFetch, ::fromFound__Found_OkFetchOrErrsNonEmptyListArgError)
        salvo.std.args.groupErrors(errors, r, 0)
    }
    if (salvo.std.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Tool, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Errs(errors))
    }
    return Union2.U1<Tool, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Ok(Tool(verbose = salvo.std.args.eval(fun(): Boolean {
        return salvo.std.args.given(found, "verbose")
    }), command = salvo.std.args.eval(fun(): Union2<Serve, Fetch>? {
        val s: salvo.std.args.Spec = salvo.main.argSpec__SpecServe()
        if (salvo.std.args.isCommand(found, s.name)) {
            return Union2.U1<Serve, Fetch>(salvo.std.args.built(salvo.std.args.parseNamed(found.rest, found.path, ::argSpec__SpecServe, ::fromFound__Found_OkServeOrErrsNonEmptyListArgError)))
        }
        val s_2: salvo.std.args.Spec = salvo.main.argSpec__SpecFetch()
        if (salvo.std.args.isCommand(found, s_2.name)) {
            return Union2.U2<Serve, Fetch>(salvo.std.args.built(salvo.std.args.parseNamed(found.rest, found.path, ::argSpec__SpecFetch, ::fromFound__Found_OkFetchOrErrsNonEmptyListArgError)))
        }
        return null
    }))))
}

fun argSpec__SpecServe(): salvo.std.args.Spec {
    val opts: salvo.platform.std.core.list.MutList<salvo.std.args.OptSpec> = mutableListOf<salvo.std.args.OptSpec>()
    val args: salvo.platform.std.core.list.MutList<salvo.std.args.ArgSpec> = mutableListOf<salvo.std.args.ArgSpec>()
    val groups: salvo.platform.std.core.list.MutList<salvo.std.args.GroupSpec> = mutableListOf<salvo.std.args.GroupSpec>()
    val commands: salvo.platform.std.core.list.MutList<salvo.std.args.CmdSpec> = mutableListOf<salvo.std.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    val child: salvo.std.args.Spec = salvo.main.argSpec__SpecNet()
    salvo.std.args.mergeGroup(opts, groups, child, "net", "flat", null)
    val required: Boolean = false
    val many: Boolean = false
    val switch: Boolean = false
    var defaultText: String? = null
    val choices: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive: Boolean = false
    defaultText = "${kotlin.run {
        val __default_1: Int = 8080
        (if (!(salvo.std.core.range.InRange_qualifies(__default_1, 1, 65535))) throw AssertionError(("salvo: " + ("the default for field `port` is not `InRange(1, 65535) Int`") + " at main:36:16")) else Unit)
        __default_1
    }}"
    if (exclusive) {
        salvo.std.args.addOne(groups, "port", required, "Port to listen on.")
    }
    var short: Char? = null
    short = 'p'
    val short_2: Char = short!!
    val defaultText_3: String = defaultText!!
    val opt: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "port", short = short_2, takesValue = !(switch), repeats = many, required = (required && !(exclusive)), help = "Port to listen on.", defaultText = defaultText_3, choices = choices, group = null)
    if ((!(exclusive) || !(salvo.std.core.list.isEmpty(opt.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt)
    }
    val required_4: Boolean = false
    val many_5: Boolean = false
    val switch_6: Boolean = false
    var defaultText_7: String? = null
    val choices_8: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive_9: Boolean = false
    defaultText_7 = "text"
    salvo.std.core.list.addPlatform(choices_8, salvo.std.args.literalText("\"json\""))
    salvo.std.core.list.addPlatform(choices_8, salvo.std.args.literalText("\"text\""))
    if (exclusive_9) {
        salvo.std.args.addOne(groups, "output", required_4, "Output format.")
    }
    val short_10: Char? = null
    val defaultText_11: String = defaultText_7!!
    val opt_12: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "output", short = short_10, takesValue = !(switch_6), repeats = many_5, required = (required_4 && !(exclusive_9)), help = "Output format.", defaultText = defaultText_11, choices = choices_8, group = null)
    if ((!(exclusive_9) || !(salvo.std.core.list.isEmpty(opt_12.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt_12)
    }
    val child_13: salvo.std.args.Spec = salvo.main.argSpec__SpecTls()
    salvo.std.args.mergeGroup(opts, groups, child_13, "tls", "together", "TLS (give both or neither).")
    var required_14: Boolean = false
    var many_15: Boolean = false
    val switch_16: Boolean = false
    val defaultText_17: String? = null
    val choices_18: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive_19: Boolean = false
    many_15 = true
    required_14 = true
    if (exclusive_19) {
        salvo.std.args.addOne(groups, "files", required_14, "Files to serve.")
    }
    val short_20: Char? = null
    val opt_21: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "files", short = short_20, takesValue = !(switch_16), repeats = many_15, required = (required_14 && !(exclusive_19)), help = "Files to serve.", defaultText = defaultText_17, choices = choices_18, group = null)
    if (exclusive_19) {
        throw AssertionError(("salvo: " + ("positional `files` is a choice of option groups") + " at main:49:16"))
    }
    salvo.std.core.list.addPlatform(args, salvo.std.args.ArgSpec(field = opt_21.field, help = opt_21.help, required = opt_21.required, many = opt_21.repeats))
    val name: String = salvo.std.args.commandName("Serve")
    return salvo.std.args.Spec(name = name, about = "Serve files over HTTP.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun fromFound__Found_OkServeOrErrsNonEmptyListArgError(found: salvo.std.args.Found): Union2<Serve, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> {
    val errors: salvo.platform.std.core.list.MutList<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>> = mutableListOf<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>()
    for (e in salvo.platform.std.core.list.each(found.errors)) {
        salvo.std.core.list.addPlatform(errors, e)
    }
    val r: Union2<Net, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> = salvo.main.fromFound__Found_OkNetOrErrsNonEmptyListArgError(found)
    salvo.std.args.groupErrors(errors, r, salvo.std.core.list.sizePlatform(found.errors))
    val positional: Boolean = false
    val shown: String = salvo.std.args.shownName("port", positional)
    var what: String = "a valid value"
    what = "an integer"
    salvo.std.args.checkValue(errors, found, "port", shown, what, fun(v: Int): String? {
        return (if (!(salvo.std.core.range.InRange_qualifies(v, 1, 65535))) {
            "${v} is not in 1..65535"
        } else {
            null
        })
    }, { __a0 -> salvo.std.core.string.fromStr__Str_IntOpt(__a0) })
    val s: salvo.std.args.Spec = salvo.main.argSpec__SpecTls()
    if (salvo.std.args.anyGiven(found, s)) {
        val r: Union2<Tls, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> = salvo.main.fromFound__Found_OkTlsOrErrsNonEmptyListArgError(found)
        salvo.std.args.groupErrors(errors, r, salvo.std.core.list.sizePlatform(found.errors))
    }
    var positional_1: Boolean = false
    positional_1 = true
    val shown_2: String = salvo.std.args.shownName("files", positional_1)
    val what_3: String = "a valid value"
    salvo.std.args.checkValue(errors, found, "files", shown_2, what_3, fun(_v: String): String? {
        return null
    }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    if (salvo.std.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Serve, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Errs(errors))
    }
    return Union2.U1<Serve, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Ok(Serve(net = salvo.std.args.eval(fun(): Net {
        return salvo.std.args.built(salvo.main.fromFound__Found_OkNetOrErrsNonEmptyListArgError(found))
    }), port = salvo.std.args.eval(fun(): Int {
        return salvo.std.args.readValue(found, "port", kotlin.run {
            val __default_4: Int = 8080
            (if (!(salvo.std.core.range.InRange_qualifies(__default_4, 1, 65535))) throw AssertionError(("salvo: " + ("the default for field `port` is not `InRange(1, 65535) Int`") + " at main:36:16")) else Unit)
            __default_4
        }, fun(v: Int): Int? {
            return (if (salvo.std.core.range.InRange_qualifies(v, 1, 65535)) {
                v
            } else {
                null
            })
        }, { __a0 -> salvo.std.core.string.fromStr__Str_IntOpt(__a0) })
    }), output = salvo.std.args.eval(fun(): String {
        val v: String? = salvo.std.args.valueOf(found, "output")
        if (!((v == null))) {
            if ((v != null && (v == "json"))) {
                val v_5: String = v!!
                return v_5
            }
            if ((v != null && (v == "text"))) {
                val v_6: String = v!!
                return v_6
            }
        }
        return "text"
    }), tls = salvo.std.args.eval(fun(): Tls? {
        val s: salvo.std.args.Spec = salvo.main.argSpec__SpecTls()
        if (salvo.std.args.anyGiven(found, s)) {
            return salvo.std.args.built(salvo.main.fromFound__Found_OkTlsOrErrsNonEmptyListArgError(found))
        }
        return null
    }), files = salvo.std.args.eval(fun(): List<String> {
        val all: List<String> = salvo.std.args.readAll(found, "files", { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
        if (salvo.std.core.list.NonEmpty_qualifies(all)) {
            return all
        }
        return throw AssertionError(("salvo: " + ("`files` was checked as required") + " at main:49:16"))
    }))))
}

fun argSpec__SpecNet(): salvo.std.args.Spec {
    val opts: salvo.platform.std.core.list.MutList<salvo.std.args.OptSpec> = mutableListOf<salvo.std.args.OptSpec>()
    val args: salvo.platform.std.core.list.MutList<salvo.std.args.ArgSpec> = mutableListOf<salvo.std.args.ArgSpec>()
    val groups: salvo.platform.std.core.list.MutList<salvo.std.args.GroupSpec> = mutableListOf<salvo.std.args.GroupSpec>()
    val commands: salvo.platform.std.core.list.MutList<salvo.std.args.CmdSpec> = mutableListOf<salvo.std.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    val required: Boolean = false
    val many: Boolean = false
    val switch: Boolean = false
    var defaultText: String? = null
    val choices: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive: Boolean = false
    defaultText = "127.0.0.1"
    if (exclusive) {
        salvo.std.args.addOne(groups, "host", required, "Address to bind.")
    }
    val short: Char? = null
    val defaultText_1: String = defaultText!!
    val opt: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "host", short = short, takesValue = !(switch), repeats = many, required = (required && !(exclusive)), help = "Address to bind.", defaultText = defaultText_1, choices = choices, group = null)
    if ((!(exclusive) || !(salvo.std.core.list.isEmpty(opt.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt)
    }
    val required_2: Boolean = false
    val many_3: Boolean = false
    val switch_4: Boolean = false
    val defaultText_5: String? = null
    val choices_6: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive_7: Boolean = false
    if (exclusive_7) {
        salvo.std.args.addOne(groups, "workers", required_2, "Connections served at once.")
    }
    val short_8: Char? = null
    val opt_9: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "workers", short = short_8, takesValue = !(switch_4), repeats = many_3, required = (required_2 && !(exclusive_7)), help = "Connections served at once.", defaultText = defaultText_5, choices = choices_6, group = null)
    if ((!(exclusive_7) || !(salvo.std.core.list.isEmpty(opt_9.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt_9)
    }
    val name: String = salvo.std.args.commandName("Net")
    return salvo.std.args.Spec(name = name, about = "Network options.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkNetOrErrsNonEmptyListArgError(found: salvo.std.args.Found): Union2<Net, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> {
    val errors: salvo.platform.std.core.list.MutList<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>> = mutableListOf<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>()
    for (e in salvo.platform.std.core.list.each(found.errors)) {
        salvo.std.core.list.addPlatform(errors, e)
    }
    val positional: Boolean = false
    val shown: String = salvo.std.args.shownName("host", positional)
    val what: String = "a valid value"
    salvo.std.args.checkValue(errors, found, "host", shown, what, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    val positional_1: Boolean = false
    val shown_2: String = salvo.std.args.shownName("workers", positional_1)
    var what_3: String = "a valid value"
    what_3 = "an integer"
    salvo.std.args.checkValue(errors, found, "workers", shown_2, what_3, fun(_v: Int): String? {
        return null
    }, { __a0 -> salvo.std.core.string.fromStr__Str_IntOpt(__a0) })
    if (salvo.std.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Net, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Errs(errors))
    }
    return Union2.U1<Net, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Ok(Net(host = salvo.std.args.eval(fun(): String {
        return salvo.std.args.readValue(found, "host", "127.0.0.1", fun(v: String): String? {
            return (if (true) {
                v
            } else {
                null
            })
        }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    }), workers = salvo.std.args.eval(fun(): Int? {
        return salvo.std.args.readOptional(found, "workers", { __a0 -> salvo.std.core.string.fromStr__Str_IntOpt(__a0) })
    }))))
}

fun argSpec__SpecTls(): salvo.std.args.Spec {
    val opts: salvo.platform.std.core.list.MutList<salvo.std.args.OptSpec> = mutableListOf<salvo.std.args.OptSpec>()
    val args: salvo.platform.std.core.list.MutList<salvo.std.args.ArgSpec> = mutableListOf<salvo.std.args.ArgSpec>()
    val groups: salvo.platform.std.core.list.MutList<salvo.std.args.GroupSpec> = mutableListOf<salvo.std.args.GroupSpec>()
    val commands: salvo.platform.std.core.list.MutList<salvo.std.args.CmdSpec> = mutableListOf<salvo.std.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    var required: Boolean = false
    val many: Boolean = false
    val switch: Boolean = false
    val defaultText: String? = null
    val choices: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive: Boolean = false
    required = true
    if (exclusive) {
        salvo.std.args.addOne(groups, "cert", required, "Certificate file.")
    }
    val short: Char? = null
    val opt: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "cert", short = short, takesValue = !(switch), repeats = many, required = (required && !(exclusive)), help = "Certificate file.", defaultText = defaultText, choices = choices, group = null)
    if ((!(exclusive) || !(salvo.std.core.list.isEmpty(opt.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt)
    }
    var required_1: Boolean = false
    val many_2: Boolean = false
    val switch_3: Boolean = false
    val defaultText_4: String? = null
    val choices_5: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive_6: Boolean = false
    required_1 = true
    if (exclusive_6) {
        salvo.std.args.addOne(groups, "key", required_1, "Private key file.")
    }
    val short_7: Char? = null
    val opt_8: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "key", short = short_7, takesValue = !(switch_3), repeats = many_2, required = (required_1 && !(exclusive_6)), help = "Private key file.", defaultText = defaultText_4, choices = choices_5, group = null)
    if ((!(exclusive_6) || !(salvo.std.core.list.isEmpty(opt_8.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt_8)
    }
    val name: String = salvo.std.args.commandName("Tls")
    return salvo.std.args.Spec(name = name, about = null, opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkTlsOrErrsNonEmptyListArgError(found: salvo.std.args.Found): Union2<Tls, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> {
    val errors: salvo.platform.std.core.list.MutList<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>> = mutableListOf<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>()
    for (e in salvo.platform.std.core.list.each(found.errors)) {
        salvo.std.core.list.addPlatform(errors, e)
    }
    val positional: Boolean = false
    val shown: String = salvo.std.args.shownName("cert", positional)
    val what: String = "a valid value"
    salvo.std.args.checkValue(errors, found, "cert", shown, what, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    val positional_1: Boolean = false
    val shown_2: String = salvo.std.args.shownName("key", positional_1)
    val what_3: String = "a valid value"
    salvo.std.args.checkValue(errors, found, "key", shown_2, what_3, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    if (salvo.std.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Tls, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Errs(errors))
    }
    return Union2.U1<Tls, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Ok(Tls(cert = salvo.std.args.eval(fun(): String {
        return salvo.std.args.readValue(found, "cert", null, fun(v: String): String? {
            return (if (true) {
                v
            } else {
                null
            })
        }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    }), key = salvo.std.args.eval(fun(): String {
        return salvo.std.args.readValue(found, "key", null, fun(v: String): String? {
            return (if (true) {
                v
            } else {
                null
            })
        }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    }))))
}

fun argSpec__SpecFetch(): salvo.std.args.Spec {
    val opts: salvo.platform.std.core.list.MutList<salvo.std.args.OptSpec> = mutableListOf<salvo.std.args.OptSpec>()
    val args: salvo.platform.std.core.list.MutList<salvo.std.args.ArgSpec> = mutableListOf<salvo.std.args.ArgSpec>()
    val groups: salvo.platform.std.core.list.MutList<salvo.std.args.GroupSpec> = mutableListOf<salvo.std.args.GroupSpec>()
    val commands: salvo.platform.std.core.list.MutList<salvo.std.args.CmdSpec> = mutableListOf<salvo.std.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    var required: Boolean = false
    val many: Boolean = false
    val switch: Boolean = false
    val defaultText: String? = null
    val choices: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    var exclusive: Boolean = false
    required = true
    val child: salvo.std.args.Spec = salvo.main.argSpec__SpecFromUrl()
    salvo.std.args.mergeGroup(opts, groups, child, "source/FromUrl", "arm", null)
    exclusive = true
    val child_1: salvo.std.args.Spec = salvo.main.argSpec__SpecFromFile()
    salvo.std.args.mergeGroup(opts, groups, child_1, "source/FromFile", "arm", null)
    exclusive = true
    val child_2: salvo.std.args.Spec = salvo.main.argSpec__SpecFromStdin()
    salvo.std.args.mergeGroup(opts, groups, child_2, "source/FromStdin", "arm", null)
    exclusive = true
    if (exclusive) {
        salvo.std.args.addOne(groups, "source", required, "Where to read.")
    }
    val short: Char? = null
    val opt: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "source", short = short, takesValue = !(switch), repeats = many, required = (required && !(exclusive)), help = "Where to read.", defaultText = defaultText, choices = choices, group = null)
    if ((!(exclusive) || !(salvo.std.core.list.isEmpty(opt.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt)
    }
    val required_3: Boolean = false
    var many_4: Boolean = false
    val switch_5: Boolean = false
    val defaultText_6: String? = null
    val choices_7: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive_8: Boolean = false
    many_4 = true
    if (exclusive_8) {
        salvo.std.args.addOne(groups, "header", required_3, "A header line to send; repeatable.")
    }
    val short_9: Char? = null
    val opt_10: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "header", short = short_9, takesValue = !(switch_5), repeats = many_4, required = (required_3 && !(exclusive_8)), help = "A header line to send; repeatable.", defaultText = defaultText_6, choices = choices_7, group = null)
    if ((!(exclusive_8) || !(salvo.std.core.list.isEmpty(opt_10.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt_10)
    }
    val name: String = salvo.std.args.commandName("Fetch")
    return salvo.std.args.Spec(name = name, about = "Fetch a document.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun fromFound__Found_OkFetchOrErrsNonEmptyListArgError(found: salvo.std.args.Found): Union2<Fetch, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> {
    val errors: salvo.platform.std.core.list.MutList<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>> = mutableListOf<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>()
    for (e in salvo.platform.std.core.list.each(found.errors)) {
        salvo.std.core.list.addPlatform(errors, e)
    }
    val s: salvo.std.args.Spec = salvo.main.argSpec__SpecFromUrl()
    if (salvo.std.args.anyGiven(found, s)) {
        val r: Union2<FromUrl, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> = salvo.main.fromFound__Found_OkFromUrlOrErrsNonEmptyListArgError(found)
        salvo.std.args.groupErrors(errors, r, salvo.std.core.list.sizePlatform(found.errors))
    }
    val s_1: salvo.std.args.Spec = salvo.main.argSpec__SpecFromFile()
    if (salvo.std.args.anyGiven(found, s_1)) {
        val r: Union2<FromFile, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> = salvo.main.fromFound__Found_OkFromFileOrErrsNonEmptyListArgError(found)
        salvo.std.args.groupErrors(errors, r, salvo.std.core.list.sizePlatform(found.errors))
    }
    val s_2: salvo.std.args.Spec = salvo.main.argSpec__SpecFromStdin()
    if (salvo.std.args.anyGiven(found, s_2)) {
        val r: Union2<FromStdin, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> = salvo.main.fromFound__Found_OkFromStdinOrErrsNonEmptyListArgError(found)
        salvo.std.args.groupErrors(errors, r, salvo.std.core.list.sizePlatform(found.errors))
    }
    val positional: Boolean = false
    val shown: String = salvo.std.args.shownName("header", positional)
    val what: String = "a valid value"
    salvo.std.args.checkValue(errors, found, "header", shown, what, fun(_v: String): String? {
        return null
    }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    if (salvo.std.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<Fetch, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Errs(errors))
    }
    return Union2.U1<Fetch, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Ok(Fetch(source = salvo.std.args.eval(fun(): Union3<FromUrl, FromFile, FromStdin> {
        val s: salvo.std.args.Spec = salvo.main.argSpec__SpecFromUrl()
        if (salvo.std.args.anyGiven(found, s)) {
            return Union3.U1<FromUrl, FromFile, FromStdin>(salvo.std.args.built(salvo.main.fromFound__Found_OkFromUrlOrErrsNonEmptyListArgError(found)))
        }
        val s_3: salvo.std.args.Spec = salvo.main.argSpec__SpecFromFile()
        if (salvo.std.args.anyGiven(found, s_3)) {
            return Union3.U2<FromUrl, FromFile, FromStdin>(salvo.std.args.built(salvo.main.fromFound__Found_OkFromFileOrErrsNonEmptyListArgError(found)))
        }
        val s_4: salvo.std.args.Spec = salvo.main.argSpec__SpecFromStdin()
        if (salvo.std.args.anyGiven(found, s_4)) {
            return Union3.U3<FromUrl, FromFile, FromStdin>(salvo.std.args.built(salvo.main.fromFound__Found_OkFromStdinOrErrsNonEmptyListArgError(found)))
        }
        val v: String? = salvo.std.args.valueOf(found, "source")
        if (!((v == null))) {
        }
        return throw AssertionError(("salvo: " + ("`source` was checked as required") + " at main:73:5"))
    }), header = salvo.std.args.eval(fun(): List<String> {
        val all: List<String> = salvo.std.args.readAll(found, "header", { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
        kotlin.run {
            return all
        }
        return throw AssertionError(("salvo: " + ("`header` was checked as required") + " at main:75:5"))
    }))))
}

fun argSpec__SpecFromUrl(): salvo.std.args.Spec {
    val opts: salvo.platform.std.core.list.MutList<salvo.std.args.OptSpec> = mutableListOf<salvo.std.args.OptSpec>()
    val args: salvo.platform.std.core.list.MutList<salvo.std.args.ArgSpec> = mutableListOf<salvo.std.args.ArgSpec>()
    val groups: salvo.platform.std.core.list.MutList<salvo.std.args.GroupSpec> = mutableListOf<salvo.std.args.GroupSpec>()
    val commands: salvo.platform.std.core.list.MutList<salvo.std.args.CmdSpec> = mutableListOf<salvo.std.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    var required: Boolean = false
    val many: Boolean = false
    val switch: Boolean = false
    val defaultText: String? = null
    val choices: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive: Boolean = false
    required = true
    if (exclusive) {
        salvo.std.args.addOne(groups, "url", required, "Where to fetch from.")
    }
    val short: Char? = null
    val opt: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "url", short = short, takesValue = !(switch), repeats = many, required = (required && !(exclusive)), help = "Where to fetch from.", defaultText = defaultText, choices = choices, group = null)
    if ((!(exclusive) || !(salvo.std.core.list.isEmpty(opt.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt)
    }
    val required_1: Boolean = false
    val many_2: Boolean = false
    val switch_3: Boolean = false
    var defaultText_4: String? = null
    val choices_5: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive_6: Boolean = false
    defaultText_4 = "${30}"
    if (exclusive_6) {
        salvo.std.args.addOne(groups, "timeout", required_1, "Seconds to wait.")
    }
    val short_7: Char? = null
    val defaultText_8: String = defaultText_4!!
    val opt_9: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "timeout", short = short_7, takesValue = !(switch_3), repeats = many_2, required = (required_1 && !(exclusive_6)), help = "Seconds to wait.", defaultText = defaultText_8, choices = choices_5, group = null)
    if ((!(exclusive_6) || !(salvo.std.core.list.isEmpty(opt_9.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt_9)
    }
    val name: String = salvo.std.args.commandName("FromUrl")
    return salvo.std.args.Spec(name = name, about = null, opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkFromUrlOrErrsNonEmptyListArgError(found: salvo.std.args.Found): Union2<FromUrl, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> {
    val errors: salvo.platform.std.core.list.MutList<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>> = mutableListOf<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>()
    for (e in salvo.platform.std.core.list.each(found.errors)) {
        salvo.std.core.list.addPlatform(errors, e)
    }
    val positional: Boolean = false
    val shown: String = salvo.std.args.shownName("url", positional)
    val what: String = "a valid value"
    salvo.std.args.checkValue(errors, found, "url", shown, what, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    val positional_1: Boolean = false
    val shown_2: String = salvo.std.args.shownName("timeout", positional_1)
    var what_3: String = "a valid value"
    what_3 = "an integer"
    salvo.std.args.checkValue(errors, found, "timeout", shown_2, what_3, fun(v: Int): String? {
        return null
    }, { __a0 -> salvo.std.core.string.fromStr__Str_IntOpt(__a0) })
    if (salvo.std.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<FromUrl, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Errs(errors))
    }
    return Union2.U1<FromUrl, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Ok(FromUrl(url = salvo.std.args.eval(fun(): String {
        return salvo.std.args.readValue(found, "url", null, fun(v: String): String? {
            return (if (true) {
                v
            } else {
                null
            })
        }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    }), timeout = salvo.std.args.eval(fun(): Int {
        return salvo.std.args.readValue(found, "timeout", 30, fun(v: Int): Int? {
            return (if (true) {
                v
            } else {
                null
            })
        }, { __a0 -> salvo.std.core.string.fromStr__Str_IntOpt(__a0) })
    }))))
}

fun argSpec__SpecFromFile(): salvo.std.args.Spec {
    val opts: salvo.platform.std.core.list.MutList<salvo.std.args.OptSpec> = mutableListOf<salvo.std.args.OptSpec>()
    val args: salvo.platform.std.core.list.MutList<salvo.std.args.ArgSpec> = mutableListOf<salvo.std.args.ArgSpec>()
    val groups: salvo.platform.std.core.list.MutList<salvo.std.args.GroupSpec> = mutableListOf<salvo.std.args.GroupSpec>()
    val commands: salvo.platform.std.core.list.MutList<salvo.std.args.CmdSpec> = mutableListOf<salvo.std.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    var required: Boolean = false
    val many: Boolean = false
    val switch: Boolean = false
    val defaultText: String? = null
    val choices: salvo.platform.std.core.list.MutList<String> = mutableListOf<String>()
    val exclusive: Boolean = false
    required = true
    if (exclusive) {
        salvo.std.args.addOne(groups, "file", required, "A file to read instead.")
    }
    val short: Char? = null
    val opt: salvo.std.args.OptSpec = salvo.std.args.OptSpec(field = "file", short = short, takesValue = !(switch), repeats = many, required = (required && !(exclusive)), help = "A file to read instead.", defaultText = defaultText, choices = choices, group = null)
    if ((!(exclusive) || !(salvo.std.core.list.isEmpty(opt.choices)))) {
        salvo.std.core.list.addPlatform(opts, opt)
    }
    val name: String = salvo.std.args.commandName("FromFile")
    return salvo.std.args.Spec(name = name, about = null, opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkFromFileOrErrsNonEmptyListArgError(found: salvo.std.args.Found): Union2<FromFile, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> {
    val errors: salvo.platform.std.core.list.MutList<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>> = mutableListOf<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>()
    for (e in salvo.platform.std.core.list.each(found.errors)) {
        salvo.std.core.list.addPlatform(errors, e)
    }
    val positional: Boolean = false
    val shown: String = salvo.std.args.shownName("file", positional)
    val what: String = "a valid value"
    salvo.std.args.checkValue(errors, found, "file", shown, what, fun(v: String): String? {
        return null
    }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    if (salvo.std.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<FromFile, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Errs(errors))
    }
    return Union2.U1<FromFile, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Ok(FromFile(file = salvo.std.args.eval(fun(): String {
        return salvo.std.args.readValue(found, "file", null, fun(v: String): String? {
            return (if (true) {
                v
            } else {
                null
            })
        }, { __a0 -> salvo.std.core.string.fromStr__Str_StrOpt(__a0) })
    }))))
}

fun argSpec__SpecFromStdin(): salvo.std.args.Spec {
    val opts: salvo.platform.std.core.list.MutList<salvo.std.args.OptSpec> = mutableListOf<salvo.std.args.OptSpec>()
    val args: salvo.platform.std.core.list.MutList<salvo.std.args.ArgSpec> = mutableListOf<salvo.std.args.ArgSpec>()
    val groups: salvo.platform.std.core.list.MutList<salvo.std.args.GroupSpec> = mutableListOf<salvo.std.args.GroupSpec>()
    val commands: salvo.platform.std.core.list.MutList<salvo.std.args.CmdSpec> = mutableListOf<salvo.std.args.CmdSpec>()
    val commandRequired: Boolean = false
    val commandFields: Int = 0
    val name: String = salvo.std.args.commandName("FromStdin")
    return salvo.std.args.Spec(name = name, about = "Read standard input.", opts = opts, args = args, groups = groups, commands = commands, commandRequired = commandRequired, commandFields = commandFields)
}

fun fromFound__Found_OkFromStdinOrErrsNonEmptyListArgError(found: salvo.std.args.Found): Union2<FromStdin, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>> {
    val errors: salvo.platform.std.core.list.MutList<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>> = mutableListOf<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>()
    for (e in salvo.platform.std.core.list.each(found.errors)) {
        salvo.std.core.list.addPlatform(errors, e)
    }
    if (salvo.std.core.list.NonEmpty_qualifies(errors)) {
        return Union2.U2<FromStdin, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Errs(errors))
    }
    return Union2.U1<FromStdin, List<Union11<salvo.std.args.UnknownOption, salvo.std.args.MissingValue, salvo.std.args.Missing, salvo.std.args.Invalid, salvo.std.args.Unexpected, salvo.std.args.Repeated, salvo.std.args.Requires, salvo.std.args.Conflict, salvo.std.args.MissingOne, salvo.std.args.UnknownCommand, salvo.std.args.HelpRequested>>>(salvo.std.core.result.Ok(FromStdin()))
}

