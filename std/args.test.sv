// [args-runtime] The tests of module `args`: the scan and the help text,
// against a description written by hand (the derive writes the same one).

struct Serve {
    port: Int,
    files: List<Str>
}

fn serve_spec() [] -> Spec<Serve> {
    return Spec<Serve> {
        name: "serve",
        about: "Serve files over HTTP.",
        opts: List(
            OptSpec { field: "host", help: "Address to bind.", default_text: "127.0.0.1" },
            OptSpec { field: "port", short: 'p', help: "Port to listen on.", required: true },
            OptSpec { field: "verbose", short: 'v', takes_value: false },
            OptSpec { field: "output", choices: List("json", "text"), default_text: "text" },
            OptSpec { field: "tag", repeats: true },
            OptSpec { field: "max_workers" }
        ),
        args: List(ArgSpec { field: "files", help: "Files to serve.", many: true })
    }
}

// What a scan found, as one line: hits, then errors.
fn found(args: List<Str>) [] -> Str => args {
    let f = scan(serve_spec(), args)
    let out: Mut Str = Mut Str("")
    for h in f.hits {
        append(out, "${h.field}=${h.value};")
    }
    for e in f.errors {
        append(out, " !${message(copy(e))}")
    }
    if f.help {
        append(out, " help")
    }
    return out
}

test "long options, with the value attached or next" {
    expect_eq(found(List("--port", "80", "--host=web", "a")), "port=80;host=web;files=a;")
}

test "short options bundle, and a value can be attached" {
    expect_eq(found(List("-vp8080", "a", "b")), "verbose=;port=8080;files=a;files=b;")
    expect_eq(found(List("-v", "-p", "80", "a")), "verbose=;port=80;files=a;")
}

test "kebab-case long names" {
    expect_eq(found(List("-p1", "--max-workers", "4", "f")), "port=1;max_workers=4;files=f;")
}

test "a negative number is a value" {
    expect_eq(found(List("-p", "-3", "f")), "port=-3;files=f;")
}

test "after -- everything is positional" {
    expect_eq(found(List("-p1", "--", "-v", "--x")), "port=1;files=-v;files=--x;")
}

test "every error is reported" {
    expect_eq(found(List("--prot", "1", "-x", "--verbose=yes", "--output", "xml", "-p")),
        "output=xml;files=1; !unknown option '--prot' !unknown option '-x' !invalid value 'yes' for '--verbose': the option takes no value !option '-p' needs a value !'--port' is required !invalid value 'xml' for '--output': expected one of json, text")
}

test "a repeated option is an error unless it repeats" {
    expect_eq(found(List("-p1", "-p2", "--tag", "a", "--tag", "b", "f")), "port=1;port=2;tag=a;tag=b;files=f; !option '--port' was given more than once")
}

test "help is asked for with --help or -h" {
    expect_eq(found(List("--help")), " !'--port' is required !'<files>' is required help")
    expect(scan(serve_spec(), List("-vh")).help, "-h in a bundle")
}

test "the readers" {
    let f = scan(serve_spec(), List("-v", "-p", "80", "--tag", "a", "--tag", "b", "x"))
    expect(given(f, "verbose"), "a switch given")
    expect(!given(f, "host"), "an option absent")
    expect_eq(value_of(f, "port") ?: "-", "80")
    expect_eq(join(values_of(f, "tag"), ","), "a,b")
}

test "a word with nowhere to go" {
    let spec = Spec<Serve> { name: "x", opts: List<OptSpec>(), args: List(ArgSpec { field: "one" }) }
    let f = scan(spec, List("a", "b"))
    expect_eq(message(copy(get(f.errors, 0) ?: unreachable!())), "unexpected argument 'b'")
}

test "command names" {
    expect_eq(command_name("Serve"), "serve")
    expect_eq(command_name("RemoteAdd"), "remote-add")
    expect_eq(command_name("Remote.Add"), "add")
}

test "the help text" {
    let want = List(
        "Serve files over HTTP.",
        "",
        "Usage: serve [OPTIONS] --port <PORT> <files>...",
        "",
        "Arguments:",
        "  <files>...                       Files to serve.",
        "",
        "Options:",
        "      --host <HOST>                Address to bind. [default: 127.0.0.1]",
        "  -p, --port <PORT>                Port to listen on.",
        "  -v, --verbose",
        "      --output <OUTPUT>            [possible values: json, text] [default: text]",
        "      --tag <TAG>",
        "      --max-workers <MAX-WORKERS>",
        "  -h, --help                       Print help",
        ""
    )
    expect_eq(help_text(serve_spec()), join(want, "\n"))
}
