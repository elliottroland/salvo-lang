//! Integration test for `salvo lsp` [cli-lsp]: speaks LSP (framed
//! JSON-RPC over stdio) to the real binary — initialize, open a broken
//! document (unsaved overlay), receive diagnostics, fix it, see them
//! clear, hover for a type, and shut down cleanly.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

const TIMEOUT: Duration = Duration::from_secs(30);

fn send(stdin: &mut ChildStdin, msg: Value) {
    let body = msg.to_string();
    write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    stdin.flush().unwrap();
}

/// Reads one framed JSON-RPC message.
fn read_message(reader: &mut impl BufRead) -> Option<Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            content_length = value.parse().ok();
        }
    }
    let mut body = vec![0u8; content_length?];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

/// Next response with the given id, skipping unrelated messages.
fn expect_response(rx: &Receiver<Value>, id: i64) -> Value {
    loop {
        let msg = rx.recv_timeout(TIMEOUT).expect("timed out waiting for response");
        if msg.get("id").and_then(Value::as_i64) == Some(id) {
            assert!(
                msg.get("error").is_none(),
                "error response: {msg}"
            );
            return msg;
        }
    }
}

/// Next `textDocument/publishDiagnostics` notification, skipping others.
fn expect_diagnostics(rx: &Receiver<Value>) -> Value {
    loop {
        let msg = rx
            .recv_timeout(TIMEOUT)
            .expect("timed out waiting for publishDiagnostics");
        if msg.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics") {
            return msg["params"].clone();
        }
    }
}

struct Lsp {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Value>,
}

impl Drop for Lsp {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn start(root: &PathBuf) -> Lsp {
    start_with_env(root, &[])
}

fn start_with_env(root: &PathBuf, env: &[(&str, &str)]) -> Lsp {
    start_with(root, env, json!({}))
}

/// A server initialized with the given client `capabilities`.
fn start_with(root: &PathBuf, env: &[(&str, &str)], capabilities: serde_json::Value) -> Lsp {
    let mut command = Command::new(env!("CARGO_BIN_EXE_salvo"));
    command
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (k, v) in env {
        command.env(k, v);
    }
    let mut child = command.spawn().expect("failed to spawn salvo lsp");
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        while let Some(msg) = read_message(&mut reader) {
            if tx.send(msg).is_err() {
                break;
            }
        }
    });

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "rootUri": format!("file://{}", root.display()),
                "capabilities": capabilities
            }
        }),
    );
    let response = expect_response(&rx, 1);
    assert!(
        response["result"]["capabilities"]["hoverProvider"] == json!(true),
        "unexpected capabilities: {response}"
    );
    // [lsp-definition]
    assert!(
        response["result"]["capabilities"]["definitionProvider"] == json!(true),
        "unexpected capabilities: {response}"
    );
    // [lsp-completion] `.` opens dot completion, `[` opens the doc-reference
    // list inside a comment, and `,` opens a struct literal's remaining
    // fields right after a field separator — all three auto-trigger as
    // typed.
    let triggers = &response["result"]["capabilities"]["completionProvider"]["triggerCharacters"];
    assert!(
        triggers == &json!([".", "[", ","]),
        "completion trigger characters: {response}"
    );
    send(
        &mut stdin,
        json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
    );
    Lsp { child, stdin, rx }
}

#[test]
fn diagnostics_hover_and_shutdown() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_session");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.sv"),
        "fn main() [use] {\n    use StdConsole()\n    println(\"hello\")\n}\n",
    )
    .unwrap();

    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("bad.sv").display());

    // Open a broken document that exists only in the editor (overlay).
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1,
                "text": "fn broken() -> Int {\n    let x: Int = \"hello\"\n    return x\n}\n"
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["uri"].as_str().unwrap(), uri);
    let diags = params["diagnostics"].as_array().unwrap();
    assert_eq!(diags.len(), 1, "diagnostics: {diags:?}");
    assert!(diags[0]["message"]
        .as_str()
        .unwrap()
        .contains("expected `Int`, found `Str`"));
    // The string literal on 0-based line 1, UTF-16 character 17.
    assert_eq!(diags[0]["range"]["start"], json!({"line": 1, "character": 17}));
    assert_eq!(diags[0]["range"]["end"], json!({"line": 1, "character": 24}));
    assert_eq!(diags[0]["severity"], json!(1));

    // Fix the document: diagnostics clear (empty publish for the URI).
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 2},
                "contentChanges": [{
                    "text": "fn answer() -> Int {\n    let value: Int = 41\n    return value\n}\n"
                }]
            }
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["uri"].as_str().unwrap(), uri);
    assert_eq!(params["diagnostics"].as_array().unwrap().len(), 0);

    // Hover over `value` in `return value` -> its checked type.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 2, "character": 11}
            }
        }),
    );
    let response = expect_response(&lsp.rx, 2);
    // [doc-markdown] Hover contents are markdown: a `salvo` code block
    // with the type, and doc sections below when there are any.
    assert_eq!(
        response["result"]["contents"]["kind"].as_str(),
        Some("markdown"),
        "unexpected hover: {response}"
    );
    assert_eq!(
        response["result"]["contents"]["value"].as_str(),
        Some("```salvo\nInt\n```"),
        "unexpected hover: {response}"
    );

    // [fn-ref-table] Hovering a fn name shows the full signature with the
    // *inferred* deductions: `x` is returned (moved), `factor` is unused
    // (kept), so the effective list is `[factor]`.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 3},
                "contentChanges": [{
                    "text": "fn scale(x: Int, factor: Int) -> Int {\n    return x\n}\n\nfn use_it() -> Int {\n    return scale(1, 2)\n}\n"
                }]
            }
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["diagnostics"].as_array().unwrap().len(), 0);

    // On the declaration name...
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 4, "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 5}
            }
        }),
    );
    let response = expect_response(&lsp.rx, 4);
    assert_eq!(
        response["result"]["contents"]["value"].as_str(),
        // [lsp-fn-origin] The origin section follows the signature.
        Some(
            "```salvo\nfn scale(x: Int, factor: Int) -> Int\n```\n\n---\n\n\
             Declared in this file.",
        ),
        "unexpected hover: {response}"
    );

    // ...and on the call-site callee.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 5, "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 5, "character": 13}
            }
        }),
    );
    let response = expect_response(&lsp.rx, 5);
    assert_eq!(
        response["result"]["contents"]["value"].as_str(),
        // [lsp-fn-origin] The origin section follows the signature.
        Some(
            "```salvo\nfn scale(x: Int, factor: Int) -> Int\n```\n\n---\n\n\
             Declared in this file.",
        ),
        "unexpected hover: {response}"
    );

    // [fate-link] Hovering a fate-linked (derived) variable presents the
    // compiler qualifier: a bare `proj` on the type line, with the
    // qualifier's parameters (root, binding site) as detail below
    // (progressive disclosure).
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 4},
                "contentChanges": [{
                    "text": "fn read(s: Str) -> None => s {\n}\n\nfn derived() {\n    let xs = \"hello\"\n    let ys = xs\n    read(ys)\n}\n"
                }]
            }
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["diagnostics"].as_array().unwrap().len(), 0);
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 6, "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 6, "character": 9}
            }
        }),
    );
    let response = expect_response(&lsp.rx, 6);
    let value = response["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("hover contents not markdown: {response}"));
    assert!(
        value.starts_with("```salvo\nproj Str\n```"),
        "unexpected hover type line: {value}"
    );
    assert!(
        value.contains("Compiler qualifier `proj`")
            && value.contains("shares fate with `xs` (bound at 6:")
            && value.contains("`copy(...)`"),
        "unexpected hover detail: {value}"
    );

    // [fate-link] The same at the *declaration* of the derived variable:
    // hovering `ys` in `let ys = xs` must also say what it shares fate
    // with — that is where a reader looks first.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 7, "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 5, "character": 9}
            }
        }),
    );
    let response = expect_response(&lsp.rx, 7);
    let value = response["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("hover contents not markdown: {response}"));
    assert!(
        value.contains("shares fate with `xs`"),
        "declaration hover should carry the fate link: {value}"
    );

    // [fate-field-disjoint] The link names the *projection* it came from,
    // not just the root: a value taken from `p.name` does not share fate
    // with all of `p`, and the hover must not say it does.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 5},
                "contentChanges": [{
                    "text": "struct Person {\n    name: Str\n}\n\nfn derived(p: Person) -> None => p {\n    let n = p.name\n    let _k = n\n}\n"
                }]
            }
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["diagnostics"].as_array().unwrap().len(), 0);
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 8, "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 5, "character": 8}
            }
        }),
    );
    let response = expect_response(&lsp.rx, 8);
    let value = response["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("hover contents not markdown: {response}"));
    assert!(
        value.contains("shares fate with `p.name`"),
        "hover should name the projection, not the whole root: {value}"
    );

    // [proj-type] A value whose *declared* type already leads with a `proj`
    // gets **one**, not two: `get` answers `(proj(list) T)?`, and the
    // fate link says the same thing the type says. Reported by
    // `demo/heap.sv` as `proj proj T?` (the heap plan's item 3, fixed
    // 2026-09-22) — a hover-only defect, since the checker's own type was
    // right all along.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 6},
                "contentChanges": [{
                    "text": "fn probe(xs: List<Str>) -> None => xs {\n    let first = xs.get(0)\n    let _seen = first\n}\n"
                }]
            }
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["diagnostics"].as_array().unwrap().len(), 0);
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 9, "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 1, "character": 9}
            }
        }),
    );
    let response = expect_response(&lsp.rx, 9);
    let value = response["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("hover contents not markdown: {response}"));
    assert!(
        value.starts_with("```salvo\nproj Str?\n```"),
        "the borrow should be stated once: {value}"
    );
    // …and the link is still named, which is the part the type does not carry.
    assert!(
        value.contains("shares fate with `xs`"),
        "hover should still carry the fate link: {value}"
    );

    // [comptime-fields] Inside a `comptime fn` body the checker has nothing
    // (the body is stamped, never checked as itself); the expansion's own
    // record answers, in `core.comptime`'s terms.
    let comptime_src = "comptime fn tag<T is Struct>(v: T) [] -> Int => v {\n    \
                        let n = 0\n    \
                        [for field in T.fields] {\n        \
                        n = n + field.index\n    \
                        }\n    \
                        return n\n}\n\
                        struct Point : Ordered<self> by auto {\n    x: Int\n}\n";
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 6},
                "contentChanges": [{"text": comptime_src}]
            }
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    // `T` in the bound, `field` at its binder, `field.index` in the body.
    for (line, character, id, expect) in [
        (0u32, 16u32, 10i64, "T is Struct"),
        (2, 9, 11, "field: Field"),
        (3, 16, 12, "field.index: Int"),
        // The declaration's own name: its signature.
        (0, 13, 13, "comptime fn tag<T is Struct>(v: T) -> Int"),
        // The `by auto` site: what it stamps, from where.
        (7, 33, 14, "comptime fn cmp<T is Struct>(a: T, b: T) -> Int"),
    ] {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/hover",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": line, "character": character}
                }
            }),
        );
        let response = expect_response(&lsp.rx, id);
        let value = response["result"]["contents"]["value"]
            .as_str()
            .unwrap_or_else(|| panic!("hover contents not markdown: {response}"));
        assert!(
            value.starts_with(&format!("```salvo\n{expect}\n```")),
            "expected `{expect}` at {line}:{character}, got: {value}"
        );
    }

    // Clean shutdown.
    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 3);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    let status = lsp.child.wait().expect("failed to wait for salvo lsp");
    assert!(status.success(), "exit status: {status}");
}

