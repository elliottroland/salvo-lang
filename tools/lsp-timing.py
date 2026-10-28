#!/usr/bin/env python3
"""[cli-phase-timings] Time the language server's analysis of an edit.

Drives `salvo lsp` over stdio with SALVO_TIMINGS=1: opens <file>, then makes
[edits] edits, each inserting a space after the first occurrence of <needle>
(put it inside a fn body for a body-only edit), and prints the phase timings
of each edit's analysis. The file on disk is never written: the edits go to
the server's overlay.

  tools/lsp-timing.py target/debug/salvo examples/cluster/salvo \\
      examples/cluster/salvo/main.sv "fn main() [use, spawn] {" 3

The open's analysis builds the server's std cache, so every edit's numbers
are warm ones."""
import json, os, subprocess, sys, threading, time

binary, root, path, needle = sys.argv[1:5]
edits = int(sys.argv[5]) if len(sys.argv) > 5 else 3
root = os.path.abspath(root)
path = os.path.abspath(path)
uri = "file://" + path
text = open(path).read()

env = dict(os.environ, SALVO_TIMINGS="1")
p = subprocess.Popen([binary, "lsp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                     stderr=subprocess.PIPE, env=env)
errs = []
def drain_err():
    for line in p.stderr:
        errs.append((time.time(), line.decode(errors="replace").rstrip()))
threading.Thread(target=drain_err, daemon=True).start()

pubs = []
def drain_out():
    while True:
        header = b""
        while not header.endswith(b"\r\n\r\n"):
            c = p.stdout.read(1)
            if not c:
                return
            header += c
        n = int([h for h in header.split(b"\r\n") if h.lower().startswith(b"content-length")][0].split(b":")[1])
        body = json.loads(p.stdout.read(n))
        if body.get("method") == "textDocument/publishDiagnostics":
            pubs.append((time.time(), len(body["params"]["diagnostics"])))
threading.Thread(target=drain_out, daemon=True).start()

def send(msg):
    data = json.dumps(msg).encode()
    p.stdin.write(b"Content-Length: %d\r\n\r\n" % len(data) + data)
    p.stdin.flush()

send({"jsonrpc": "2.0", "id": 1, "method": "initialize",
      "params": {"rootUri": "file://" + root, "capabilities": {}}})
send({"jsonrpc": "2.0", "method": "initialized", "params": {}})
send({"jsonrpc": "2.0", "method": "textDocument/didOpen",
      "params": {"textDocument": {"uri": uri, "languageId": "salvo", "version": 1, "text": text}}})

def wait_pub(k, timeout=60):
    t0 = time.time()
    while len(pubs) < k and time.time() - t0 < timeout:
        time.sleep(0.01)

wait_pub(1)
time.sleep(0.3)
for i in range(edits):
    mark = len(errs)
    at = text.index(needle) + len(needle)
    text = text[:at] + " " + text[at:]
    send({"jsonrpc": "2.0", "method": "textDocument/didChange",
          "params": {"textDocument": {"uri": uri, "version": i + 2},
                     "contentChanges": [{"text": text}]}})
    wait_pub(i + 2)
    time.sleep(0.3)
    print(f"--- edit {i + 1}")
    for _, line in errs[mark:]:
        if line.startswith("timing:"):
            print(line)
send({"jsonrpc": "2.0", "id": 2, "method": "shutdown"})
send({"jsonrpc": "2.0", "method": "exit"})
p.wait(timeout=10)
