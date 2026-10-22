// VS Code extension for Salvo: starts `salvo lsp` (LSP over stdio) and
// wires it to .sv documents. The binary path is configurable via
// `salvo.serverPath` so a freshly rebuilt compiler can be picked up with the
// "Salvo: Restart Language Server" command (or a settings change).

import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
// Serializes restarts: a config change (or the restart command) always
// waits for any restart already in flight, so two changes in quick
// succession cannot race to create two clients that stomp on `client`
// (each leaking its own output channel in the process).
let restarting: Promise<void> = Promise.resolve();

/** Resolves `salvo.serverPath`: absolute paths and bare command names are
 * used as-is; relative paths are resolved against the first workspace
 * folder (so `target/debug/salvo` works inside the salvo-lang repo). */
function resolveServerPath(): string {
  const config = vscode.workspace.getConfiguration("salvo");
  const configured = config.get<string>("serverPath", "salvo").trim() || "salvo";
  if (path.isAbsolute(configured) || !configured.includes(path.sep)) {
    return configured;
  }
  const folder = vscode.workspace.workspaceFolders?.[0];
  return folder ? path.join(folder.uri.fsPath, configured) : configured;
}

async function startClient(): Promise<void> {
  const command = resolveServerPath();

  // A relative/absolute path that does not exist is a configuration
  // problem worth a clear message (a bare command name is left to the OS
  // PATH lookup).
  if (command.includes(path.sep) && !fs.existsSync(command)) {
    void vscode.window.showErrorMessage(
      `Salvo: server binary not found at \`${command}\`. ` +
        "Set `salvo.serverPath` to your salvo binary (e.g. target/debug/salvo) " +
        "and run \"Salvo: Restart Language Server\".",
    );
    return;
  }

  const args = ["lsp"];
  const backend = vscode.workspace
    .getConfiguration("salvo")
    .get<string>("backend", "")
    .trim();
  if (backend.length > 0) {
    args.push("--backend", backend);
  }

  const serverEnv = vscode.workspace
    .getConfiguration("salvo")
    .get<Record<string, string>>("serverEnv", {});

  const serverOptions: ServerOptions = {
    command,
    args,
    options: {
      // Merge over the editor's own environment so unset vars (PATH, etc.)
      // still resolve; `serverEnv` only adds or overrides specific keys —
      // e.g. SALVO_LSP_TRACE/SALVO_LSP_SLOW_MS/SALVO_LSP_SLOW_DIR when
      // diagnosing a stuck or spinning server [lsp-trace] [lsp-slow-dump].
      env: { ...process.env, ...serverEnv },
    },
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "salvo" }],
  };

  client = new LanguageClient(
    "salvo",
    "Salvo Language Server",
    serverOptions,
    clientOptions,
  );
  try {
    await client.start();
  } catch (err) {
    client = undefined;
    void vscode.window.showErrorMessage(
      `Salvo: failed to start \`${command} ${args.join(" ")}\`: ${err}. ` +
        "Check `salvo.serverPath`, then run \"Salvo: Restart Language Server\".",
    );
  }
}

async function stopClient(): Promise<void> {
  if (client) {
    const current = client;
    client = undefined;
    // `stop()` alone leaves the client's output channel (the "Salvo
    // Language Server" entry in the Output panel) undisposed — a restart
    // that only stopped would leak one channel per restart. `dispose()`
    // stops the server (if still running) and disposes the channel.
    await current.dispose().catch(() => {
      /* server already gone (e.g. binary replaced mid-run) */
    });
  }
}

async function restartClient(): Promise<void> {
  restarting = restarting.then(stopClient).then(startClient);
  await restarting;
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("salvo.restartServer", restartClient),
    // Repoint automatically when the configuration changes.
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (
        event.affectsConfiguration("salvo.serverPath") ||
        event.affectsConfiguration("salvo.backend") ||
        event.affectsConfiguration("salvo.serverEnv")
      ) {
        await restartClient();
      }
    }),
  );
  await restartClient();
}

export async function deactivate(): Promise<void> {
  await restarting;
  await stopClient();
}