// [diag-import-suggest] An unresolved handler in `use` publishes a
// diagnostic carrying import suggestions in `data`, and
// `textDocument/codeAction` turns them into a quickfix inserting the
// import line at the top of the file.
#[test]
fn code_actions_offer_import_quickfix() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_code_action");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1,
                "text": "fn main() [use] {\n    use DefaultRandom\n}\n"
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    let diags = params["diagnostics"].as_array().unwrap();
    assert_eq!(diags.len(), 1, "diagnostics: {diags:?}");
    assert!(diags[0]["message"]
        .as_str()
        .unwrap()
        .contains("unknown handler `DefaultRandom`"));
    assert_eq!(
        diags[0]["data"]["imports"],
        json!(["random.DefaultRandom"]),
        "diagnostic: {:?}",
        diags[0]
    );

    // The client echoes the diagnostic back in the codeAction context.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "textDocument/codeAction",
            "params": {
                "textDocument": {"uri": uri},
                "range": diags[0]["range"],
                "context": {"diagnostics": [diags[0]]}
            }
        }),
    );
    let response = expect_response(&lsp.rx, 2);
    let actions = response["result"].as_array().unwrap();
    assert_eq!(actions.len(), 1, "actions: {actions:?}");
    let action = &actions[0];
    assert_eq!(
        action["title"].as_str(),
        Some("Add `import random.DefaultRandom`")
    );
    assert_eq!(action["kind"].as_str(), Some("quickfix"));
    let edit = &action["edit"]["changes"][&uri][0];
    assert_eq!(edit["newText"].as_str(), Some("import random.DefaultRandom\n"));
    // No existing imports: inserted at the top of the file.
    assert_eq!(edit["range"]["start"], json!({"line": 0, "character": 0}));

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 3);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    let status = lsp.child.wait().expect("failed to wait for salvo lsp");
    assert!(status.success(), "exit status: {status}");
}

// [lsp-definition] `textDocument/definition` resolves fn names through
// `fn_refs` (overload-precise) and every other declaration name — structs,
// qualifiers, handlers, effect members, type aliases — through `def_refs`,
// across files.
#[test]
fn goto_definition_resolves_names() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_definition");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // A second file provides a struct the main document imports.
    std::fs::write(
        root.join("shapes.sv"),
        "export struct Point {\n    x: Int,\n    y: Int\n}\n",
    )
    .unwrap();

    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    let shapes_uri = format!("file://{}", root.join("shapes.sv").display());
    // Lines (0-based):
    // 0 import shapes.Point
    // 1
    // 2 effect Beeper {
    // 3     fn beep() -> Int
    // 4 }
    // 5
    // 6 handler Loud of Beeper {
    // 7     fn beep() -> Int {
    // 8         return 7
    // 9     }
    // 10 }
    // 11
    // 12 fn double(n: Int) -> Int {
    // 13     return n + n
    // 14 }
    // 15
    // 16 fn main() [use] {
    // 17     use Loud
    // 18     let p = Point {x: 1, y: 2}
    // 19     let d = double(beep())
    // 20     let _used = d + p.x + size("ab")
    // 21 }
    let text = "import shapes.Point\n\
                \n\
                effect Beeper {\n\
                \x20   fn beep() -> Int\n\
                }\n\
                \n\
                handler Loud of Beeper {\n\
                \x20   fn beep() -> Int {\n\
                \x20       return 7\n\
                \x20   }\n\
                }\n\
                \n\
                fn double(n: Int) -> Int {\n\
                \x20   return n + n\n\
                }\n\
                \n\
                fn main() [use] {\n\
                \x20   use Loud\n\
                \x20   let p = Point {x: 1, y: 2}\n\
                \x20   let d = double(beep())\n\
                \x20   let _used = d + p.x + size(\"ab\")\n\
                }\n";
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": text
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "unexpected diagnostics: {params}"
    );

    // A call-site callee jumps to the fn declaration's name.
    let loc = definition(&mut lsp, 10, &uri, 19, 12);
    assert_eq!(loc["uri"].as_str(), Some(uri.as_str()), "loc: {loc}");
    assert_eq!(loc["range"]["start"], json!({"line": 12, "character": 3}));

    // A struct name jumps into the *other* file.
    let loc = definition(&mut lsp, 11, &uri, 18, 12);
    assert_eq!(loc["uri"].as_str(), Some(shapes_uri.as_str()), "loc: {loc}");
    // `export struct Point` [mod-export]: the name sits 7 columns further in.
    assert_eq!(loc["range"]["start"], json!({"line": 0, "character": 14}));

    // An effect member call jumps to the member's declaration in the
    // `effect` block (members have no `FnKey`).
    let loc = definition(&mut lsp, 12, &uri, 19, 20);
    assert_eq!(loc["uri"].as_str(), Some(uri.as_str()), "loc: {loc}");
    assert_eq!(loc["range"]["start"], json!({"line": 3, "character": 7}));

    // A handler name in `use` jumps to the handler declaration.
    let loc = definition(&mut lsp, 13, &uri, 17, 8);
    assert_eq!(loc["uri"].as_str(), Some(uri.as_str()), "loc: {loc}");
    assert_eq!(loc["range"]["start"], json!({"line": 6, "character": 8}));

    // The effect name in the handler's `of` clause.
    let loc = definition(&mut lsp, 14, &uri, 6, 16);
    assert_eq!(loc["range"]["start"], json!({"line": 2, "character": 7}));

    // [lsp-fn-origin] A fn's hover says which module it came from — the
    // fact that matters when a name is overloaded across scopes (user
    // request 2026-09-11). `double` is declared in this file.
    let value = hover(&mut lsp, 30, &uri, 19, 12);
    assert!(
        value.contains("Declared in this file."),
        "a local fn should say so: {value}"
    );

    // A std fn names its std module instead.
    let value = hover(&mut lsp, 31, &uri, 20, 27);
    assert!(
        value.contains("the standard library"),
        "a std fn should name the standard library: {value}"
    );

    // A declaration from the *other* file names its module.
    let value = hover(&mut lsp, 32, &uri, 18, 12);
    assert!(
        value.contains("From `shapes`"),
        "a cross-file declaration should name its module: {value}"
    );

    // [doc-comment] Hover reaches a declaration in *another* file: the
    // imported struct's name on line 18 shows its declaration and fields,
    // exactly as a local one would (user request 2026-09-11).
    let value = hover(&mut lsp, 20, &uri, 18, 12);
    assert!(
        value.contains("struct Point") && value.contains("x"),
        "imported struct hover should show its declaration: {value}"
    );

    // ...and on the `import` line itself, where a reader looks first.
    let value = hover(&mut lsp, 21, &uri, 0, 15);
    assert!(
        value.contains("struct Point"),
        "hover on the import should show the imported declaration: {value}"
    );

    // A position with no name under it yields no location.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 15, "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 1, "character": 0}
            }
        }),
    );
    let response = expect_response(&lsp.rx, 15);
    assert!(
        response["result"].is_null(),
        "expected no definition: {response}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    let status = lsp.child.wait().expect("failed to wait for salvo lsp");
    assert!(status.success(), "exit status: {status}");
}

/// Requests a definition and returns the single resulting `Location`.
fn definition(lsp: &mut Lsp, id: i64, uri: &str, line: u32, character: u32) -> Value {
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": id, "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": line, "character": character}
            }
        }),
    );
    let response = expect_response(&lsp.rx, id);
    let result = response["result"].clone();
    assert!(!result.is_null(), "no definition returned: {response}");
    result
}

/// Requests a hover and returns its markdown value.
fn hover(lsp: &mut Lsp, id: i64, uri: &str, line: u32, character: u32) -> String {
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": id, "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": line, "character": character}
            }
        }),
    );
    let response = expect_response(&lsp.rx, id);
    response["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("no hover markdown: {response}"))
        .to_string()
}

/// Doc comments in hover [doc-comment] [doc-markdown] [doc-symbol-ref]
/// [doc-struct-fields] [doc-hover-narrowed]: fn docs, struct docs with a
/// per-field section, `[symbol]` references, and a variable's type as
/// narrowed at the hovered position.
#[test]
fn hover_renders_doc_comments() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_docs");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Line numbers matter below; keep this source in sync with them.
    let source = "\
// An unrelated comment, separated by a blank line.

// A person we know about.
//
// Only [name] is required; [surname] may be absent.
struct Person {
    // Their given name.
    name: Str,
    surname: Str? = None,
    age: Int
}

// Describes a [person].
//
// Markdown works: *emphasis* and `code`. [Nonexistent] stays literal.
fn describe(person: Person) -> Str {
    return person.name
}

fn narrow(value: Int | Str) -> Str => !value {
    if value is Str {
        return value
    }
    return \"other\"
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "diagnostics: {params}"
    );

    // [doc-comment] A fn's docs are the block directly above it; the
    // signature stays the code block and the docs follow.
    let value = hover(&mut lsp, 20, &uri, 15, 5);
    assert!(
        value.starts_with("```salvo\nfn describe(person: Person) -> Str\n```"),
        "unexpected fn hover: {value}"
    );
    assert!(value.contains("Describes a"), "unexpected fn hover: {value}");
    // [doc-markdown] Markdown passes through verbatim.
    assert!(
        value.contains("*emphasis* and `code`"),
        "unexpected fn hover: {value}"
    );
    // [doc-symbol-ref] A resolvable reference becomes a link to the
    // declaration; an unresolvable one is left exactly as written.
    assert!(
        value.contains("[`person`](file://") && value.contains("#L16,13"),
        "unexpected fn hover: {value}"
    );
    assert!(
        value.contains("[Nonexistent] stays literal"),
        "unexpected fn hover: {value}"
    );

    // [doc-comment] The blank line above "A person we know about." ends
    // the block, so the unrelated first comment is not part of it.
    let value = hover(&mut lsp, 21, &uri, 5, 8);
    assert!(
        value.starts_with("```salvo\nstruct Person\n```"),
        "unexpected struct hover: {value}"
    );
    assert!(
        value.contains("A person we know about."),
        "unexpected struct hover: {value}"
    );
    assert!(
        !value.contains("An unrelated comment"),
        "unrelated comment leaked into the docs: {value}"
    );
    // [doc-struct-fields] Every field is listed with its type and default;
    // documented ones carry their own comment.
    assert!(
        value.contains("**Fields**")
            && value.contains("- `name: Str` — Their given name.")
            && value.contains("- `surname: Str? = None`")
            && value.contains("- `age: Int`"),
        "unexpected field section: {value}"
    );

    // Hovering a *use* of the struct name gives the same docs.
    let at_use = hover(&mut lsp, 22, &uri, 15, 22);
    assert!(
        at_use.contains("A person we know about.") && at_use.contains("**Fields**"),
        "unexpected struct-use hover: {at_use}"
    );

    // [doc-hover-narrowed] A variable hovers as the type known at that
    // position, with the declared type when narrowing changed it.
    let value = hover(&mut lsp, 23, &uri, 21, 16);
    assert!(
        value.starts_with("```salvo\nStr\n```"),
        "unexpected narrowed hover: {value}"
    );
    assert!(
        value.contains("Declared as `Int | Str`"),
        "unexpected narrowed hover: {value}"
    );
    // ...and on the parameter's own declaration, with no narrowing note.
    let value = hover(&mut lsp, 24, &uri, 19, 12);
    assert_eq!(
        value, "```salvo\nInt | Str\n```",
        "unexpected parameter hover: {value}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// Docs on *nested* declarations [doc-comment]: struct fields (at the
/// declaration and at an access), effect members (at the declaration and
/// at a call), handler state and handler members. Each says what declares
/// it, and `[symbol]` reaches the owner's own names [doc-symbol-ref].
#[test]
fn hover_reaches_fields_and_members() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_members");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Line numbers matter below; keep this source in sync with them.
    let source = "\
struct Person {
    // Their given name.
    //
    // Never absent: a [Person] without one cannot be built.
    name: Str,
    age: Int = 0
}

// Somewhere to write text.
effect Log {
    // Writes one line.
    //
    // The [line] is consumed.
    fn write(line: Str) -> None => line
}

handler StdLog of Log {
    state {
        // How many lines we have written.
        count: Int = 0
    }

    // Writes [line] and bumps [count].
    fn write(line: Str) -> None {
    }
}

