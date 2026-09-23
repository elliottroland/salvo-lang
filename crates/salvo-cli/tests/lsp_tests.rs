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
    let mut child = Command::new(env!("CARGO_BIN_EXE_salvo"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn salvo lsp");
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
                "capabilities": {}
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
        "fn main() [use] {\n    use StdOutConsole()\n    println(\"hello\")\n}\n",
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

fn narrow(value: Int | Str) -> Str {
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
    // How many lines we have written.
    count: Int = 0

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
    let at_use = hover(&mut lsp, 31, &uri, 26, 12);
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
    let at_call = hover(&mut lsp, 34, &uri, 26, 4);
    assert_eq!(at_call, at_decl, "member call hover differs: {at_call}");

    // Handler state and handler members, with `[symbol]` reaching the
    // handler's own names.
    let state = hover(&mut lsp, 35, &uri, 18, 4);
    assert!(
        state.starts_with("```salvo\ncount: Int = 0\n```")
            && state.contains("How many lines we have written.")
            && state.contains("Field of handler `StdLog` (state)."),
        "unexpected state hover: {state}"
    );
    let member = hover(&mut lsp, 36, &uri, 21, 8);
    assert!(
        member.starts_with("```salvo\nfn write(line: Str) -> None\n```")
            && member.contains("[`line`](file://")
            && member.contains("[`count`](file://")
            && member.contains("Member of handler `StdLog`."),
        "unexpected handler member hover: {member}"
    );

    // [lsp-definition] A field access also navigates to its declaration.
    let location = definition(&mut lsp, 37, &uri, 26, 12);
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

/// [qual-refn-docs] Merged documentation: a refinement is written somewhere
/// else entirely — in the qualifier that owns the claim — so hovering the
/// refined function is the only place a reader can learn what it
/// additionally establishes *here*. The section lists the effective
/// deduction, where it came from, and the refinement's own doc comment; a
/// suppressed conflict [qual-refn-conflict] is shown too, since the type
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
    let xs: Mut List<Int> = mut_list_of()
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

fn describe(s: Str) -> Str {
    return s
}

// Two members of one effect sharing a name, which is the shape `read_to` has
// in `core.fs` [effect-member-overload].
effect Store {
    fn keep(n: Int) -> Bool => n
    fn keep(s: Str) -> Bool => s
}

fn main() [use] -> None {
    use StdOutConsole()
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
