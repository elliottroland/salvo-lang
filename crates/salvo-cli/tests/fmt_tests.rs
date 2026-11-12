//! Integration tests for `salvo fmt` [cli-fmt]: files formatted in place
//! and only when they change, `--check`, standard input, files that do not
//! parse, directories, and the import sort [fmt-imports].

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// A fresh, empty scratch directory for `test`.
fn scratch(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("fmt_{test}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A project at `dir` whose source root is `src/`.
fn project(test: &str) -> PathBuf {
    let dir = scratch(test);
    fs::write(dir.join("salvo.toml"), "[project]\nname = \"p\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"src\"\n").unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    dir
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

const UNFORMATTED: &str = "fn main( ) -> Int {\n  let x = 1\n\n\n  return x\n}\n";
const FORMATTED: &str = "fn main() -> Int {\n    let x = 1\n\n    return x\n}\n";

// [cli-fmt] With no paths, the project's source tree is formatted in place;
// formatting again changes nothing.
#[test]
fn the_project_is_formatted_in_place_and_formatting_again_changes_nothing() {
    let dir = project("in_place");
    fs::write(dir.join("src/main.sv"), UNFORMATTED).unwrap();
    fs::create_dir_all(dir.join("src/app")).unwrap();
    fs::write(dir.join("src/app/util.test.sv"), "test \"t\" {\n  assert!(true)\n}\n").unwrap();
    let out = salvo_in(&dir, &["fmt"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stderr).contains("fmt: reformatted 2 of 2 files"), "{}", text(&out.stderr));
    assert_eq!(fs::read_to_string(dir.join("src/main.sv")).unwrap(), FORMATTED);
    assert_eq!(fs::read_to_string(dir.join("src/app/util.test.sv")).unwrap(), "test \"t\" {\n    assert!(true)\n}\n");
    let again = salvo_in(&dir.join("src/app"), &["fmt"]);
    assert!(again.status.success(), "{}", text(&again.stderr));
    assert!(text(&again.stderr).contains("fmt: reformatted 0 of 2 files"), "{}", text(&again.stderr));
    assert_eq!(fs::read_to_string(dir.join("src/main.sv")).unwrap(), FORMATTED);
}

// [cli-fmt] A file already formatted is not written: its modification time
// stays.
#[test]
fn a_formatted_file_is_not_written() {
    let dir = project("untouched");
    let path = dir.join("src/main.sv");
    fs::write(&path, FORMATTED).unwrap();
    let past = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    fs::File::options().write(true).open(&path).unwrap().set_modified(past).unwrap();
    let out = salvo_in(&dir, &["fmt"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), past);
}

// [cli-fmt] `--check` writes nothing, lists each file that would change
// relative to the working directory, and fails; it passes once they are
// formatted.
#[test]
fn check_lists_the_files_that_would_change() {
    let dir = project("check");
    fs::write(dir.join("src/a.sv"), UNFORMATTED).unwrap();
    fs::write(dir.join("src/b.sv"), FORMATTED).unwrap();
    let out = salvo_in(&dir, &["fmt", "--check"]);
    assert!(!out.status.success());
    assert_eq!(text(&out.stdout), "src/a.sv\n");
    assert!(text(&out.stderr).contains("fmt: 1 of 2 files would be reformatted"), "{}", text(&out.stderr));
    assert_eq!(fs::read_to_string(dir.join("src/a.sv")).unwrap(), UNFORMATTED);
    // Named from a subdirectory, the path is relative to it.
    let inner = salvo_in(&dir.join("src"), &["fmt", "--check", "a.sv", "b.sv"]);
    assert_eq!(text(&inner.stdout), "a.sv\n");
    assert!(salvo_in(&dir, &["fmt"]).status.success());
    let clean = salvo_in(&dir, &["fmt", "--check"]);
    assert!(clean.status.success(), "{}", text(&clean.stderr));
    assert_eq!(text(&clean.stdout), "");
}

// [cli-fmt] `-` formats standard input to standard output; with `--check`
// it prints nothing and the exit says whether the input was formatted.
#[test]
fn stdin_is_formatted_to_stdout() {
    let dir = scratch("stdin");
    let out = salvo_stdin(&dir, &["fmt", "-"], UNFORMATTED);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), FORMATTED);
    let check = salvo_stdin(&dir, &["fmt", "--check", "-"], UNFORMATTED);
    assert!(!check.status.success());
    assert_eq!(text(&check.stdout), "");
    let clean = salvo_stdin(&dir, &["fmt", "--check", "-"], FORMATTED);
    assert!(clean.status.success(), "{}", text(&clean.stderr));
    assert_eq!(text(&clean.stdout), "");
    let broken = salvo_stdin(&dir, &["fmt", "-"], "fn f( {\n");
    assert!(!broken.status.success());
    assert_eq!(text(&broken.stdout), "");
    assert!(text(&broken.stderr).contains("--> <stdin>:1:7"), "{}", text(&broken.stderr));
    // `-` is the only path or none.
    assert!(!salvo_in(&dir, &["fmt", "-", "x.sv"]).status.success());
}

// [cli-fmt] A file that does not parse is left as written, its errors
// reported as diagnostics are, and the exit fails; the others are still
// formatted.
#[test]
fn a_file_that_does_not_parse_is_left_as_written() {
    let dir = project("parse_error");
    let broken = "fn f( {\n  return 1\n}\n";
    fs::write(dir.join("src/bad.sv"), broken).unwrap();
    fs::write(dir.join("src/main.sv"), UNFORMATTED).unwrap();
    let out = salvo_in(&dir, &["fmt"]);
    assert!(!out.status.success());
    let stderr = text(&out.stderr);
    assert!(stderr.contains("error: expected identifier, found `{` --> src/bad.sv:1:7"), "{stderr}");
    assert!(stderr.contains("fmt: 1 file left as written"), "{stderr}");
    assert_eq!(fs::read_to_string(dir.join("src/bad.sv")).unwrap(), broken);
    assert_eq!(fs::read_to_string(dir.join("src/main.sv")).unwrap(), FORMATTED);
    // So does `--check`, which does not list it.
    let check = salvo_in(&dir, &["fmt", "--check"]);
    assert!(!check.status.success());
    assert_eq!(text(&check.stdout), "");
}

// [cli-fmt] A directory named is formatted throughout, without a manifest;
// hidden directories are skipped, and a file that is no `.sv` is refused.
#[test]
fn a_directory_named_is_formatted_throughout() {
    let dir = scratch("directory");
    fs::create_dir_all(dir.join("tree/deep")).unwrap();
    fs::create_dir_all(dir.join("tree/.hidden")).unwrap();
    fs::write(dir.join("tree/a.sv"), UNFORMATTED).unwrap();
    fs::write(dir.join("tree/deep/b.sv"), UNFORMATTED).unwrap();
    fs::write(dir.join("tree/.hidden/c.sv"), UNFORMATTED).unwrap();
    fs::write(dir.join("tree/notes.txt"), "not salvo\n").unwrap();
    let out = salvo_in(&dir, &["fmt", "--check", "tree"]);
    assert_eq!(text(&out.stdout), "tree/a.sv\ntree/deep/b.sv\n");
    assert!(salvo_in(&dir, &["fmt", "tree"]).status.success());
    assert_eq!(fs::read_to_string(dir.join("tree/deep/b.sv")).unwrap(), FORMATTED);
    assert_eq!(fs::read_to_string(dir.join("tree/.hidden/c.sv")).unwrap(), UNFORMATTED);
    let txt = salvo_in(&dir, &["fmt", "tree/notes.txt"]);
    assert!(!txt.status.success());
    assert!(text(&txt.stderr).contains("is not a Salvo source file"), "{}", text(&txt.stderr));
}

// [cli-fmt] [fmt-imports] Imports are sorted, each with the comment written
// above it, and nothing is removed (an unused import stays).
#[test]
fn imports_sort_with_their_comments() {
    let dir = scratch("imports");
    let written = "// The app.\n\nimport std.random.DefaultRandom\n// The heap orders the work.\nimport std.heap.Heap\n\nimport std.heap.Heap\n\nfn main() -> Int {\n    return 1\n}\n";
    fs::write(dir.join("main.sv"), written).unwrap();
    assert!(salvo_in(&dir, &["fmt", "main.sv"]).status.success());
    let once = fs::read_to_string(dir.join("main.sv")).unwrap();
    assert_eq!(
        once,
        "// The app.\n\n// The heap orders the work.\nimport std.heap.Heap\nimport std.heap.Heap\nimport std.random.DefaultRandom\n\nfn main() -> Int {\n    return 1\n}\n"
    );
    let again = salvo_in(&dir, &["fmt", "--check", "main.sv"]);
    assert!(again.status.success(), "{}", text(&again.stdout));
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

// [cli-fmt] std is formatted like any project: `salvo fmt --check` in a copy
// of `lib/` finds every one of std's files (annexes included), and writes
// none of them.
#[test]
fn check_in_a_copy_of_std_finds_its_files() {
    let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib");
    let dir = scratch("std_copy");
    fs::copy(lib.join("salvo.toml"), dir.join("salvo.toml")).unwrap();
    let files = sv_files(&lib.join("std"));
    assert!(files.len() > 50, "found only {} files", files.len());
    for f in &files {
        let to = dir.join("std").join(f);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(lib.join("std").join(f), &to).unwrap();
    }
    let out = salvo_in(&dir, &["fmt", "--check"]);
    let stderr = text(&out.stderr);
    assert!(stderr.contains(&format!(" of {} files would be reformatted", files.len())), "{stderr}");
    for listed in text(&out.stdout).lines() {
        assert!(listed.starts_with("std/") && listed.ends_with(".sv"), "{listed}");
    }
    for f in &files {
        assert_eq!(fs::read(dir.join("std").join(f)).unwrap(), fs::read(lib.join("std").join(f)).unwrap(), "{}", f.display());
    }
}

// [cli-fmt] The language page's example formats as the page says, and its
// formatting is formatted already.
#[test]
fn the_formatting_page_example_formats_as_shown() {
    let page = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/language/Formatting.md")).unwrap();
    let fences: Vec<&str> = page.split("```salvo\n").skip(1).map(|f| f.split("```").next().unwrap()).collect();
    let (before, after) = (fences[0], fences[1]);
    assert_eq!(salvo_syntax::fmt::format(before).unwrap(), after);
    assert_eq!(salvo_syntax::fmt::format(after).unwrap(), after);
}