fn f(p: Person) [Log] {
    write(p.name)
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "diagnostics: {params}"
    );

    // A field at its declaration: its own line, its docs, and its owner.
    let at_decl = hover(&mut lsp, 30, &uri, 4, 5);
    assert!(
        at_decl.starts_with("```salvo\nname: Str\n```"),
        "unexpected field hover: {at_decl}"
    );
    assert!(
        at_decl.contains("Their given name.")
            && at_decl.contains("[`Person`](file://")
            && at_decl.contains("Field of struct `Person`."),
        "unexpected field hover: {at_decl}"
    );
    // The same docs at a field *access* — `Checked::field_refs`.
    let at_use = hover(&mut lsp, 31, &uri, 28, 12);
    assert_eq!(at_use, at_decl, "field access hover differs: {at_use}");
    // A field's default renders as written.
    let defaulted = hover(&mut lsp, 32, &uri, 5, 5);
    assert!(
        defaulted.starts_with("```salvo\nage: Int = 0\n```"),
        "unexpected field hover: {defaulted}"
    );

    // An effect member at its declaration, with its declared deductions
    // (members have no `FnKey`, so nothing is inferred for them).
    let at_decl = hover(&mut lsp, 33, &uri, 13, 8);
    assert!(
        at_decl.starts_with("```salvo\nfn write(line: Str) -> None\n```"),
        "unexpected member hover: {at_decl}"
    );
    assert!(
        at_decl.contains("Writes one line.")
            && at_decl.contains("[`line`](file://")
            && at_decl.contains("Member of effect `Log`."),
        "unexpected member hover: {at_decl}"
    );
    // ...and at a call site, which resolves through `def_refs`.
    let at_call = hover(&mut lsp, 34, &uri, 28, 4);
    assert_eq!(at_call, at_decl, "member call hover differs: {at_call}");

    // Handler state and handler members, with `[symbol]` reaching the
    // handler's own names.
    let state = hover(&mut lsp, 35, &uri, 19, 8);
    assert!(
        state.starts_with("```salvo\ncount: Int = 0\n```")
            && state.contains("How many lines we have written.")
            && state.contains("Field of handler `StdLog` (state)."),
        "unexpected state hover: {state}"
    );
    let member = hover(&mut lsp, 36, &uri, 23, 8);
    assert!(
        member.starts_with("```salvo\nfn write(line: Str) -> None\n```")
            && member.contains("[`line`](file://")
            && member.contains("[`count`](file://")
            && member.contains("Member of handler `StdLog`."),
        "unexpected handler member hover: {member}"
    );

    // [lsp-definition] A field access also navigates to its declaration.
    let location = definition(&mut lsp, 37, &uri, 28, 12);
    assert_eq!(location["range"]["start"]["line"], json!(4));
    assert_eq!(location["range"]["start"]["character"], json!(4));

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-hover-field-struct] Hovering a field whose type is a **struct** shows
/// that struct's own field list — the same **Fields** block hovering the
/// struct name gives — so a reader does not have to chase the type to learn
/// its shape. Reached through `?` too (an optional-struct field). A field of
/// a scalar type shows no such section.
#[test]
fn hover_a_struct_field_shows_the_structs_fields() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_field_struct");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Line numbers matter below; keep this source in sync with them.
    let source = "\
struct Metadata {
    created: Int,
    tag: Str
}

struct Record {
    // The record's metadata.
    meta: Metadata,
    note: Metadata? = None,
    id: Int
}

fn read(r: Record) -> Int {
    return r.meta.created
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["diagnostics"].as_array().unwrap().len(), 0, "diagnostics: {params}");

    // On the `meta: Metadata` field declaration: the field line, its owner,
    // and `Metadata`'s own fields.
    let at_decl = hover(&mut lsp, 40, &uri, 7, 5);
    assert!(
        at_decl.starts_with("```salvo\nmeta: Metadata\n```"),
        "unexpected field hover: {at_decl}"
    );
    assert!(at_decl.contains("Field of struct `Record`."), "unexpected field hover: {at_decl}");
    assert!(
        at_decl.contains("**`Metadata` fields**")
            && at_decl.contains("- `created: Int`")
            && at_decl.contains("- `tag: Str`"),
        "struct-typed field hover should list the struct's fields: {at_decl}"
    );

    // At a field *access* (`r.meta`), the same expansion through `field_refs`.
    let at_use = hover(&mut lsp, 41, &uri, 13, 13);
    assert!(
        at_use.contains("**`Metadata` fields**") && at_use.contains("- `created: Int`"),
        "field-access hover should expand too: {at_use}"
    );

    // An **optional** struct field (`note: Metadata?`) expands the same way.
    let optional = hover(&mut lsp, 42, &uri, 8, 5);
    assert!(
        optional.starts_with("```salvo\nnote: Metadata? = None\n```")
            && optional.contains("**`Metadata` fields**")
            && optional.contains("- `created: Int`"),
        "optional struct-typed field hover should list the struct's fields: {optional}"
    );

    // A scalar field (`id: Int`) shows no struct-fields section.
    let scalar = hover(&mut lsp, 43, &uri, 9, 5);
    assert!(
        scalar.starts_with("```salvo\nid: Int\n```") && !scalar.contains("fields**"),
        "a scalar field should show no struct-fields section: {scalar}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [qual-refn-docs] Merged documentation: a refinement is written somewhere
/// else entirely — in the qualifier that owns the claim — so hovering the
/// refined function is the only place a reader can learn what it
/// additionally establishes *here*. The section lists the effective
/// deduction, where it came from, and the refinement's own doc comment; a
/// suppressed conflict [qual-refn] is shown too, since the type
/// system stays silent about it by design.
#[test]
fn hover_merges_refinement_docs() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_refn");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Line numbers matter below; keep this source in sync with them.
    let source = "\
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }

    // Adding an element leaves the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}

fn main() [use] {
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "diagnostics: {params}"
    );

    // Hovering the call to std's `add` (line 12, the `add` token).
    let value = hover(&mut lsp, 40, &uri, 11, 5);
    assert!(
        value.contains("**Refinements** — in scope here:"),
        "unexpected hover: {value}"
    );
    assert!(
        value.contains("- `[list: +NonEmpty preserve Idx]` — from `Idx`, `NonEmpty`"),
        "unexpected hover: {value}"
    );
    assert!(
        value.contains("Adding an element leaves the list non-empty."),
        "the refinement's own docs should be merged in: {value}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

// [doc-comment] [doc-qualifies-body] Hover on the two declaration kinds that
// had none: a `params` group (its members are the point of it), and a
// predicate qualifier, which shows its `qualifies` body when short (user
// requests 2026-09-11).
#[test]
fn hover_covers_params_groups_and_short_qualifies_bodies() {
    let root = std::env::temp_dir().join("salvo_lsp_hover_decls");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    // Lines (0-based):
    // 0 params Show<T> {
    // 1     fn show(v: T) -> Str
    // 2 }
    // 3
    // 4 qualifier Positive of Int {
    // 5     fn qualifies(int: Int) -> Bool {
    // 6         return int > 0
    // 7     }
    // 8 }
    let text = "params Show<T> {\n\
                \x20   fn show(v: T) -> Str\n\
                }\n\
                \n\
                qualifier Positive of Int {\n\
                \x20   fn qualifies(int: Int) -> Bool {\n\
                \x20       return int > 0\n\
                \x20   }\n\
                }\n";
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": text
            }}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);

    // The group's name: its declaration, members included.
    let value = hover(&mut lsp, 40, &uri, 0, 8);
    assert!(
        value.contains("params Show<T>") && value.contains("fn show"),
        "params hover should show the group and its members: {value}"
    );

    // The qualifier's name: its declaration plus the short predicate body.
    let value = hover(&mut lsp, 41, &uri, 4, 12);
    assert!(
        value.contains("qualifier Positive"),
        "qualifier hover should show its declaration: {value}"
    );
    assert!(
        value.contains("Holds when `int > 0`."),
        "a single-`return` predicate should show the expression alone: {value}"
    );
    assert!(
        !value.contains("return int > 0"),
        "the `return` and braces should not be shown: {value}"
    );

    // A `qualifies` that is more than one `return` shows nothing extra: the
    // qualifier's own doc comment is the place for that.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 2},
                "contentChanges": [{
                    "text": "qualifier Big of Int {\n\
                             \x20   fn qualifies(int: Int) -> Bool {\n\
                             \x20       let limit = 10\n\
                             \x20       return int > limit\n\
                             \x20   }\n\
                             }\n"
                }]
            }
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let value = hover(&mut lsp, 42, &uri, 0, 12);
    assert!(
        value.contains("qualifier Big") && !value.contains("Holds when"),
        "a multi-statement `qualifies` should be hidden: {value}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    let status = lsp.child.wait().expect("failed to wait for salvo lsp");
    assert!(status.success(), "exit status: {status}");
}

// [lsp-definition] [doc-comment] Two name positions that had no hover: the
// group name in a struct's obligation clause (`: Linear<self>`), and the
// qualifier name in an `is` check — where hovering used to fall through to
// the enclosing expression's `Bool` (user reports 2026-09-11).
#[test]
fn hover_reaches_obligation_and_is_check_names() {
    let root = std::env::temp_dir().join("salvo_lsp_hover_names");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    // Lines (0-based):
    // 0 params Show<T> {
    // 1     fn show(v: T) -> Str
    // 2 }
    // 3
    // 4 struct Card : Show<self> {
    // 5     face: Str
    // 6 }
    // 7
    // 8 fn show(c: Card) -> Str => c {
    // 9     return c.face
    // 10 }
    // 11
    // 12 qualifier Positive of Int {
    // 13     fn qualifies(int: Int) -> Bool {
    // 14         return int > 0
    // 15     }
    // 16 }
    // 17
    // 18 fn probe(i: Int) -> Int => i {
    // 19     if i is Positive {
    // 20         return 1
    // 21     }
    // 22     return 0
    // 23 }
    let text = "params Show<T> {\n\
                \x20   fn show(v: T) -> Str\n\
                }\n\
                \n\
                struct Card : Show<self> {\n\
                \x20   face: Str\n\
                }\n\
                \n\
                fn show(c: Card) -> Str => c {\n\
                \x20   return c.face\n\
                }\n\
                \n\
                qualifier Positive of Int {\n\
                \x20   fn qualifies(int: Int) -> Bool {\n\
                \x20       return int > 0\n\
                \x20   }\n\
                }\n\
                \n\
                fn probe(i: Int) -> Int => i {\n\
                \x20   if i is Positive {\n\
                \x20       return 1\n\
                \x20   }\n\
                \x20   return 0\n\
                }\n";
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": text
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "unexpected diagnostics: {params}"
    );

    // The group name in the obligation clause: `Show` on line 4.
    let value = hover(&mut lsp, 50, &uri, 4, 15);
    assert!(
        value.contains("params Show<T>") && value.contains("fn show"),
        "obligation-clause group hover should reach the group: {value}"
    );

    // The qualifier name in the `is` check: `Positive` on line 19.
    let value = hover(&mut lsp, 51, &uri, 19, 14);
    assert!(
        value.contains("qualifier Positive") && value.contains("Holds when `int > 0`."),
        "`is`-check qualifier hover should reach the qualifier: {value}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 98, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 98);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    let status = lsp.child.wait().expect("failed to wait for salvo lsp");
    assert!(status.success(), "exit status: {status}");
}

/// [lsp-hover-linear] [lsp-hover-overloads] Three things a hover used to leave
/// out (user requests 2026-09-18): the `linear` modifier of a linear type, the
/// `canbe linear` bound of a generic that may carry an obligation, and the
/// *other* declarations visible under a name — an overload set reads as a
/// choice, and only the full list shows what the choice was between.
#[test]
fn hover_shows_linearity_and_the_rest_of_an_overload_set() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_linear_overloads");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Line numbers matter below; keep this source in sync with them.
    let source = "\
export linear struct Ticket {
    id: Int
}

fn redeem(ticket: Ticket) -> Int => !ticket {
    let id = copy(ticket.id)
    discard(ticket)
    return id
}

fn hold<T canbe linear>(value: T) -> T => !value {
    return value
}

fn describe(n: Int) -> Str {
    return \"int\"
}

fn describe(s: Str) -> Str => !s {
    return s
}

// Two members of one effect sharing a name, which is the shape `read_to` has
// in `fs` [effect-member-overload].
effect Store {
    fn keep(n: Int) -> Bool => n
    fn keep(s: Str) -> Bool => s
}

