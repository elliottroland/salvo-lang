# Debugging a runaway `salvo lsp` (100% CPU)

This file is for the next time `salvo` is found spinning at ~100% CPU in the
background — the suspected idle hot-loop in the language server. The code
investigation found no idle spin in our loop or in the `lsp-server` transport
(both block correctly), so a live spin means the process is either being *fed*
messages or stuck *inside* one analysis. The goal of these steps is to catch it
in the act and name the exact function or message.

Two aids are now built in:

- **`SALVO_LSP_TRACE`** — set in the environment, the server logs every message
  it dispatches, every debounce wake, and every diagnostics publish to stderr
  with a millisecond timestamp. Off (silent) by default.
- **A parent-death watchdog** — the server now exits on its own when the editor
  that spawned it dies (polls the parent PID once a second). This bounds the
  damage: a zombie server can no longer spin forever. If you still see a
  runaway, it is *not* an orphaned zombie — it is a live spin worth sampling.

---

## 1. Sample the live process (most decisive, zero setup)

While it is spinning, grab the PID and sample it. One stack trace of the hot
thread names the culprit.

```sh
# Find it (there may be several salvo processes; pick the one burning CPU).
ps aux | grep '[s]alvo' | sort -k3 -nr | head

# PID of the top CPU consumer, then sample for 5 seconds at 1ms:
PID=$(ps aux | grep '[s]alvo lsp' | sort -k3 -nr | awk 'NR==1{print $2}')
echo "sampling $PID"
sample "$PID" 5 1 -file /tmp/salvo-lsp-sample.txt
open -t /tmp/salvo-lsp-sample.txt      # or: less /tmp/salvo-lsp-sample.txt
```

Read the report top-down under the **heaviest thread**:

- Frames in `salvo_core::check` / `deduce` / `comptime` / `ir` → the spin is a
  **runaway analysis** on some buffer (e.g. the deduction fixpoint, or a
  comptime/iter expansion). Note the function at the base of the hot stack.
- Frames in `salvo_cli::lsp` (`main_loop`, `dispatch`, `publish_diagnostics`)
  with `recv`/`try_recv`/`recv_timeout` → the loop is being **fed messages**;
  go to step 2 to see what.
- Frames in `crossbeam_channel` / `std::sync` only, with the thread named
  `LspServerReader`/`LspServerWriter` → a transport-level issue; capture the
  trace in step 2 and note the editor/client in use.

If `sample` needs permission, you may be prompted for `sudo`; `sudo sample "$PID" 5 1`
works too.

### Alternative: a few manual backtraces with lldb

```sh
sudo lldb -p "$PID" \
  -o "thread backtrace all" -o "detach" -o "quit" 2>&1 | tee /tmp/salvo-lsp-bt.txt
```

Run it two or three times a second apart; the frame that is present every time
on the busy thread is the hot spot.

---

## 2. Reproduce with tracing on

Point your editor at a `salvo lsp` launched with the trace env var, or run it by
hand against a recorded session. The log goes to **stderr**, so most editors
surface it in the LSP server's output/log panel; otherwise redirect it.

```sh
# However your editor launches the server, add this to its environment:
SALVO_LSP_TRACE=1

# Or capture stderr to a file (editor-dependent; e.g. a wrapper script):
#   #!/bin/sh
#   exec env SALVO_LSP_TRACE=1 /path/to/salvo lsp 2>>/tmp/salvo-lsp.log
```

Then watch the log while **doing nothing** in the editor:

```sh
tail -f /tmp/salvo-lsp.log
```

Interpreting it:

- **The log keeps scrolling with no edits** → the loop is being fed. The
  message that repeats (`dispatch request …` / `dispatch notification …`) names
  the driver — a client resending a request, or a `publish_diagnostics` that the
  client answers with another request. Capture ~20 lines and note the method.
- **`publish_diagnostics: analysing …` appears and never a matching next line**
  → the analysis after it is the spin (combine with the step-1 sample).
- **The log is quiet** but CPU is pinned → the spin is on a thread that does not
  go through `dispatch` (transport or a background thread); rely on step 1.

---

## 3. What to capture for a bug report / fix

- The `sample` output file (`/tmp/salvo-lsp-sample.txt`) — the single most useful artifact.
- ~20 lines of `SALVO_LSP_TRACE` output around the spin, if reproducible.
- Which editor/LSP client, and whether it had just been quit/killed/reloaded.
- The open file(s) at the time — a runaway analysis is usually one specific buffer.

---

## Notes on what was already ruled out (so you don't re-check)

- `Server::main_loop` blocks on `recv()` when idle; the debounce inner loop only
  runs while `dirty` and waits `recv_timeout(120ms)`. Neither busy-waits, and
  both exit on channel disconnect. (`crates/salvo-cli/src/lsp.rs`)
- `lsp-server` 0.7's reader thread returns `Ok(None)` on a clean stdin EOF and
  ends, closing the channel — it does not loop on EOF.
- There are no other `loop {}` / `thread::spawn` / polling constructs in the CLI
  except the (non-spinning) test runner.

The remaining live-spin candidates are therefore (a) the client feeding the
server, or (b) a single analysis that does not terminate — which is exactly what
steps 1 and 2 distinguish.
