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
    assert_eq!(
        response["result"]["contents"]["value"].as_str(),
        Some("Int"),
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
        Some("fn scale(x: Int, factor: Int) -> [factor] Int"),
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
        Some("fn scale(x: Int, factor: Int) -> [factor] Int"),
        "unexpected hover: {response}"
    );

    // [fate-link] Hovering a fate-linked (derived) variable presents the
    // compiler qualifier: a bare `ReadOnly` on the type line, with the
    // qualifier's parameters (root, binding site) as detail below
    // (progressive disclosure).
    send(
        &mut lsp.stdin,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 4},
                "contentChanges": [{
                    "text": "fn read(s: Str) -> [s] None {\n}\n\nfn derived() {\n    let xs = \"hello\"\n    let ys = xs\n    read(ys)\n}\n"
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
    let contents = response["result"]["contents"]
        .as_array()
        .unwrap_or_else(|| panic!("hover contents not an array: {response}"));
    assert_eq!(
        contents[0]["value"].as_str(),
        Some("ReadOnly Str"),
        "unexpected hover type line: {response}"
    );
    let detail = contents[1].as_str().unwrap();
    assert!(
        detail.contains("Compiler qualifier `ReadOnly`")
            && detail.contains("shares fate with `xs` (bound at 6:")
            && detail.contains("`copy(...)`"),
        "unexpected hover detail: {detail}"
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
        "struct Point {\n    x: Int,\n    y: Int\n}\n",
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
    // 20 }
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
    assert_eq!(loc["range"]["start"], json!({"line": 0, "character": 7}));

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