fn main() [use] -> None {
    use StdConsole()
    let t = Ticket {id: 1}
    println(\"${redeem(t)} ${describe(1)}\")
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "diagnostics: {params}"
    );

    // [lsp-hover-linear] The struct's own declaration, and a use of it: the
    // obligation leads the signature exactly as it does in source.
    // [mod-export] The hover deliberately does **not** show `export` (user
    // decision 2026-09-18): it already says which module the declaration comes
    // from [lsp-fn-origin], which is what a reader wants from it.
    let value = hover(&mut lsp, 40, &uri, 0, 22);
    assert!(
        value.contains("linear struct Ticket") && !value.contains("export"),
        "expected the linear modifier and no `export`: {value}"
    );
    let value = hover(&mut lsp, 41, &uri, 31, 13);
    assert!(
        value.contains("linear struct Ticket") && !value.contains("export"),
        "expected the linear modifier at a use, and no `export`: {value}"
    );

    // [lsp-hover-linear] A generic that may be handed an obligation says so.
    let value = hover(&mut lsp, 42, &uri, 10, 4);
    assert!(
        value.contains("fn hold<T canbe linear>"),
        "expected the canbe bound in the signature: {value}"
    );

    // [lsp-hover-overloads] Hovering one overload lists the others, with the
    // module they come from — and does not list itself.
    let value = hover(&mut lsp, 43, &uri, 32, 29);
    assert!(
        value.starts_with("```salvo\nfn describe(n: Int) -> Str"),
        "expected the resolved overload first: {value}"
    );
    assert!(
        value.contains("Also visible under this name:")
            && value.contains("fn describe(s: Str) -> Str"),
        "expected the other overload listed: {value}"
    );
    assert!(
        value.matches("fn describe(n: Int)").count() == 1,
        "the resolved overload must not repeat in the list: {value}"
    );

    // [lsp-hover-overloads] [effect-member-overload] The member case, which is
    // the one the request named: hovering one `keep` shows the other, marked as
    // a member of its effect.
    let value = hover(&mut lsp, 44, &uri, 25, 8);
    assert!(
        value.contains("fn keep(n: Int) -> Bool"),
        "expected the hovered member: {value}"
    );
    assert!(
        value.contains("Also visible under this name:")
            && value.contains("`fn keep(s: Str) -> Bool` — of effect `Store`"),
        "expected the sibling member listed: {value}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-hover-overloads] A sibling overload's own doc comment renders inside
/// the "Also visible under this name" entry, as an indented continuation of
/// its bullet — a reader comparing candidates needs more than a bare
/// signature, and the resolved overload already gets its docs shown.
#[test]
fn hover_shows_doc_comments_of_sibling_overloads() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_overload_docs");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Line numbers matter below; keep this source in sync with them.
    let source = "\
// Describes an amount as a count.
fn describe(n: Int) -> Str {
    return \"int\"
}

// Describes an amount as free text.
fn describe(s: Str) -> Str => !s {
    return s
}

fn main() [use] -> None {
    use StdConsole()
    println(describe(1))
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "diagnostics: {params}"
    );

    // Hovering the resolved `Int` overload at the call site in `main`.
    let value = hover(&mut lsp, 50, &uri, 12, 13);
    assert!(
        value.starts_with("```salvo\nfn describe(n: Int) -> Str"),
        "expected the resolved overload first: {value}"
    );
    assert!(
        value.contains("Describes an amount as a count."),
        "expected the resolved overload's own doc comment: {value}"
    );
    assert!(
        value.contains("Also visible under this name:")
            && value.contains("fn describe(s: Str) -> Str")
            && value.contains("Describes an amount as free text."),
        "expected the sibling's doc comment inside the overload list: {value}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}


#[test]
fn hover_shows_preserve_refinements() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_preserve");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Line numbers matter below; keep this source in sync with them.
    let source = "\
struct Box canbe Mut { n: Int }
struct Key { s: Str }

qualifier Inside(box: Box) of Key {
    fn qualifies(key: Key, box: Box) -> Bool {
        return box.n > 0
    }

    // Bumping never empties the box.
    refn bump(box: Mut Box) => box: preserve Inside
}

fn bump(box: Mut Box) [] -> None => box: Mut {
    box.n = box.n + 1
}

fn wrap(box: Mut Box) [] -> None
=> box: Mut, box: preserve Inside {
    bump(box)
    return None
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "diagnostics: {params}"
    );

    // Hovering the `bump` call inside `wrap` (line 19, zero-based 18).
    let value = hover(&mut lsp, 40, &uri, 18, 5);
    assert!(
        value.contains("**Refinements** — in scope here:"),
        "unexpected hover: {value}"
    );
    assert!(
        value.contains("- `[box: preserve Inside]` — from `Inside`"),
        "expected the preserve entry: {value}"
    );
    assert!(
        value.contains("Bumping never empties the box."),
        "the refinement's own docs should be merged in: {value}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [std-shadow] [cli-lsp] The standard library is developed through the
/// shadow path: opening the repo's `std/` makes every std module an
/// on-disk shadowing file — `is_std` for the checker, a real document for
/// the editor. Hover must work there (the file-index lookup admits
/// shadows), the open buffer must *replace* the disk content rather than
/// be added beside it (or every declaration duplicates), and the
/// refinements section shows `preserve` entries [qual-preserve] — checked
/// against the real `core.list`, at the total `swap`'s delegation.
#[test]
fn hover_works_in_a_std_shadowing_file() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let root = root.canonicalize().unwrap();
    let source = std::fs::read_to_string(root.join("core/list.sv")).unwrap();
    let line = source
        .lines()
        .position(|l| l.contains("swap(list, i + 0, j + 0)"))
        .expect("the delegation line") as u32;
    let col = source
        .lines()
        .nth(line as usize)
        .unwrap()
        .find("swap")
        .unwrap() as u32;
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("core/list.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    // The buffer replaces the shadowing file's disk content: no duplicate
    // declarations, and std analyzes clean.
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "diagnostics: {params}"
    );
    let value = hover(&mut lsp, 40, &uri, line, col + 1);
    assert!(
        value.contains("-> Checked<Bool>"),
        "expected the fallible swap's signature: {value}"
    );
    assert!(
        value.contains("- `[list: preserve Idx]` — from `Idx`"),
        "expected the preserve refinement: {value}"
    );
    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [proj-infer] [lsp-hover] The opaque lends render where they are written —
/// on the return type, `-> Mut View<T> holds proj(xs)` — and not as a clause
/// entry (the pre-2026-09-24 `=> proj(xs)` spelling is gone). The same holds
/// inside a fn *type*, whose Display carries its own `… holds proj(c)`.
#[test]
fn hover_renders_the_opaque_projection_on_the_return_type() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_opaque_proj");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Line numbers matter below; keep this source in sync with them.
    let source = "\
struct View<T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn borrowed<T>(xs: List<T>) -> Mut View<T> holds proj(xs) => xs {
    return Mut View<T> { items: xs, at: 0 }
}

fn total<C, It>(c: C, ?iter: (c: C) -> Mut It holds proj(c), ?Yield<It, Int>) -> Int => c {
    let sum = 0
    let p = iter(c)
    for n in p {
        sum = sum + n
    }
    return sum
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2)
    let v = borrowed(xs)
    println(\"${v.at}\")
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    // [proj-infer-fn-type] One diagnostic, and it is expected: `total`'s slot
    // returns a bare type parameter, so its lend is inferred and the written
    // one is spare (2026-09-26). The fixture keeps it deliberately — a written
    // fn-type lend is the thing whose *rendering* is under test here, and hover
    // shows what the author wrote rather than what the compiler would infer.
    let params = expect_diagnostics(&lsp.rx);
    let diags = params["diagnostics"].as_array().unwrap();
    assert_eq!(diags.len(), 1, "diagnostics: {params}");
    assert!(
        diags[0]["message"]
            .as_str()
            .unwrap()
            .contains("says what the signature already infers"),
        "the only diagnostic is the spare-lend warning: {params}"
    );

    // The fn declaration: annotation on the return, no `proj` clause entry.
    let value = hover(&mut lsp, 30, &uri, 5, 4);
    assert!(
        value.contains("-> Mut View<T> holds proj(xs)"),
        "expected the opaque annotation on the return type: {value}"
    );
    assert!(
        !value.contains("=> proj") && !value.contains(", proj(xs)"),
        "the lends must not repeat as a clause entry: {value}"
    );

    // The fn type: its Display carries the annotation on its own return.
    let value = hover(&mut lsp, 31, &uri, 9, 4);
    assert!(
        value.contains("-> Mut It holds proj(c)"),
        "expected the fn type's own annotation: {value}"
    );

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [manifest-discovery] A workspace holding two projects: each document is
/// analysed under its own `salvo.toml`, so a name declared in one is not a
/// duplicate of the same name in the other, and a project's `main` is not
/// ambiguous with the other's. A file with no manifest above it falls back to
/// the workspace root and is told so.
#[test]
fn documents_are_analysed_under_their_own_manifest() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_manifests");
    let _ = std::fs::remove_dir_all(&root);
    for name in ["one", "two"] {
        let dir = root.join(name).join("salvo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            root.join(name).join("salvo.toml"),
            format!("[project]\nname = \"{name}\"\n\n[build]\nsrc = \"salvo\"\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("main.sv"),
            "struct Shared { x: Int }\nfn main() [use] {\n    use StdConsole()\n    println(\"hi\")\n}\n",
        )
        .unwrap();
    }
    std::fs::create_dir_all(root.join("loose")).unwrap();
    std::fs::write(root.join("loose/stray.sv"), "fn helper() -> Int { return 1 }\n").unwrap();

    let mut lsp = start(&root);
    let one = root.join("one/salvo/main.sv");
    let uri = format!("file://{}", one.display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1,
                "text": std::fs::read_to_string(&one).unwrap()
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["uri"].as_str().unwrap(), uri);
    let diags = params["diagnostics"].as_array().unwrap();
    assert!(diags.is_empty(), "a project analysed alone is clean: {diags:?}");

    // The stray file: analysed under the workspace root, and told so.
    let stray = root.join("loose/stray.sv");
    let stray_uri = format!("file://{}", stray.display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": stray_uri, "languageId": "salvo", "version": 1,
                "text": std::fs::read_to_string(&stray).unwrap()
            }}
        }),
    );
    let mut saw_note = false;
    for _ in 0..2 {
        let params = expect_diagnostics(&lsp.rx);
        if params["uri"].as_str().unwrap() == stray_uri {
            let diags = params["diagnostics"].as_array().unwrap();
            saw_note = diags.iter().any(|d| {
                d["message"].as_str().unwrap().contains("no `salvo.toml` above this file")
            });
        }
    }
    assert!(saw_note, "the fallback should be stated at the file");

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null}),
    );
    let _ = expect_response(&lsp.rx, 3);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    let _ = lsp.child.wait();
}

