//! [cli-fmt] `salvo fmt [--check] [PATH…]`: every file formatted by
//! `salvo_syntax::fmt::format`, the one formatter the language server's
//! document formatting calls too [lsp-format].

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use salvo_syntax::fmt::{format, FormatError};

/// Runs the command: `paths` empty formats the project's source tree, `-`
/// alone reads stdin and writes stdout.
pub fn run(paths: &[PathBuf], check: bool) -> ExitCode {
    if paths.iter().any(|p| p.as_os_str() == "-") {
        if paths.len() > 1 {
            eprintln!("error: `-` (standard input) must be the only path");
            return ExitCode::FAILURE;
        }
        return stdin(check);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cwd = cwd.canonicalize().unwrap_or(cwd);
    let (files, mut failed) = match files_of(paths) {
        Ok(found) => found,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    let mut changed = 0;
    for path in &files {
        let shown = relative(path, &cwd);
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(err) => {
                eprintln!("error: failed to read `{shown}`: {err}");
                failed += 1;
                continue;
            }
        };
        let formatted = match format(&text) {
            Ok(f) => f,
            Err(error) => {
                report(&error, &shown, &text);
                failed += 1;
                continue;
            }
        };
        if formatted == text {
            continue;
        }
        changed += 1;
        if check {
            println!("{shown}");
        } else if let Err(err) = std::fs::write(path, &formatted) {
            eprintln!("error: failed to write `{shown}`: {err}");
            failed += 1;
            changed -= 1;
        }
    }
    let total = files.len();
    let plural = |n: usize| if n == 1 { "" } else { "s" };
    match (check, changed) {
        (true, 0) => eprintln!("fmt: 0 of {total} file{} would be reformatted", plural(total)),
        (true, n) => eprintln!("fmt: {n} of {total} file{} would be reformatted; run `salvo fmt`", plural(total)),
        (false, n) => eprintln!("fmt: reformatted {n} of {total} file{}", plural(total)),
    }
    if failed > 0 {
        eprintln!("fmt: {failed} file{} left as written", plural(failed));
    }
    if failed > 0 || (check && changed > 0) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `-`: the text on stdin, formatted to stdout; under `--check` nothing is
/// printed, and the exit says whether it was formatted.
fn stdin(check: bool) -> ExitCode {
    let mut text = String::new();
    if let Err(err) = std::io::stdin().read_to_string(&mut text) {
        eprintln!("error: failed to read standard input: {err}");
        return ExitCode::FAILURE;
    }
    match format(&text) {
        Err(error) => {
            report(&error, "<stdin>", &text);
            ExitCode::FAILURE
        }
        Ok(formatted) if check => {
            if formatted == text {
                ExitCode::SUCCESS
            } else {
                eprintln!("fmt: <stdin> would be reformatted");
                ExitCode::FAILURE
            }
        }
        Ok(formatted) => {
            let mut out = std::io::stdout().lock();
            if out.write_all(formatted.as_bytes()).and_then(|()| out.flush()).is_err() {
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
    }
}

/// Why a file was left as written, on stderr: its parse errors in the usual
/// form, or the printer's refusal.
fn report(error: &FormatError, name: &str, text: &str) {
    match error {
        FormatError::Parse(errors) => {
            for d in errors {
                eprintln!("{}", d.render(name, text));
            }
        }
        FormatError::Unsafe(why) => eprintln!("error: `{name}` was left as written: {why} (a formatter defect)"),
    }
}

/// The files to format, and how many named paths could not be: the
/// project's source tree for no paths, else each file named and every
/// `.sv` under each directory named.
fn files_of(paths: &[PathBuf]) -> Result<(Vec<PathBuf>, usize), String> {
    let mut files = Vec::new();
    let mut failed = 0;
    if paths.is_empty() {
        let start = PathBuf::from(".");
        let Some(project) = salvo_core::Project::find(&start)? else {
            return Err(salvo_syntax::messages::rendered(&salvo_syntax::messages::manifest_discovery_no_manifest(
                salvo_core::MANIFEST_FILE,
                &start.display(),
            )));
        };
        let (found, errors) = salvo_core::source::source_tree_sv_files(&project.src());
        for e in &errors {
            eprintln!("error: {e}");
        }
        return Ok((found, errors.len()));
    }
    for path in paths {
        if path.is_dir() {
            let root = path.canonicalize().unwrap_or_else(|_| path.clone());
            let (found, errors) = salvo_core::source::walk_tree(&root, &[], salvo_core::source::Walk::Named, |e| e == "sv");
            for e in &errors {
                eprintln!("error: {e}");
            }
            failed += errors.len();
            files.extend(found);
        } else if !path.exists() {
            eprintln!("error: `{}` does not exist", path.display());
            failed += 1;
        } else if path.extension().is_none_or(|e| e != "sv") {
            eprintln!("error: `{}` is not a Salvo source file (`.sv`)", path.display());
            failed += 1;
        } else {
            files.push(path.canonicalize().unwrap_or_else(|_| path.clone()));
        }
    }
    files.sort();
    files.dedup();
    Ok((files, failed))
}

/// `path` relative to `cwd` (`../x.sv` where it is outside), as a file is
/// named to the user.
fn relative(path: &Path, cwd: &Path) -> String {
    let path: Vec<Component> = path.components().collect();
    let cwd: Vec<Component> = cwd.components().collect();
    let common = path.iter().zip(&cwd).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return path.iter().collect::<PathBuf>().display().to_string();
    }
    let mut out = PathBuf::new();
    for _ in common..cwd.len() {
        out.push("..");
    }
    out.extend(&path[common..]);
    out.display().to_string()
}
