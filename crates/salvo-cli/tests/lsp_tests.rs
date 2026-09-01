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