/// [doc-module] [manifest-deps] A module's leading comment is its
/// documentation, shown wherever the module is named: the module part of an
/// `import` (a whole-module import and the segments before an item), and an
/// `@module` selector. A dependency's modules hover the same way, saying which
/// dependency they came from — and a declaration reached from one names the
/// dependency in its origin line.
#[test]
fn hover_shows_module_docs_where_a_module_is_named() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_module_docs");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("salvo/shop")).unwrap();
    std::fs::create_dir_all(root.join("salvo_modules/aws/salvo")).unwrap();
    std::fs::write(
        root.join("salvo.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\nmodules = \"salvo_modules\"\n\n[dependencies]\naws = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("salvo_modules/aws/salvo.toml"),
        "[project]\nname = \"aws\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\n",
    )
    .unwrap();
    // A header directly above the first import is the module's (rule (b)).
    std::fs::write(
        root.join("salvo_modules/aws/salvo/aws.sv"),
        "// AWS services, modelled as actors.\n\
         //\n\
         // Start with [ProfileCredentials].\n\
         import time.Duration\n\
         \n\
         // Where a profile's credentials are read from.\n\
         export struct ProfileCredentials {\n    profile: Str = \"default\",\n    path: Str\n}\n",
    )
    .unwrap();
    // A header followed by a blank line is the module's; the run above
    // `label` is `label`'s.
    std::fs::write(
        root.join("salvo/shop/shapes.sv"),
        "// Shapes for the shop.\n\
         \n\
         // Labels [n].\n\
         export fn label(n: Int) -> Str {\n    return \"shape ${n}\"\n}\n",
    )
    .unwrap();
    // Line numbers matter below; keep this source in sync with them.
    let source = "\
import aws.ProfileCredentials
import shop.shapes.label
import time

fn label(n: Int) -> Str {
    return \"own ${n}\"
}

fn main() [Console] {
    let creds = ProfileCredentials {path: \"~/.aws/credentials\"}
    println(creds.path)
    println(label@shapes(1))
    println(label@main(2))
}
";
    std::fs::write(root.join("salvo/main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("salvo/main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "diagnostics: {params}"
    );

    // The module segment of a named import, from a dependency: the module's
    // docs, a `[symbol]` reference resolved against that module, and the
    // dependency named.
    let value = hover(&mut lsp, 30, &uri, 0, 8);
    assert!(value.starts_with("```salvo\nmodule aws\n```"), "unexpected module hover: {value}");
    assert!(value.contains("AWS services, modelled as actors."), "{value}");
    assert!(value.contains("[`ProfileCredentials`](file://"), "{value}");
    assert!(value.contains("Dependency `aws`, in `"), "{value}");
    assert!(value.contains("salvo_modules/aws/salvo/aws.sv`"), "{value}");

    // A two-segment module part is one name: hovering `shop` covers
    // `shop.shapes`, and the header before the blank line is the module's
    // (not `label`'s).
    let value = hover(&mut lsp, 31, &uri, 1, 8);
    assert!(value.starts_with("```salvo\nmodule shop.shapes\n```"), "{value}");
    assert!(value.contains("Shapes for the shop."), "{value}");
    assert!(!value.contains("Labels"), "the fn's docs are not the module's: {value}");
    assert!(value.contains("In `shop/shapes.sv`."), "{value}");
    // [doc-module] And what it offers.
    assert!(value.contains("**Functions**"), "{value}");
    assert!(value.contains("fn label(n: Int)"), "{value}");
    let value = hover(&mut lsp, 32, &uri, 1, 13);
    assert!(value.starts_with("```salvo\nmodule shop.shapes\n```"), "{value}");

    // A whole-module import of a std module: std's own header comment, and
    // the standard library named.
    let value = hover(&mut lsp, 33, &uri, 2, 8);
    assert!(value.starts_with("```salvo\nmodule time\n```"), "{value}");
    assert!(value.contains("The standard library."), "{value}");
    // [doc-module] The first functions, then a count of the rest.
    assert!(value.contains("fn seconds("), "{value}");
    assert!(value.contains(" more"), "{value}");
    assert!(value.contains("**Types**: "), "{value}");
    // [lsp-std-source] An embedded std module is reachable on disk.
    let location = definition(&mut lsp, 39, &uri, 2, 8);
    let std_uri = location["uri"].as_str().unwrap().to_string();
    assert!(std_uri.ends_with("/time.sv"), "{location}");
    let std_path = std_uri.trim_start_matches("file://");
    assert!(std::fs::read_to_string(std_path).unwrap().contains("struct Duration"), "{std_path}");

    // An `@module` selector, written as a suffix [mod-suffix]: the full path.
    let value = hover(&mut lsp, 34, &uri, 11, 20);
    assert!(value.starts_with("```salvo\nmodule shop.shapes\n```"), "{value}");
    assert!(value.contains("Shapes for the shop."), "{value}");

    // The item imported from the dependency hovers as usual, and its origin
    // line names the dependency.
    let value = hover(&mut lsp, 35, &uri, 9, 18);
    assert!(value.contains("struct ProfileCredentials"), "{value}");
    assert!(value.contains("Where a profile's credentials are read from."), "{value}");

    // The fn at `label@shapes(1)` is the other module's — the origin line
    // says so — and `@main` hovers as this file.
    let value = hover(&mut lsp, 36, &uri, 11, 14);
    assert!(value.contains("From `shop.shapes`."), "{value}");
    let value = hover(&mut lsp, 38, &uri, 12, 19);
    assert!(value.starts_with("```salvo\nmodule main\n```"), "{value}");
    assert!(value.contains("This file."), "{value}");

    // Go-to-definition on a module name lands at the top of its file.
    let location = definition(&mut lsp, 37, &uri, 0, 8);
    assert!(
        location["uri"].as_str().unwrap().ends_with("salvo_modules/aws/salvo/aws.sv"),
        "{location}"
    );
    assert_eq!(location["range"]["start"]["line"], 0);

    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-definition] [manifest-discovery] A definition's location joins the
/// file's name to the analysis's source root — the project's `src`, here a
/// subdirectory — not the workspace root (a defect until 2026-09-30, when it
/// named `<workspace>/main.sv`, which did not exist).
#[test]
fn definitions_resolve_against_the_projects_source_root() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_src_root");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("salvo")).unwrap();
    std::fs::write(root.join("salvo.toml"), "[project]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"salvo\"\n").unwrap();
    let text = "struct Point { x: Int }\n\nfn read(p: Point) [] -> Int {\n    return p.x\n}\n";
    std::fs::write(root.join("salvo/main.sv"), text).unwrap();
    let root = root.canonicalize().unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("salvo/main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": text}}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    // `Point` in `read(p: Point)`.
    let location = definition(&mut lsp, 2, &uri, 2, 12);
    assert_eq!(location["uri"].as_str().unwrap(), uri, "{location}");
    assert_eq!(location["range"]["start"], json!({"line": 0, "character": 7}), "{location}");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-completion] A handler's state field is offered as a completion
/// throughout a member body — the same as a parameter — even in a member
/// written *before* the `state {}` block (the TODOs in salvo-noticeboard's
/// disk.sv). It is `always_in_scope`, so the "declared before the cursor"
/// rule a `let` binding obeys does not hide it.
#[test]
fn completion_offers_handler_state_fields() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_completion_state");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // `before` is written above the `state {}` block, `after` below it; both
    // reference `notice_count`, so completion must offer it in each. The
    // comment in `before` exercises a `[…]` doc-reference inside a member
    // body — the `[notice_by_id]` case from disk.sv.
    let source = "\
effect Board {
    fn before() -> None
    fn after() -> None
}

handler DiskBoard of Board {
    fn before() -> None {
        // load [
        notice
    }

    state {
        notice_count: Int = 0
    }

    fn after() -> None {
        notice
    }
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let complete = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> Vec<(String, String)> {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/completion",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": line, "character": character}
                }
            }),
        );
        let response = expect_response(&lsp.rx, id);
        response["result"]
            .as_array()
            .unwrap_or_else(|| panic!("no completion list: {response}"))
            .iter()
            .map(|i| (i["label"].as_str().unwrap().to_string(), i["detail"].as_str().unwrap_or("").to_string()))
            .collect()
    };
    // In `before`'s body (line 8), above the `state {}` block.
    let items = complete(&mut lsp, 2, 8, 14);
    let before = items.iter().find(|(l, _)| l == "notice_count");
    assert!(before.is_some(), "state field offered in a member above `state {{}}`: {items:?}");
    assert_eq!(before.unwrap().1, "Int", "state field's detail is its type");
    // In `after`'s body (line 16), below the `state {}` block.
    let items = complete(&mut lsp, 3, 16, 14);
    assert!(
        items.iter().any(|(l, _)| l == "notice_count"),
        "state field offered in a member below `state {{}}`: {items:?}"
    );
    // [doc-symbol-ref] Inside a `[…]` doc reference in a member-body comment
    // (line 7, right after `[`): the state field is offered too, so a comment
    // can link `[notice_count]` with auto-complete (the disk.sv case).
    let items = complete(&mut lsp, 4, 7, 17);
    assert!(
        items.iter().any(|(l, _)| l == "notice_count"),
        "state field offered in a comment's `[` ref: {items:?}"
    );
    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-completion] [effect-member-completion] Completion offers the members
/// of effects available at the cursor — the fn's declared effects and their
/// transitive prerequisites — and withholds members of effects not in scope.
/// A `[Fs]` fn can call `Streams`' members because `effect Fs [Streams]` pulls
/// the prerequisite in (the TODOs in salvo-noticeboard's disk.sv).
#[test]
fn completion_offers_effect_members_in_scope() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_completion_effect_members");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // `Reader` requires `Bytes` ([effect-prereq]), so a `[Reader]` fn has both
    // effects in scope and may call members of each. `Other` is unrelated and
    // its member `unrelated` must never be offered here.
    let source = "\
effect Bytes {
    fn read_byte() -> Int
}

effect Reader [Bytes] {
    fn read_line() -> Str
}

effect Other {
    fn unrelated() -> None
}

fn load() [Reader] -> Str {
    read
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let complete = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> Vec<(String, String)> {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/completion",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": line, "character": character}
                }
            }),
        );
        let response = expect_response(&lsp.rx, id);
        response["result"]
            .as_array()
            .unwrap_or_else(|| panic!("no completion list: {response}"))
            .iter()
            .map(|i| (i["label"].as_str().unwrap().to_string(), i["detail"].as_str().unwrap_or("").to_string()))
            .collect()
    };
    // In `load`'s body (line 13, at `read`): the declared effect's member and
    // its prerequisite's member are both offered; `Other`'s is not.
    let items = complete(&mut lsp, 2, 13, 8);
    assert!(
        items.iter().any(|(l, _)| l == "read_line"),
        "declared effect's member offered: {items:?}"
    );
    assert!(
        items.iter().any(|(l, _)| l == "read_byte"),
        "prerequisite effect's member offered: {items:?}"
    );
    assert!(
        !items.iter().any(|(l, _)| l == "unrelated"),
        "member of an effect not in scope is withheld: {items:?}"
    );
    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-completion] Completion offers the functions in scope; after
/// `receiver.`, the ones whose first parameter the receiver fills, before
/// those taking any type, and none taking something else.
#[test]
fn completion_offers_functions_and_dot_receivers() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_completion");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "\
fn shout(s: Str) -> Str {
    return to_upper(s)
}

fn main() [Console] {
    let name = \"salvo\"
    let n = 3
    println(name.)
    sh
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let complete = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> Vec<(String, String)> {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/completion",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": line, "character": character}
                }
            }),
        );
        let response = expect_response(&lsp.rx, id);
        response["result"]
            .as_array()
            .unwrap_or_else(|| panic!("no completion list: {response}"))
            .iter()
            .map(|i| (i["label"].as_str().unwrap().to_string(), i["sortText"].as_str().unwrap_or("").to_string()))
            .collect()
    };
    // After `name.`: Str's functions, the program's own included.
    let items = complete(&mut lsp, 2, 7, 17);
    let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.contains(&"shout"), "{labels:?}");
    assert!(labels.contains(&"to_upper"), "{labels:?}");
    assert!(labels.contains(&"trim"), "{labels:?}");
    assert!(!labels.contains(&"main"), "{labels:?}");
    assert!(!labels.contains(&"push_heap"), "{labels:?}");
    let rank = |l: &str| items.iter().find(|(x, _)| x == l).map(|(_, r)| r.clone()).unwrap();
    assert!(rank("trim") < rank("copy"), "a Str fn ranks before a generic one");
    // A bare word: everything in scope.
    let items = complete(&mut lsp, 3, 8, 6);
    let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.contains(&"shout") && labels.contains(&"main") && labels.contains(&"println"), "{labels:?}");
    // [lsp-completion] Local bindings in scope are offered too: `main`'s own
    // `name` and `n`, declared above the cursor.
    assert!(labels.contains(&"name") && labels.contains(&"n"), "locals in scope: {labels:?}");
    // A local ranks ahead of a fn (sortText `0name` vs `0println`… both tier
    // 0, but a local is a value already in hand): at least it is present with
    // a VARIABLE-tier sortText.
    let name_rank = items.iter().find(|(l, _)| l == "name").map(|(_, r)| r.clone()).unwrap();
    assert!(name_rank.starts_with('0'), "a local ranks in the top tier: {name_rank}");
    // The local carries its type as detail.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 7, "method": "textDocument/completion",
            "params": {"textDocument": {"uri": uri}, "position": {"line": 8, "character": 6}}
        }),
    );
    let response = expect_response(&lsp.rx, 7);
    let name_detail = response["result"].as_array().unwrap().iter()
        .find(|i| i["label"] == "name")
        .and_then(|i| i["detail"].as_str())
        .unwrap_or("");
    assert_eq!(name_detail, "Str", "local's detail is its type: {name_detail}");
    send(
        &mut lsp.stdin,
        json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}),
    );
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-completion] Fields first after a struct's dot; types first where a
/// type is written, and offered with only whitespace before the cursor;
/// modules, sub-modules and a module's exported items on an `import` line.
#[test]
fn completion_offers_fields_types_and_modules() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_completion_more");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "\
import fs.
import ti

struct Point {
    x: Int,
    y: Int
}

fn norm(p: Point) -> Int {
    return p.
}

fn take(p: Po) -> Int {
    return 0
}

fn main() [Console] {
    
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let complete = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> Vec<(String, String)> {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/completion",
                "params": {"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}
            }),
        );
        let response = expect_response(&lsp.rx, id);
        response["result"]
            .as_array()
            .unwrap_or_else(|| panic!("no completion list: {response}"))
            .iter()
            .map(|i| (i["label"].as_str().unwrap().to_string(), i["sortText"].as_str().unwrap_or("").to_string()))
            .collect()
    };
    let rank = |items: &[(String, String)], l: &str| {
        items.iter().find(|(x, _)| x == l).map(|(_, r)| r.clone()).unwrap_or_else(|| panic!("no `{l}` in {items:?}"))
    };
    // `import fs.`: its sub-modules and exported items.
    let items = complete(&mut lsp, 2, 0, 10);
    rank(&items, "mem");
    rank(&items, "read_to_str");
    // `import ti`: top-level modules.
    let items = complete(&mut lsp, 3, 1, 9);
    rank(&items, "time");
    // `p.`: the fields first.
    let items = complete(&mut lsp, 4, 9, 13);
    assert!(rank(&items, "x") < rank(&items, "copy"), "{items:?}");
    // `p: Po`: types first.
    let items = complete(&mut lsp, 5, 12, 13);
    assert!(rank(&items, "Point") < rank(&items, "norm"), "{items:?}");
    // An empty line in a body: fns and types both.
    let items = complete(&mut lsp, 6, 17, 4);
    rank(&items, "norm");
    rank(&items, "Point");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-completion] Inside a struct literal's braces, completion offers the
