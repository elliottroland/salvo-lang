//! The CLI side of `salvo fmt` [cli-fmt]: which files it reads and writes,
//! `--check`, standard input, files that do not parse, directories, and
//! std. What the formatter writes is the print fixtures' (`salvo-core`'s
//! `tests/print/`, `print_tests.rs`), the import sort [fmt-imports] and the
//! language page's example included.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// A project at `dir` whose source root is `src/`.
fn project(dir: &Path) -> PathBuf {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("salvo.toml"), "[project]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"src\"\n").unwrap();
    dir.to_path_buf()
}

fn salvo_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_salvo")).current_dir(dir).args(args).output().expect("failed to run salvo")
}

fn salvo_stdin(dir: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_salvo"))
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run salvo");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap()
}

/// The `.sv` files under `dir`, relative to it.
fn sv_files(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, root, out);
            } else if p.extension().is_some_and(|x| x == "sv") {
                out.push(p.strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// The failures of a scenario, each named by it.
struct Scenarios(Vec<String>);

impl Scenarios {
    fn check(&mut self, scenario: &str, holds: bool, what: &str, out: Option<&Output>) {
        if !holds {
            let shown = out.map(|o| format!("\n  exit: {}\n  stdout: {}\n  stderr: {}", o.status, text(&o.stdout), text(&o.stderr))).unwrap_or_default();
            self.0.push(format!("[{scenario}] {what}{shown}"));
        }
    }
}

const UNFORMATTED: &str = "fn main( ) -> Int {\n  let x = 1\n\n\n  return x\n}\n";
const FORMATTED: &str = "fn main() -> Int {\n    let x = 1\n\n    return x\n}\n";

/// [cli-fmt] Every CLI scenario of `salvo fmt`, each in its own directory of
/// one scratch tree, each failure named by its scenario:
///
/// - `in place`: with no paths, the project's source tree is formatted in
///   place (a test annex too), and formatting again, from a subdirectory,
///   changes nothing;
/// - `unchanged`: a file already formatted is not written, its modification
///   time stays;
/// - `check`: `--check` writes nothing, lists each file that would change
///   relative to the working directory, and fails; it passes, listing
///   nothing, once they are formatted;
/// - `stdin`: `-` formats standard input to standard output; with `--check`
///   it prints nothing and the exit says whether the input was formatted;
///   input that does not parse fails, reported as `<stdin>`; `-` is the only
///   path or none;
/// - `parse error`: a file that does not parse is left as written, its
///   errors reported as diagnostics are, and the exit fails; the others are
///   still formatted, and `--check` fails without listing it;
/// - `directory`: a directory named is formatted throughout, without a
///   manifest, its hidden directories skipped; a file that is no `.sv` is
///   refused;
/// - `std copy`: std is formatted like any project: `--check` in a copy of
///   `lib/` finds every one of std's files (annexes included), and writes
///   none of them.
#[test]
fn fmt_reads_and_writes_the_files_it_should() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fmt_cli");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let mut s = Scenarios(Vec::new());

    let dir = project(&root.join("in_place"));
    fs::write(dir.join("src/main.sv"), UNFORMATTED).unwrap();
    fs::create_dir_all(dir.join("src/app")).unwrap();
    fs::write(dir.join("src/app/util.test.sv"), "test \"t\" {\n  assert!(true)\n}\n").unwrap();
    let out = salvo_in(&dir, &["fmt"]);
    s.check("in place", out.status.success() && text(&out.stderr).contains("fmt: reformatted 2 of 2 files"), "both files reformatted", Some(&out));
    s.check("in place", read(dir.join("src/main.sv")) == FORMATTED, "main.sv formatted", None);
    s.check("in place", read(dir.join("src/app/util.test.sv")) == "test \"t\" {\n    assert!(true)\n}\n", "the annex formatted", None);
    // Formatted now, so not written again: its modification time stays.
    let main = dir.join("src/main.sv");
    let past = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    fs::File::options().write(true).open(&main).unwrap().set_modified(past).unwrap();
    let again = salvo_in(&dir.join("src/app"), &["fmt"]);
    s.check("in place", again.status.success() && text(&again.stderr).contains("fmt: reformatted 0 of 2 files"), "nothing reformatted again", Some(&again));
    s.check("in place", read(&main) == FORMATTED, "main.sv unchanged", None);
    s.check("unchanged", fs::metadata(&main).unwrap().modified().unwrap() == past, "a formatted file is not written", None);

    let dir = project(&root.join("check"));
    fs::write(dir.join("src/a.sv"), UNFORMATTED).unwrap();
    fs::write(dir.join("src/b.sv"), FORMATTED).unwrap();
    let out = salvo_in(&dir, &["fmt", "--check"]);
    s.check("check", !out.status.success() && text(&out.stdout) == "src/a.sv\n", "fails, listing the file that would change", Some(&out));
    s.check("check", text(&out.stderr).contains("fmt: 1 of 2 files would be reformatted"), "counts it", Some(&out));
    s.check("check", read(dir.join("src/a.sv")) == UNFORMATTED, "writes nothing", None);
    let inner = salvo_in(&dir.join("src"), &["fmt", "--check", "a.sv", "b.sv"]);
    s.check("check", text(&inner.stdout) == "a.sv\n", "lists relative to the working directory", Some(&inner));
    let formatted = salvo_in(&dir, &["fmt"]);
    s.check("check", formatted.status.success(), "formats", Some(&formatted));
    let clean = salvo_in(&dir, &["fmt", "--check"]);
    s.check("check", clean.status.success() && text(&clean.stdout).is_empty(), "passes once formatted", Some(&clean));

    let dir = root.join("stdin");
    fs::create_dir_all(&dir).unwrap();
    let out = salvo_stdin(&dir, &["fmt", "-"], UNFORMATTED);
    s.check("stdin", out.status.success() && text(&out.stdout) == FORMATTED, "formats to standard output", Some(&out));
    let check = salvo_stdin(&dir, &["fmt", "--check", "-"], UNFORMATTED);
    s.check("stdin", !check.status.success() && text(&check.stdout).is_empty(), "`--check` fails on unformatted input, printing nothing", Some(&check));
    let clean = salvo_stdin(&dir, &["fmt", "--check", "-"], FORMATTED);
    s.check("stdin", clean.status.success() && text(&clean.stdout).is_empty(), "`--check` passes on formatted input, printing nothing", Some(&clean));
    let broken = salvo_stdin(&dir, &["fmt", "-"], "fn f( {\n");
    s.check("stdin", !broken.status.success() && text(&broken.stdout).is_empty() && text(&broken.stderr).contains("--> <stdin>:1:7"), "input that does not parse fails, reported at `<stdin>`", Some(&broken));
    let both = salvo_in(&dir, &["fmt", "-", "x.sv"]);
    s.check("stdin", !both.status.success(), "`-` is the only path", Some(&both));

    let dir = project(&root.join("parse_error"));
    let broken = "fn f( {\n  return 1\n}\n";
    fs::write(dir.join("src/bad.sv"), broken).unwrap();
    fs::write(dir.join("src/main.sv"), UNFORMATTED).unwrap();
    let out = salvo_in(&dir, &["-v", "fmt"]);
    let stderr = text(&out.stderr);
    s.check("parse error", !out.status.success(), "fails", Some(&out));
    s.check("parse error", stderr.contains("error[syntax-expected:identifier]") && stderr.contains("--> src/bad.sv:1:7"), "reports the error as a diagnostic", Some(&out));
    s.check("parse error", stderr.contains("fmt: 1 file left as written"), "counts the file left", Some(&out));
    s.check("parse error", read(dir.join("src/bad.sv")) == broken, "leaves it as written", None);
    s.check("parse error", read(dir.join("src/main.sv")) == FORMATTED, "formats the others", None);
    let check = salvo_in(&dir, &["fmt", "--check"]);
    s.check("parse error", !check.status.success() && text(&check.stdout).is_empty(), "`--check` fails without listing it", Some(&check));

    let dir = root.join("directory");
    fs::create_dir_all(dir.join("tree/deep")).unwrap();
    fs::create_dir_all(dir.join("tree/.hidden")).unwrap();
    fs::write(dir.join("tree/a.sv"), UNFORMATTED).unwrap();
    fs::write(dir.join("tree/deep/b.sv"), UNFORMATTED).unwrap();
    fs::write(dir.join("tree/.hidden/c.sv"), UNFORMATTED).unwrap();
    fs::write(dir.join("tree/notes.txt"), "not salvo\n").unwrap();
    let out = salvo_in(&dir, &["fmt", "--check", "tree"]);
    s.check("directory", text(&out.stdout) == "tree/a.sv\ntree/deep/b.sv\n", "`--check` lists the files throughout, the hidden ones skipped", Some(&out));
    let formatted = salvo_in(&dir, &["fmt", "tree"]);
    s.check("directory", formatted.status.success(), "formats", Some(&formatted));
    s.check("directory", read(dir.join("tree/deep/b.sv")) == FORMATTED, "a nested file formatted", None);
    s.check("directory", read(dir.join("tree/.hidden/c.sv")) == UNFORMATTED, "a hidden directory skipped", None);
    let txt = salvo_in(&dir, &["fmt", "tree/notes.txt"]);
    s.check("directory", !txt.status.success() && text(&txt.stderr).contains("is not a Salvo source file"), "a file that is no `.sv` refused", Some(&txt));

    let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib");
    let dir = root.join("std_copy");
    fs::create_dir_all(&dir).unwrap();
    fs::copy(lib.join("salvo.toml"), dir.join("salvo.toml")).unwrap();
    let files = sv_files(&lib.join("std"));
    s.check("std copy", files.len() > 50, &format!("found only {} files", files.len()), None);
    for f in &files {
        let to = dir.join("std").join(f);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(lib.join("std").join(f), &to).unwrap();
    }
    let out = salvo_in(&dir, &["fmt", "--check"]);
    s.check("std copy", text(&out.stderr).contains(&format!(" of {} files would be reformatted", files.len())), "finds every one of std's files", Some(&out));
    for listed in text(&out.stdout).lines() {
        s.check("std copy", listed.starts_with("std/") && listed.ends_with(".sv"), &format!("lists `{listed}`"), None);
    }
    for f in &files {
        s.check("std copy", fs::read(dir.join("std").join(f)).unwrap() == fs::read(lib.join("std").join(f)).unwrap(), &format!("wrote {}", f.display()), None);
    }

    assert!(s.0.is_empty(), "{} failures:\n{}", s.0.len(), s.0.join("\n\n"));
}