/// struct's remaining fields — excluding ones already written — scoped to
/// the innermost enclosing literal when literals nest.
#[test]
fn completion_offers_struct_literal_remaining_fields() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_completion_struct_lit");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "\
struct Inner {
    x: Int,
    y: Int
}

struct Person {
    name: Str,
    age: Int,
    nickname: Str?,
    inner: Inner
}

fn make_person() -> Person {
    return Person {
        name: \"a\",
        
    }
}

fn make_nested() -> Person {
    return Person {
        name: \"a\",
        inner: Inner {
            x: 1,
            
        }
    }
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let complete = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> Vec<(String, String)> {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/completion",
                "params": {"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}
            }),
        );
        let response = expect_response(&lsp.rx, id);
        response["result"]
            .as_array()
            .unwrap_or_else(|| panic!("no completion list: {response}"))
            .iter()
            .map(|i| (i["label"].as_str().unwrap().to_string(), i["sortText"].as_str().unwrap_or("").to_string()))
            .collect()
    };
    let rank = |items: &[(String, String)], l: &str| {
        items.iter().find(|(x, _)| x == l).map(|(_, r)| r.clone()).unwrap_or_else(|| panic!("no `{l}` in {items:?}"))
    };
    let has = |items: &[(String, String)], l: &str| items.iter().any(|(x, _)| x == l);
    // The blank line inside `Person { name: "a", | }`: remaining fields only.
    let items = complete(&mut lsp, 2, 15, 8);
    rank(&items, "age");
    rank(&items, "nickname");
    rank(&items, "inner");
    assert!(!has(&items, "name"), "`name` is already written: {items:?}");
    // The blank line inside the nested `Inner { x: 1, | }`: scoped to
    // `Inner`'s remaining fields, not `Person`'s.
    let items = complete(&mut lsp, 3, 24, 12);
    rank(&items, "y");
    assert!(!has(&items, "x"), "`x` is already written: {items:?}");
    assert!(!has(&items, "age"), "outer `Person` fields must not leak in: {items:?}");
    assert!(!has(&items, "name"), "outer `Person` fields must not leak in: {items:?}");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-completion] Dot-completion still finds the receiver's fields when
/// the cursor sits *before* already-typed trailing text — `if arg.` with
/// `description !is None {` already written after the dot — not just at the
/// end of the buffer. Before this, the probe buffer trimmed only backward
/// from the cursor, so patching glued the receiver straight onto the
/// trailing identifier (`arg` + `description` → `argdescription`) and the
/// receiver's type lookup found nothing.
#[test]
fn completion_after_a_dot_works_before_already_typed_trailing_text() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_completion_mid_token");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "\
struct Arg<T> {
    short_name: Str?,
    description: Str?
}

fn name<T>(arg: Arg<T>) -> Str {
    if arg.description !is None {
        return \"x\"
    }
    return \"y\"
}

fn take<T>(a: Arg<T>) -> None {}

fn call_it<T>(arg: Arg<T>) -> None {
    take(arg.)
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let complete = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> Vec<String> {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/completion",
                "params": {"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}
            }),
        );
        let response = expect_response(&lsp.rx, id);
        response["result"]
            .as_array()
            .unwrap_or_else(|| panic!("no completion list: {response}"))
            .iter()
            .map(|i| i["label"].as_str().unwrap().to_string())
            .collect()
    };
    // `if arg.` — cursor right after the dot, `description !is None {`
    // already written after it.
    let items = complete(&mut lsp, 2, 6, 11);
    assert!(items.contains(&"short_name".to_string()), "expected `short_name`: {items:?}");
    assert!(items.contains(&"description".to_string()), "expected `description`: {items:?}");
    // `take(arg.)` — cursor right after the dot, a `)` already written
    // right after it.
    let items = complete(&mut lsp, 3, 15, 13);
    assert!(items.contains(&"short_name".to_string()), "expected `short_name`: {items:?}");
    assert!(items.contains(&"description".to_string()), "expected `description`: {items:?}");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-rename] Renaming a struct field reaches a struct-literal label
/// (`Arg { short_name: … }`), not just a dot-access read — and still does
/// when another module declares a struct of the same name, which forces the
/// struct's internal type key to carry a module suffix (`Arg§main`). Before
/// this, `record_field_ref` looked up the rename's declaration site by that
/// suffixed key in a table keyed by the plain name only, so the lookup
/// missed — silently recording no reference — for any struct literal, and
/// for a dot-access read too once the struct's name was not globally unique
/// (defect reported 2026-10-20).
#[test]
fn rename_reaches_a_struct_literal_field_and_survives_a_name_clash() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_rename_field_clash");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Two files each declare their own `Arg`, forcing the second's internal
    // key to carry a module suffix.
    let source_a = "\
struct Arg<T> {
    short_name: Str?
    long_name: Str?
}

fn go() -> None {
    let a = Arg {
        short_name: \"-p\",
        long_name: \"--print\"
    }
}
";
    let source_b = "\
struct Arg<T> {
    short_name: Str?
}

fn read(a: Arg<Int>) -> Str? {
    return a.short_name
}
";
    std::fs::write(root.join("a.sv"), source_a).unwrap();
    std::fs::write(root.join("b.sv"), source_b).unwrap();
    let mut lsp = start(&root);
    let uri_a = format!("file://{}", root.join("a.sv").display());
    let uri_b = format!("file://{}", root.join("b.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri_a, "languageId": "salvo", "version": 1, "text": source_a}}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri_b, "languageId": "salvo", "version": 1, "text": source_b}}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    // Rename from the struct-literal USE in a.sv, line 7 (`short_name: "-p"`).
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "textDocument/rename",
            "params": {"textDocument": {"uri": uri_a}, "position": {"line": 7, "character": 12}, "newName": "brief_name"}
        }),
    );
    let response = expect_response(&lsp.rx, 2);
    let edits = response["result"]["changes"][&uri_a]
        .as_array()
        .unwrap_or_else(|| panic!("no edits for a.sv: {response}"));
    assert_eq!(edits.len(), 2, "expected the declaration and the literal label: {edits:?}");
    let new_texts: Vec<&str> = edits.iter().map(|e| e["newText"].as_str().unwrap()).collect();
    assert!(new_texts.iter().all(|t| *t == "brief_name"), "{new_texts:?}");
    // b.sv's own, separate `Arg` must be untouched.
    assert!(
        response["result"]["changes"].get(&uri_b).is_none(),
        "a different module's same-named struct must not be touched: {response}"
    );
    // Rename from the dot-access READ in b.sv, to confirm the same fix
    // (plain-name lookup under a module-suffixed key) covers that path too.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "id": 3, "method": "textDocument/rename",
            "params": {"textDocument": {"uri": uri_b}, "position": {"line": 5, "character": 14}, "newName": "brief_name"}
        }),
    );
    let response = expect_response(&lsp.rx, 3);
    let edits = response["result"]["changes"][&uri_b]
        .as_array()
        .unwrap_or_else(|| panic!("no edits for b.sv: {response}"));
    assert_eq!(edits.len(), 2, "expected the declaration and the dot-access read: {edits:?}");
    assert!(
        response["result"]["changes"].get(&uri_a).is_none(),
        "a different module's same-named struct must not be touched: {response}"
    );
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-comment-continuation] Pressing Enter inside a `// LABEL: text`
/// comment aligns the continuation line to the paragraph start — right
/// after `LABEL: ` — not just to the client's own static `// `; a plain
/// `//` comment with no label keeps the ordinary `// ` continuation (no
/// edit at all, since the client's own rule already got it right).
#[test]
fn on_type_formatting_aligns_a_labeled_comments_continuation() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_on_type_format");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "\
fn go() -> None {
    // TODO: Should be able to specify foo here for any arg
    // 
    // a plain continuing comment that wraps to the next line
    // 
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let on_type_format = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> serde_json::Value {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/onTypeFormatting",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": line, "character": character},
                    "ch": "\n",
                    "options": {"tabSize": 4, "insertSpaces": true}
                }
            }),
        );
        expect_response(&lsp.rx, id)["result"].clone()
    };
    // Right after the client's own static `    // ` on the line below the
    // labeled TODO (line 2, 7 chars: 4-space indent + `// `): aligns to
    // where `Should` starts on line 1 (column 13).
    let result = on_type_format(&mut lsp, 2, 2, 7);
    let edits = result.as_array().unwrap_or_else(|| panic!("expected an edit: {result}"));
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0]["newText"], json!("    //       "), "{edits:?}");
    // The same position after a plain, label-less comment (line 4): no
    // edit — the client's static `// ` is already correct.
    let result = on_type_format(&mut lsp, 3, 4, 7);
    assert_eq!(result, json!(null), "a plain comment needs no realignment: {result}");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-comment-continuation] [doc-tags] A doc tag continues under its text
/// — after `@param`'s name — and an indented continuation line keeps its
/// indentation, so a tag runs over as many lines as it needs.
#[test]
fn on_type_formatting_aligns_doc_tag_continuations() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_on_type_format_tags");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "\
// @param line  the command line, quotes
// 
//              and escapes
// 
// @returns the words
// 
fn go(line: Str) -> Str {
    return line
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": source
            }}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let on_type_format = |lsp: &mut Lsp, id: i64, line: u32| -> serde_json::Value {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/onTypeFormatting",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": line, "character": 3},
                    "ch": "\n",
                    "options": {"tabSize": 4, "insertSpaces": true}
                }
            }),
        );
        expect_response(&lsp.rx, id)["result"].clone()
    };
    for (id, line, want) in [(2, 1, "//              "), (3, 3, "//              "), (4, 5, "//          ")] {
        let result = on_type_format(&mut lsp, id, line);
        let edits = result.as_array().unwrap_or_else(|| panic!("expected an edit on line {line}: {result}"));
        assert_eq!(edits[0]["newText"], json!(want), "line {line}: {edits:?}");
    }
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-completion] Overloads of one name collapse to a single bare-prefix
/// entry; a `//` comment offers no code completion, except inside a `[…]`
/// reference where it offers code symbols; and the assertion builtins appear
/// by prefix.
#[test]
fn completion_overloads_comments_and_builtins() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_completion_misc");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // `pick` has two overloads; the comment lines exercise the `[…]` ref and
    // plain-prose cases.
    let source = "\
struct Widget { size: Int }

fn pick(a: Int) -> Int => a {
    return a
}

fn pick(a: Int, b: Int) -> Int => a, b {
    return a + b
}

// A reference to [Wi
// Just some prose about wi
fn main() [Console] {
    pi
    un
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": source}}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let complete = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> Vec<(String, String)> {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/completion",
                "params": {"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}
            }),
        );
        let response = expect_response(&lsp.rx, id);
        response["result"].as_array().unwrap_or_else(|| panic!("no completion list: {response}"))
            .iter()
            .map(|i| (i["label"].as_str().unwrap().to_string(), i["detail"].as_str().unwrap_or("").to_string()))
            .collect()
    };
    // [lsp-completion] `pi` → one `pick` entry (not one per overload), its
    // detail noting the count.
    let items = complete(&mut lsp, 2, 13, 6);
    let picks: Vec<&(String, String)> = items.iter().filter(|(l, _)| l == "pick").collect();
    assert_eq!(picks.len(), 1, "one `pick` entry: {items:?}");
    assert!(picks[0].1.contains("2 overloads"), "detail notes overloads: {picks:?}");
    // [lsp-completion] `un` on a body line → the assertion builtins by prefix.
    let items = complete(&mut lsp, 3, 14, 6);
    let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.contains(&"unreachable!") && labels.contains(&"assert!"), "builtins by prefix: {labels:?}");
    // [doc-symbol-ref] Inside `[…]` in a comment: code symbols are offered.
    let items = complete(&mut lsp, 4, 10, 20);
    let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.contains(&"Widget") && labels.contains(&"pick"), "refs in `[`: {labels:?}");
    // [lsp-completion] Right after the `[` (empty prefix) the full list is
    // offered too — the scenario the `[` trigger character creates, so a
    // reference auto-suggests as it is opened (user request 2026-10-14).
    let items = complete(&mut lsp, 6, 10, 19);
    let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
    assert!(labels.contains(&"Widget") && labels.contains(&"pick"), "refs right after `[`: {labels:?}");
    // Plain prose in a comment (no `[`): nothing is offered.
    let items = complete(&mut lsp, 5, 11, 25);
    assert!(items.is_empty(), "no completion in comment prose: {items:?}");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-signature-help] `textDocument/signatureHelp` inside a call's argument
/// list returns one signature per overload, with the active parameter (the
/// comma count before the cursor) marked.
#[test]
fn signature_help_offers_overloads_and_active_parameter() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_sighelp");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "\
fn pick(a: Int) -> Int => a {
    return a
}

fn pick(a: Int, b: Int) -> Int => a, b {
    return a + b
}

fn main() [Console] {
    let x = pick(1, 2)
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": source}}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    let sig_help = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> serde_json::Value {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/signatureHelp",
                "params": {"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}
            }),
        );
        expect_response(&lsp.rx, id)["result"].clone()
    };
    // Cursor just after `pick(` (line 9, `    let x = pick(1, 2)` — `(` is at
    // column 16, so column 17 is the first argument).
    let result = sig_help(&mut lsp, 2, 9, 17);
    let sigs = result["signatures"].as_array().expect("signatures");
    // Both overloads of `pick` are offered.
    assert_eq!(sigs.len(), 2, "two overloads: {result}");
    let labels: Vec<&str> = sigs.iter().map(|s| s["label"].as_str().unwrap()).collect();
    assert!(labels.iter().any(|l| l.contains("pick(a: Int)")), "{labels:?}");
    assert!(labels.iter().any(|l| l.contains("pick(a: Int, b: Int)")), "{labels:?}");
    // At the first argument, the active parameter is 0.
    assert_eq!(result["activeParameter"].as_u64(), Some(0), "{result}");
    // After the comma (column 20, the second argument), it is 1.
    let result = sig_help(&mut lsp, 3, 9, 20);
    assert_eq!(result["activeParameter"].as_u64(), Some(1), "{result}");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-completion] [struct-opaque] After the dot on a value of an opaque
/// struct, the fields are offered in the declaring module and not elsewhere.
#[test]
fn completion_hides_an_opaque_structs_fields_outside_its_module() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_completion_opaque");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let bag = "\
export opaque struct Bag {
    secret: Int
}

export fn make() [] -> Bag {
    return Bag { secret: 1 }
}

fn inside(b: Bag) -> Int {
    return b.
}
";
    let main = "\
import bag.Bag

fn outside(b: Bag) -> Int {
    return b.
}

fn main() [Console] {
}
";
    std::fs::write(root.join("bag.sv"), bag).unwrap();
    std::fs::write(root.join("main.sv"), main).unwrap();
    let mut lsp = start(&root);
    let open = |lsp: &mut Lsp, file: &str, text: &str| {
        let uri = format!("file://{}", root.join(file).display());
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": text}}
            }),
        );
        let _ = expect_diagnostics(&lsp.rx);
        uri
    };
    let bag_uri = open(&mut lsp, "bag.sv", bag);
    let main_uri = open(&mut lsp, "main.sv", main);
    let complete = |lsp: &mut Lsp, id: i64, uri: &str, line: u32, character: u32| -> Vec<String> {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/completion",
                "params": {"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}
            }),
        );
        let response = expect_response(&lsp.rx, id);
        response["result"]
            .as_array()
            .unwrap_or_else(|| panic!("no completion list: {response}"))
            .iter()
            .map(|i| i["label"].as_str().unwrap().to_string())
            .collect()
    };
    let inside = complete(&mut lsp, 2, &bag_uri, 9, 13);
    assert!(inside.iter().any(|l| l == "secret"), "{inside:?}");
    let outside = complete(&mut lsp, 3, &main_uri, 3, 13);
    assert!(!outside.iter().any(|l| l == "secret"), "{outside:?}");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-hover-iter-fn] [lsp-definition] An `iter fn` hovers as written, at
/// its declaration and at a call; a field reached through a chain inside a
/// dot-call's argument jumps to its declaration.
#[test]
fn iter_fns_hover_as_written_and_chained_fields_resolve() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_iter_hover");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "\
iter fn upto(n: Int) -> Emitted Int | Finished {
    state {
        at: Int = 0
    }
    if at >= n {
        return finished()
    }
    at = at + 1
    return emitted(at)
}

struct Inner {
    stamp: Int
}

struct Outer {
    inner: Inner
}

fn show(o: Outer) [Console] {
    for x in upto(2) {
        println(\"${x}\")
    }
    println(o.inner.stamp.to_str())
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": source}}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["diagnostics"].as_array().unwrap().len(), 0, "diagnostics: {params}");
    let at_decl = hover(&mut lsp, 2, &uri, 0, 9);
    assert!(at_decl.contains("iter fn upto(n: Int) -> Emitted Int | Finished"), "{at_decl}");
    assert!(!at_decl.contains("__Iter"), "{at_decl}");
    let at_call = hover(&mut lsp, 3, &uri, 20, 14);
    assert!(at_call.contains("iter fn upto(n: Int)"), "{at_call}");
    // `stamp` in `println(o.inner.stamp.to_str())`: line 23, column 20.
    let def = definition(&mut lsp, 4, &uri, 23, 21);
    assert_eq!(def["range"]["start"]["line"], 12, "{def}");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-analysis-reuse] Requests between two edits answer from one analysis,
/// and an edit drops it: a hover after a change sees the changed buffer, not
/// the analysis the previous hover was answered from.
#[test]
fn an_edit_drops_the_reused_analysis() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_reuse");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let first = "fn main() [] {\n    let x = 42\n    let _ = x\n}\n";
    std::fs::write(root.join("main.sv"), first).unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": first}}
        }),
    );
    let _ = expect_diagnostics(&lsp.rx);
    // Twice from one analysis: the same answer.
    assert!(hover(&mut lsp, 2, &uri, 1, 8).contains("Int"));
    assert!(hover(&mut lsp, 3, &uri, 1, 8).contains("Int"));
    let second = "fn main() [] {\n    let x = \"s\"\n    let _ = x\n}\n";
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": second}]}
        }),
    );
    let after = hover(&mut lsp, 4, &uri, 1, 8);
    assert!(after.contains("Str"), "the hover after an edit read a stale analysis: {after}");
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-debounce] A burst of edits collapses into one analysis of the final
/// buffer. The server waits out a short quiet period before analysing and
/// absorbs messages that arrive inside it, so intermediate (here broken)
/// states never get their own publish: the only diagnostics that come back
/// describe the last edit. This is what stops the editor from "working
/// through previous versions in order" when the whole-program check is slower
/// than typing.
#[test]
fn a_burst_of_edits_publishes_only_the_final_state() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_debounce");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.sv"), "fn main() [] {\n}\n").unwrap();

    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());

    // Open clean; expect a clean publish.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1,
                "text": "fn main() [] {\n}\n"
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["diagnostics"].as_array().unwrap().len(), 0, "{params}");

    // Fire a rapid burst of changes with no reads between them. Every
    // intermediate text is broken (a type mismatch); only the final one is
    // clean. Sent back-to-back, they should all land inside one debounce
    // window and coalesce.
    for v in 2..=7 {
        let broken = format!(
            "fn main() [] {{\n    let x: Int = \"v{v}\"\n}}\n"
        );
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "method": "textDocument/didChange",
                "params": {
                    "textDocument": {"uri": uri, "version": v},
                    "contentChanges": [{"text": broken}]
                }
            }),
        );
    }
    // Final edit: clean.
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 8},
                "contentChanges": [{"text": "fn main() [] {\n    let x: Int = 42\n    let _ = x\n}\n"}]
            }
        }),
    );

    // The next publish must be clean: the burst coalesced into one analysis
    // of the final buffer. If the server analysed each intermediate edit it
    // would publish the broken states first, and this would see one of them.
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(
        params["diagnostics"].as_array().unwrap().len(),
        0,
        "the burst should have coalesced to the final clean state: {params}"
    );

    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [lsp-slow-dump] A slow analysis pass dumps the exact buffer it was
/// analysing to a single fixed path, overwritten each time, so a transient
/// (often unsaved) editor state that pegs the server can be reproduced. Here
/// `SALVO_LSP_SLOW_MS=0` makes every pass "slow", and `SALVO_LSP_SLOW_DIR`
/// pins the dump to a scratch dir; opening one document is enough to force a
/// publish-diagnostics pass, after which the dump must hold that document's
/// source verbatim.
#[test]
fn a_slow_pass_dumps_the_analysed_buffer() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_slow_dump");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let dump_dir = root.join("dump");
    std::fs::create_dir_all(&dump_dir).unwrap();
    // On disk: a trivial file. The overlay below is what must be dumped.
    std::fs::write(root.join("main.sv"), "fn main() [] {\n}\n").unwrap();

    let mut lsp = start_with_env(
        &root,
        &[
            ("SALVO_LSP_SLOW_MS", "0"),
            ("SALVO_LSP_SLOW_DIR", dump_dir.to_str().unwrap()),
        ],
    );
    let uri = format!("file://{}/main.sv", root.display());
    // The unsaved buffer differs from disk — this exact text must be dumped.
    let buffer = "fn main() [use] {\n    use StdConsole()\n    println(\"dump me\")\n}\n";
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": buffer}}
        }),
    );
    // Wait for the analysis pass (which, at threshold 0, dumps).
    let _ = expect_diagnostics(&lsp.rx);

    let sv = dump_dir.join("salvo-lsp-slow.sv");
    let txt = dump_dir.join("salvo-lsp-slow.txt");
    // The dump is written during the pass; the publish follows it, but give a
    // brief grace in case of filesystem lag.
    for _ in 0..50 {
        if sv.is_file() && txt.is_file() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let dumped = std::fs::read_to_string(&sv).expect("slow-pass .sv dump missing");
    assert_eq!(dumped, buffer, "the dump must be the analysed overlay verbatim");
    let meta = std::fs::read_to_string(&txt).expect("slow-pass .txt header missing");
    assert!(meta.contains("slow-pass dump"), "meta header: {meta}");
    assert!(meta.contains("main.sv"), "meta names the document: {meta}");

    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().expect("failed to wait for salvo lsp");
}

/// [fn-type-named] A type-named constructor hovers as its **readable** type
/// (`fn Mut Map<K, V canbe linear>…`), not its canonical key (`Mut_Map`).
/// Regression for the hover showing the mangled backend identifier after
/// collection constructors became type-named functions.
#[test]
fn hovering_a_type_named_constructor_shows_the_readable_name() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_type_named_hover");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.sv"), "fn main() [] {\n}\n").unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}/main.sv", root.display());
    // Line index 2 is the `tally` line; the callee `Mut Map` begins at col 35.
    let buf = "fn main() [use] {\n    use StdConsole()\n    let tally: Mut Map<Str, Int> = Mut Map(\"a\": 1)\n    let _ = tally\n}\n";
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": buf}}
    }));
    let _ = expect_diagnostics(&lsp.rx);
    // Column 36 = inside the `Mut` of the `Mut Map(...)` callee.
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
        "params": {"textDocument": {"uri": uri}, "position": {"line": 2, "character": 36}}
    }));
    let r = expect_response(&lsp.rx, 2);
    let hover = r["result"]["contents"]["value"].as_str().unwrap_or("");
    assert!(
        hover.contains("fn Mut Map<"),
        "constructor hover should read `fn Mut Map<…>`, got: {hover}"
    );
    assert!(
        !hover.contains("Mut_Map"),
        "constructor hover must not leak the canonical key `Mut_Map`: {hover}"
    );
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().unwrap();
}


/// [lsp-hover-overloads] [iter-fn] Hovering a `next` must not list a
/// compiler-generated iterator-struct `next` (`next(__p: Mut __Iter_…)`) in
/// the "Also visible under this name" section — that struct is unspellable
/// backend detail. Regression for the generated overload leaking into hover.
#[test]
fn hover_next_hides_generated_iterator_overloads() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_next_overloads");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.sv"), "fn main() [] {\n}\n").unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}/main.sv", root.display());
    // An `iter fn` generates a `__Iter_counts_…` struct and its `next`; a
    // user struct with its own `next` gives a second, real overload. Hovering
    // the user `next` must show only real overloads.
    let buf = concat!(
        "struct Pair<T> : Yield<self, T> canbe Mut {\n    items: List<T>,\n    at: Int\n}\n\n",
        "fn next<T>(p: Mut Pair<T>) [] -> Emitted T | Finished => p: Mut {\n    return finished()\n}\n\n",
        "iter fn counts(n: Int) -> Emitted Int | Finished {\n    state { at: Int = 0 }\n    if at >= n { return finished() }\n    at = at + 1\n    return emitted(at)\n}\n\n",
        "fn main() [] {\n}\n",
    );
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": buf}}
    }));
    let _ = expect_diagnostics(&lsp.rx);
    // Hover the user `next` declaration name: line index 5 (`fn next<T>...`),
    // the `next` begins at column 3.
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
        "params": {"textDocument": {"uri": uri}, "position": {"line": 5, "character": 4}}
    }));
    let r = expect_response(&lsp.rx, 2);
    let hover = r["result"]["contents"]["value"].as_str().unwrap_or("");
    assert!(
        !hover.contains("__Iter_"),
        "hover must not leak the generated iterator struct: {hover}"
    );
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().unwrap();
}

/// [lsp-definition] Go-to-definition on a call to a locally-declared function
/// lands on that local declaration — the scenario behind a report of `notices`
/// jumping into std. (A one-line editor/file desync, not a resolution bug, was
/// the actual cause there; this guards the resolution path regardless.)
#[test]
fn goto_definition_lands_on_the_local_declaration() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_goto_local");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.sv"), "fn main() [] {\n}\n").unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}/main.sv", root.display());
    // The call is a few lines above the declaration, as in the report.
    let buf = concat!(
        "fn use_it() -> Int {\n    return describe(5)\n}\n\n",
        "fn describe(n: Int) [] -> Int => n {\n    return n\n}\n",
    );
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": buf}}
    }));
    let _ = expect_diagnostics(&lsp.rx);
    // Line index 1 (`    return describe(5)`), `describe` call at column 11.
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": {"textDocument": {"uri": uri}, "position": {"line": 1, "character": 12}}
    }));
    let r = expect_response(&lsp.rx, 2);
    let result = &r["result"];
    assert_eq!(
        result["uri"].as_str(), Some(uri.as_str()),
        "goto should land in the local file: {result}"
    );
    // The local `fn describe` is on line index 4.
    assert_eq!(
        result["range"]["start"]["line"].as_u64(), Some(4),
        "goto should land on the local `fn describe` declaration (line 4): {result}"
    );
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().unwrap();
}

/// [lsp-std-source] Go-to-definition INTO std opens a read-only temp copy,
/// and go-to-definition FROM there resolves to another std definition (also a
/// temp copy) — hover/goto work inside std, not just at its boundary.
#[test]
fn goto_definition_works_inside_std() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_goto_std");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.sv"), "fn main() [] {\n}\n").unwrap();
    let mut lsp = start(&root);
    let uri = format!("file://{}/main.sv", root.display());
    // A `List` and a call to a std fn with a clear single definition.
    let buf = "fn use_it() -> Int {\n    let xs = List(1, 2, 3)\n    return size(xs)\n}\n";
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": buf}}
    }));
    let _ = expect_diagnostics(&lsp.rx);
    // Goto on `size` (line index 2, `    return size(xs)` — `size` at col 11)
    // → into std.
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": {"textDocument": {"uri": uri}, "position": {"line": 2, "character": 12}}
    }));
    let into_std = expect_response(&lsp.rx, 2);
    let std_uri = into_std["result"]["uri"].as_str().expect("goto into std should yield a uri").to_string();
    assert!(std_uri.contains("salvo-std-"), "goto `size` should land in a std temp copy: {into_std}");
    let std_line = into_std["result"]["range"]["start"]["line"].as_u64().unwrap();
    let std_char = into_std["result"]["range"]["start"]["character"].as_u64().unwrap();

    // Open that std file and goto-definition on the `size` declaration name
    // itself — it should resolve (to the same declaration), proving position
    // features are live inside std. Any non-null result at a std uri is the
    // proof; a dead `analyze_for` (the old behaviour) returned null.
    let std_text = {
        let path = std_uri.strip_prefix("file://").unwrap();
        std::fs::read_to_string(path).expect("the std temp copy must exist")
    };
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": std_uri, "languageId": "salvo", "version": 1, "text": std_text}}
    }));
    let _ = expect_diagnostics(&lsp.rx);
    // Hover the std declaration — a live analysis gives a signature; the dead
    // path gave nothing.
    send(&mut lsp.stdin, json!({
        "jsonrpc": "2.0", "id": 3, "method": "textDocument/hover",
        "params": {"textDocument": {"uri": std_uri}, "position": {"line": std_line, "character": std_char}}
    }));
    let hov = expect_response(&lsp.rx, 3);
    assert!(
        hov["result"]["contents"]["value"].as_str().is_some_and(|v| v.contains("size")),
        "hover inside std should describe `size`, got: {hov}"
    );
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().unwrap();
}

// [lsp-definition] A namespace member's type reference hovers and resolves
// to its declaration even when it is never reached through a struct literal
// or call argument — a bare union arm (`type Type = Option.Int | ...`) and a
// never-instantiated struct's field type (`type: Option.Type`) are both
// `Type::Named` nodes validated only by `Checker::validate_type`, which used
// to skip `lower_base_ref` (and so never recorded a `def_refs` entry) for an
// unqualified reference — only a qualified one (`Mut Option.Int`) reached
// it. Regression for the gap found 2026-10-19 via `namespace Option { ... }`
// sharing its name with a struct of the same name [namespace].
#[test]
fn hover_resolves_a_namespace_members_type_reference() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_namespace_hover");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let mut lsp = start(&root);
    let uri = format!("file://{}", root.join("main.sv").display());
    // Lines (0-based):
    // 0 namespace Option {
    // 1     struct Int {}
    // 2     struct Str {}
    // 3     struct Bool {}
    // 4     type Type = Option.Int | Option.Str | Option.Bool
    // 5 }
    // 6
    // 7 struct Option {
    // 8     short_name: Str?
    // 9     long_name: Str?
    // 10     description: Str
    // 11     type: Option.Type
    // 12 }
    let text = "namespace Option {\n\
                \x20   struct Int {}\n\
                \x20   struct Str {}\n\
                \x20   struct Bool {}\n\
                \x20   type Type = Option.Int | Option.Str | Option.Bool\n\
                }\n\
                \n\
                struct Option {\n\
                \x20   short_name: Str?\n\
                \x20   long_name: Str?\n\
                \x20   description: Str\n\
                \x20   type: Option.Type\n\
                }\n";
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "salvo", "version": 1, "text": text
            }}
        }),
    );
    let params = expect_diagnostics(&lsp.rx);
    assert_eq!(params["diagnostics"].as_array().unwrap().len(), 0, "unexpected diagnostics: {params}");

    // The bare union arms on line 4, never reached by a struct literal.
    let at_int_arm = hover(&mut lsp, 10, &uri, 4, 17);
    assert!(at_int_arm.contains("Int"), "expected Option.Int's hover, got: {at_int_arm}");
    let at_str_arm = hover(&mut lsp, 11, &uri, 4, 30);
    assert!(at_str_arm.contains("Str"), "expected Option.Str's hover, got: {at_str_arm}");
    let at_bool_arm = hover(&mut lsp, 12, &uri, 4, 43);
    assert!(at_bool_arm.contains("Bool"), "expected Option.Bool's hover, got: {at_bool_arm}");

    // The never-instantiated struct's field type on line 11.
    let at_field_ty = hover(&mut lsp, 13, &uri, 11, 11);
    assert!(at_field_ty.contains("Type"), "expected Option.Type's hover, got: {at_field_ty}");

    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
    expect_response(&lsp.rx, 99);
    send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
    lsp.child.wait().unwrap();
}

/// [lsp-completion] [when-exhaustive] Between the arms of a `when` — on a blank
/// line, or over a half-written `is …` head — completion offers exactly the
/// arms not yet written, one item each plus one writing them all; the same
/// for a comptime `[when f.name]`, `[when f.kind]` and `[when f.type]` (whose
/// catch-all `is *` is offered beside the uncovered types). A client with
/// snippet support gets the arm with the cursor placed in its body (user
/// request 2026-10-20).
#[test]
fn completion_offers_the_missing_when_arms() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lsp_completion_when_arms");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "\
struct Circle { r: Int }
struct Square { s: Int }
type Shape = Circle | Square | None

fn area(s: Shape) -> Int {
    when s {
        is Circle { return 1 }
        
    }
}

fn pick(s: Shape) -> None {
    when t = s {
        is S
    }
}

struct P { a: Int, b: Str, c: Int? }

comptime fn names(p: P) [] -> Int => p {
    [for f in P.fields] {
        [when f.name] {
            is \"a\" {}
            
        }
        [when f.kind] {
            is Struct {}
            
        }
        [when f.type] {
            is Int {}
            
        }
    }
    return 0
}
";
    std::fs::write(root.join("main.sv"), source).unwrap();
    let uri = format!("file://{}", root.join("main.sv").display());
    // (label, newText, textEdit start/end character) per item.
    type Item = (String, String, u64, u64);
    let complete = |lsp: &mut Lsp, id: i64, line: u32, character: u32| -> Vec<Item> {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "id": id, "method": "textDocument/completion",
                "params": {"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}
            }),
        );
        let response = expect_response(&lsp.rx, id);
        response["result"]
            .as_array()
            .unwrap_or_else(|| panic!("no completion list: {response}"))
            .iter()
            .map(|i| {
                (
                    i["label"].as_str().unwrap().to_string(),
                    i["textEdit"]["newText"].as_str().unwrap_or("").to_string(),
                    i["textEdit"]["range"]["start"]["character"].as_u64().unwrap_or(0),
                    i["textEdit"]["range"]["end"]["character"].as_u64().unwrap_or(0),
                )
            })
            .collect()
    };
    let labels = |items: &[Item]| items.iter().map(|i| i.0.clone()).collect::<Vec<_>>();
    let open = |lsp: &mut Lsp| {
        send(
            &mut lsp.stdin,
            json!({
                "jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": source}}
            }),
        );
        let _ = expect_diagnostics(&lsp.rx);
    };
    let stop = |mut lsp: Lsp| {
        send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown", "params": null}));
        expect_response(&lsp.rx, 99);
        send(&mut lsp.stdin, json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
        lsp.child.wait().expect("failed to wait for salvo lsp");
    };

    let mut lsp = start(&root);
    open(&mut lsp);
    // A blank line after `is Circle`: only the two arms left, and nothing else.
    let items = complete(&mut lsp, 2, 7, 8);
    assert_eq!(labels(&items), ["is Square", "is None", "is … (all 2 missing arms)"], "{items:?}");
    let square = items.iter().find(|i| i.0 == "is Square").unwrap();
    assert_eq!((square.1.as_str(), square.2, square.3), ("is Square {}", 8, 8), "{items:?}");
    let all = items.iter().find(|i| i.0.starts_with("is …")).unwrap();
    assert_eq!(all.1, "is Square {}\n        is None {}", "{items:?}");
    // A half-written `is S` in the binding form: every arm, replacing the head.
    let items = complete(&mut lsp, 3, 13, 12);
    assert_eq!(labels(&items), ["is Circle", "is Square", "is None", "is … (all 3 missing arms)"], "{items:?}");
    assert!(items.iter().all(|i| (i.2, i.3) == (8, 12)), "the edit replaces `is S`: {items:?}");
    // The comptime dispatches.
    assert_eq!(labels(&complete(&mut lsp, 4, 23, 12)), ["is \"b\"", "is \"c\"", "is … (all 2 missing arms)"]);
    assert_eq!(
        labels(&complete(&mut lsp, 5, 27, 12)),
        ["is Union", "is Tuple", "is FnType", "is Opaque", "is … (all 4 missing arms)"]
    );
    assert_eq!(labels(&complete(&mut lsp, 6, 31, 12)), ["is Str", "is Int?", "is *", "is … (all 2 missing arms)"]);
    stop(lsp);

    // With snippet support the cursor lands in the arm's body.
    let caps = json!({"textDocument": {"completion": {"completionItem": {"snippetSupport": true}}}});
    let mut lsp = start_with(&root, &[], caps);
    open(&mut lsp);
    let items = complete(&mut lsp, 7, 7, 8);
    let square = items.iter().find(|i| i.0 == "is Square").unwrap();
    assert_eq!(square.1, "is Square {\n            $0\n        }", "{items:?}");
    stop(lsp);
}
