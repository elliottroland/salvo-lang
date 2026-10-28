//! [test-run-data] The compile-and-run data (TESTS.md C3): every program
//! under `tests/run/<area>/*.sv` (the format: `salvo_testkit::run`), and
//! every example under `examples/`, compiled by each backend it names and
//! run, its stdout, exit code or trap asserted.
//!
//! One test per area. Within it:
//!
//! - **Emission** runs on a pool of threads over the process's std fronts
//!   ([std-cache]); a program that does not emit fails here, toolchain or
//!   not.
//! - **Stamps** ([`salvo_testkit::cached`]) are keyed by what decides the
//!   outcome: the backend, its toolchain's version, the generated files, the
//!   arguments and the expected outcome. A program whose generated code did
//!   not change is not compiled again.
//! - **Rust** (TD9): the programs missing a stamp compile in batches, one
//!   crate each — std's modules (and the runtime files) once at the crate
//!   root, each program a module `p<i>` whose own `crate::` paths are
//!   rewritten into it, and a `main` dispatching on `SALVO_RUN_CASE`. A
//!   program whose shared files differ from the batch's goes to another
//!   batch; a batch that fails to compile is compiled again one program per
//!   crate, so the failure names its program.
//! - **Kotlin** ([kt-std-library]): as the codegen shards did — std compiled
//!   once into a library, each program's own files rewritten into a package
//!   namespace of its own, a few `kotlinc` processes per area.
//! - Every compiled program then runs on a pool of threads, with a time
//!   limit, in a fresh working directory of its own.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use salvo_core::SourceSet;
use salvo_testkit::run::{Backend, Case, Outcome};

const TMP: &str = env!("CARGO_TARGET_TMPDIR");

/// How long one program may run.
const RUN_LIMIT: Duration = Duration::from_secs(60);

/// At most this many programs share one Rust crate: rustc's front end is
/// one thread, so a batch's wall time grows with it, while std is compiled
/// once per batch.
const RUST_BATCH: usize = 16;

macro_rules! areas {
    ($($area:ident),* $(,)?) => {
        $(
            #[test]
            fn $area() {
                run_area(stringify!($area));
            }
        )*
        /// Every directory under `tests/run/` has its test.
        #[test]
        fn every_area_has_a_test() {
            let listed: BTreeSet<&str> = [$(stringify!($area)),*].into_iter().collect();
            let found: BTreeSet<String> = std::fs::read_dir(run_dir())
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            let missing: Vec<&String> = found.iter().filter(|f| !listed.contains(f.as_str())).collect();
            assert!(missing.is_empty(), "tests/run/ areas without an `areas!` line: {missing:?}");
        }
    };
}

areas!(actors, args, collections, comptime, control, deductions, effects, files, functions, handles, implicits, iteration, linear, modules, network, operators, pilot, platform, qualifiers, regressions, throwing, time, types);

fn run_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/run")
}

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// Every example runs to its `expected.txt` on both backends,
/// which is the parity claim of `examples/README.md`.
#[test]
fn examples() {
    let mut names: Vec<String> = std::fs::read_dir(examples_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("salvo").is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no examples found");
    let programs: Vec<Program> = names
        .iter()
        .map(|name| {
            let dir = examples_dir().join(name);
            let expected = std::fs::read_to_string(dir.join("expected.txt"))
                .unwrap_or_else(|_| panic!("examples/{name}/expected.txt is missing"));
            Program {
                label: format!("examples/{name}"),
                source: Source::Project(dir),
                backends: Backend::ALL.to_vec(),
                args: Vec::new(),
                stdin: None,
                outcome: Outcome { stdout: expected, ..Outcome::default() },
            }
        })
        .collect();
    run_programs("examples", programs);
}

/// Where a program's sources come from.
enum Source {
    /// A data file's case: its files, written out and read as a source tree.
    Case(Case),
    /// A project directory (an example), read as `salvo compile` reads it.
    Project(PathBuf),
}

/// One program to compile and run.
struct Program {
    /// `area/file.sv` or `area/file.sv#name`, for reports.
    label: String,
    source: Source,
    backends: Vec<Backend>,
    args: Vec<String>,
    stdin: Option<String>,
    outcome: Outcome,
}

fn run_area(area: &str) {
    let dir = run_dir().join(area);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sv"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no `.sv` data files in {}", dir.display());
    let mut programs = Vec::new();
    let mut errors = Vec::new();
    for path in &paths {
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(path).unwrap();
        match salvo_testkit::run::parse(&text) {
            Ok(cases) => {
                let one = cases.len() == 1;
                for case in cases {
                    let label = if one { format!("{area}/{file}") } else { format!("{area}/{file}#{}", case.name) };
                    programs.push(Program {
                        label,
                        backends: case.backends.clone(),
                        args: case.args.clone(),
                        stdin: case.stdin.clone(),
                        outcome: case.outcome.clone(),
                        source: Source::Case(case),
                    });
                }
            }
            Err(e) => errors.push(format!("{area}/{file}: {e}")),
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    run_programs(area, programs);
}

/// One program on one backend, emitted.
struct Job<'p> {
    program: &'p Program,
    backend: Backend,
    files: Vec<(PathBuf, String)>,
}

impl Job<'_> {
    fn label(&self) -> String {
        format!("{} ({})", self.program.label, self.backend.name())
    }
}

/// Runs `f` over `items` on a pool of threads, answers in order.
fn pool<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(items.len().max(1));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out: Mutex<Vec<(usize, R)>> = Mutex::new(Vec::with_capacity(items.len()));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(item) = items.get(i) else { return };
                let r = f(item);
                out.lock().unwrap().push((i, r));
            });
        }
    });
    let mut out = out.into_inner().unwrap();
    out.sort_by_key(|(i, _)| *i);
    out.into_iter().map(|(_, r)| r).collect()
}

/// A name usable as a path segment, a package and a stamp label.
fn ident(label: &str) -> String {
    label.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

/// `program`'s generated files for `backend`, or what went wrong.
fn emit(program: &Program, backend: Backend) -> Result<Vec<(PathBuf, String)>, String> {
    let ext = backend.ext();
    let front = salvo_testkit::front::std_front(ext);
    let built = match &program.source {
        Source::Project(dir) => salvo_testkit::front::project(ext, dir),
        Source::Case(case) => {
            // Written out and read back as a source tree, so a companion or
            // platform file is classified as `salvo compile` would; a case
            // with a `salvo.toml` is a project, read through its manifest.
            let dir = Path::new(TMP).join("run-src").join(format!("{}-{}", ident(&program.label), ext));
            let _ = std::fs::remove_dir_all(&dir);
            for (name, content) in &case.files {
                let path = dir.join(name);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, content).unwrap();
            }
            if case.files.iter().any(|(n, _)| n == salvo_core::MANIFEST_FILE) {
                salvo_testkit::front::project(ext, &dir)
            } else {
                let mut sources = SourceSet::default();
                let errors = sources.add_dir(&dir, ext, false, false);
                if !errors.is_empty() {
                    return Err(format!("reading the program: {errors:?}"));
                }
                front.program(sources.files, sources.companions)
            }
        }
    };
    let parse: Vec<String> = built.diagnostics.iter().filter(|d| d.is_error()).map(|d| d.render(&built.program.files)).collect();
    if !parse.is_empty() {
        return Err(format!("parse or expansion errors:\n{}", parse.join("\n")));
    }
    let emitted = match backend {
        Backend::Rust => salvo_backend_rust::emit_program_over(&built.program, front)
            .map(|fs| fs.into_iter().map(|f| (f.rel_path, f.content)).collect::<Vec<_>>()),
        Backend::Kotlin => salvo_backend_kotlin::emit_program_over(&built.program, front)
            .map(|fs| fs.into_iter().map(|f| (f.rel_path, f.content)).collect::<Vec<_>>()),
    };
    emitted.map_err(|errors| format!("codegen errors:\n{}", errors.join("\n")))
}

/// The stamp for `job` having run to its outcome under `version`.
fn stamp(job: &Job, version: &str) -> Option<salvo_testkit::Stamp> {
    let o = &job.program.outcome;
    let mut parts: Vec<Vec<u8>> = vec![
        b"run-data".to_vec(),
        job.backend.name().as_bytes().to_vec(),
        version.as_bytes().to_vec(),
        job.program.args.join("\0").into_bytes(),
        job.program.stdin.clone().map(|s| format!("stdin:{s}")).unwrap_or_default().into_bytes(),
        o.stderr.clone().map(|s| format!("stderr:{s}")).unwrap_or_default().into_bytes(),
        o.stdout.as_bytes().to_vec(),
        o.exit.to_string().into_bytes(),
        o.trap.clone().unwrap_or_default().into_bytes(),
        vec![o.trap.is_some() as u8],
    ];
    for (rel, content) in &job.files {
        parts.push(rel.to_string_lossy().as_bytes().to_vec());
        parts.push(content.as_bytes().to_vec());
    }
    let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
    salvo_testkit::cached(TMP, &format!("run-data {}", job.label()), &refs)
}

/// How to start one compiled program.
struct Launch {
    program: PathBuf,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

fn run_programs(area: &str, programs: Vec<Program>) {
    if salvo_testkit::skip_e2e() {
        return;
    }
    let mut seen = BTreeSet::new();
    for p in &programs {
        assert!(seen.insert(p.label.clone()), "two programs are labelled {}", p.label);
    }
    // Emission, on every backend each program names.
    let wanted: Vec<(&Program, Backend)> = programs.iter().flat_map(|p| p.backends.iter().map(move |b| (p, *b))).collect();
    let emitted = pool(&wanted, |(p, b)| emit(p, *b));
    let mut failures: Vec<String> = Vec::new();
    let mut jobs: Vec<Job> = Vec::new();
    for ((program, backend), result) in wanted.into_iter().zip(emitted) {
        match result {
            Ok(files) => jobs.push(Job { program, backend, files }),
            Err(e) => failures.push(format!("{} ({}): {e}", program.label, backend.name())),
        }
    }
    // Stamps: what has run to this outcome before is not run again.
    let mut pending: Vec<(Job, salvo_testkit::Stamp)> = Vec::new();
    let mut versions: HashMap<Backend, Option<String>> = HashMap::new();
    for job in jobs {
        let version = versions
            .entry(job.backend)
            .or_insert_with(|| {
                let t = match job.backend {
                    Backend::Rust => salvo_testkit::rustc(),
                    Backend::Kotlin => salvo_testkit::kotlinc(TMP),
                };
                t.available.then_some(t.version)
            })
            .clone();
        let Some(version) = version else { continue };
        if let Some(s) = stamp(&job, &version) {
            pending.push((job, s));
        }
    }
    let (rust, kotlin): (Vec<_>, Vec<_>) = pending.into_iter().partition(|(j, _)| j.backend == Backend::Rust);
    let scratch = salvo_testkit::scratch(TMP, &format!("run-{area}"));
    let mut launches: Vec<(Job, salvo_testkit::Stamp, Launch)> = Vec::new();
    let t = Instant::now();
    let (rs_launches, rs_failures) = compile_rust(&scratch.join("rust"), rust);
    let t_rs = t.elapsed();
    let (kt_launches, kt_failures) = compile_kotlin(&scratch.join("kotlin"), kotlin);
    let t_kt = t.elapsed() - t_rs;
    launches.extend(rs_launches);
    launches.extend(kt_launches);
    failures.extend(rs_failures);
    failures.extend(kt_failures);
    // Every compiled program runs, its outcome compared.
    let verdicts = pool(&launches, |(job, _, launch)| check_run(job, launch));
    eprintln!("[run-data] {area}: rustc {:.1}s, kotlinc {:.1}s, runs {:.1}s", t_rs.as_secs_f64(), t_kt.as_secs_f64(), (t.elapsed() - t_rs - t_kt).as_secs_f64());
    for ((_, stamp, _), verdict) in launches.into_iter().zip(verdicts) {
        match verdict {
            Ok(()) => stamp.verified(),
            Err(e) => failures.push(e),
        }
    }
    if failures.is_empty() {
        let _ = std::fs::remove_dir_all(&scratch);
    }
    assert!(failures.is_empty(), "{} failures in {area}:\n\n{}", failures.len(), failures.join("\n\n"));
}

/// Runs `launch` and compares what it did with `job`'s outcome. The program
/// runs in a fresh working directory of its own, so the files a program
/// writes at relative paths never meet another run's (the two backends run
/// the same program at once) and never land in the source tree.
fn check_run(job: &Job, launch: &Launch) -> Result<(), String> {
    let cwd = Path::new(TMP).join("run-cwd").join(ident(&job.label()));
    let _ = std::fs::remove_dir_all(&cwd);
    std::fs::create_dir_all(&cwd).map_err(|e| format!("{}: {e}", job.label()))?;
    let mut cmd = Command::new(&launch.program);
    cmd.current_dir(&cwd).args(&launch.args).args(&job.program.args);
    for (k, v) in &launch.env {
        cmd.env(k, v);
    }
    let ran = run_limited(cmd, job.program.stdin.as_deref()).map_err(|e| format!("{}: {e}", job.label()));
    let _ = std::fs::remove_dir_all(&cwd);
    let ran = ran?;
    let o = &job.program.outcome;
    let mut wrong = Vec::new();
    if ran.stdout != o.stdout {
        wrong.push(format!("--- expected stdout:\n{}--- got:\n{}", o.stdout, ran.stdout));
    }
    match &o.trap {
        Some(text) => {
            let wanted = format!("salvo: {text}");
            if ran.code == Some(0) || !ran.stderr.contains(&wanted) {
                wrong.push(format!("expected a trap `{wanted}`, got exit {:?}", ran.code));
            }
        }
        None if ran.code != Some(o.exit) => wrong.push(format!("expected exit {}, got {:?}", o.exit, ran.code)),
        None => {}
    }
    if let Some(stderr) = &o.stderr {
        if ran.stderr != *stderr {
            wrong.push(format!("--- expected stderr:\n{stderr}--- got (below)"));
        }
    }
    if wrong.is_empty() {
        return Ok(());
    }
    Err(format!("{}: {}\n--- stderr:\n{}", job.label(), wrong.join("\n"), ran.stderr))
}

struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Runs `cmd` to its end or [`RUN_LIMIT`], whichever is first.
fn run_limited(mut cmd: Command, stdin: Option<&str>) -> Result<Ran, String> {
    let mut child = cmd
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start {:?}: {e}", cmd.get_program()))?;
    if let Some(text) = stdin {
        let mut input = child.stdin.take().unwrap();
        let text = text.to_string();
        // Written from a thread, so a program that prints before it reads
        // cannot deadlock against a full pipe.
        std::thread::spawn(move || {
            use std::io::Write;
            let _ = input.write_all(text.as_bytes());
        });
    }
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    let out_reader = std::thread::spawn(move || {
        let mut s = Vec::new();
        let _ = out.read_to_end(&mut s);
        s
    });
    let err_reader = std::thread::spawn(move || {
        let mut s = Vec::new();
        let _ = err.read_to_end(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > RUN_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("still running after {}s", RUN_LIMIT.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let stdout = String::from_utf8_lossy(&out_reader.join().unwrap()).to_string();
    let stderr = String::from_utf8_lossy(&err_reader.join().unwrap()).to_string();
    Ok(Ran { code: status.code(), stdout, stderr })
}

// ===== Rust: one crate per batch (TD9) =====

/// Runtime files the Rust backend writes beside std's modules: the same
/// text in every program that has them.
const RS_RUNTIME: &[&str] = &["wire.rs", "strings.rs", "seq.rs", "scheduler.rs", "hoststreams.rs", "hosttime.rs"];

/// Whether a generated file is std's or the runtime's — shared by every
/// program of a batch — rather than the program's own.
fn rs_shared(rel: &Path) -> bool {
    rel.starts_with("std") || rel.starts_with("platform/std") || rel.starts_with("unions") || RS_RUNTIME.iter().any(|r| rel == Path::new(r))
}

/// A program split for batching: its own files, its shared files, the
/// shared modules its root mounts, and its union arities.
struct RsSplit {
    own: Vec<(PathBuf, String)>,
    shared: BTreeMap<PathBuf, String>,
    mounts: BTreeMap<String, String>,
    arities: BTreeSet<String>,
}

/// Splits `files` (one program) for a batch, the program becoming module
/// `module`: the root's mounts of shared modules come out (the batch root
/// mounts them), and every `crate::x` naming the program's own items
/// becomes `crate::<module>::x`.
fn rs_split(files: &[(PathBuf, String)], module: &str) -> Result<RsSplit, String> {
    let mut own = Vec::new();
    let mut shared = BTreeMap::new();
    let mut arities = BTreeSet::new();
    for (rel, content) in files {
        if rel == Path::new("unions/mod.rs") {
            arities.extend(content.lines().filter_map(|l| l.strip_prefix("mod ").and_then(|l| l.strip_suffix(';'))).map(str::to_string));
        } else if rs_shared(rel) {
            shared.insert(rel.clone(), content.clone());
        } else {
            own.push((rel.clone(), content.clone()));
        }
    }
    let root = own.iter().position(|(rel, _)| rel == Path::new("main.rs")).ok_or("no `main.rs`")?;
    let mut mounts = BTreeMap::new();
    let mut kept = Vec::new();
    let lines: Vec<&str> = own[root].1.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let path = lines[i].strip_prefix("#[path = \"").and_then(|l| l.strip_suffix("\"]"));
        let name = lines.get(i + 1).and_then(|l| l.strip_prefix("pub mod ")).and_then(|l| l.strip_suffix(';'));
        if let (Some(path), Some(name)) = (path, name) {
            if rs_shared(Path::new(path)) {
                mounts.insert(name.to_string(), path.to_string());
                i += 2;
                continue;
            }
        }
        kept.push(lines[i]);
        i += 1;
    }
    own[root].1 = kept.join("\n") + "\n";
    for (_, content) in &mut own {
        *content = rs_rebase(content, module, &mounts);
    }
    Ok(RsSplit { own, shared, mounts, arities })
}

/// `crate::x` → `crate::<module>::x` for every `x` not a shared module.
fn rs_rebase(content: &str, module: &str, shared: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(i) = rest.find("crate::") {
        let before = rest[..i].chars().next_back();
        out.push_str(&rest[..i + "crate::".len()]);
        rest = &rest[i + "crate::".len()..];
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$') {
            continue;
        }
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !shared.contains_key(&name) {
            out.push_str(module);
            out.push_str("::");
        }
    }
    out.push_str(rest);
    out
}

/// One Rust batch being assembled.
#[derive(Default)]
struct RsBatch {
    members: Vec<usize>,
    shared: BTreeMap<PathBuf, String>,
    mounts: BTreeMap<String, String>,
    arities: BTreeSet<String>,
}

impl RsBatch {
    fn admits(&self, s: &RsSplit) -> bool {
        self.members.len() < RUST_BATCH
            && s.shared.iter().all(|(rel, c)| self.shared.get(rel).is_none_or(|have| have == c))
            && s.mounts.iter().all(|(n, p)| self.mounts.get(n).is_none_or(|have| have == p))
    }

    fn add(&mut self, i: usize, s: &RsSplit) {
        self.members.push(i);
        self.shared.extend(s.shared.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.mounts.extend(s.mounts.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.arities.extend(s.arities.iter().cloned());
    }
}

/// Writes one crate holding `members` (indices into `splits`) under `dir`,
/// compiles it, and answers the binary or rustc's complaint.
fn rs_build(dir: &Path, batch: &RsBatch, splits: &[RsSplit]) -> Result<PathBuf, String> {
    let _ = std::fs::remove_dir_all(dir);
    let write = |rel: &Path, content: &str| {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    };
    for (rel, content) in &batch.shared {
        write(rel, content);
    }
    let mut root = String::from("#![allow(warnings)]\n");
    if !batch.arities.is_empty() {
        let mut m = String::new();
        for a in &batch.arities {
            m.push_str(&format!("mod {a};\npub use {a}::*;\n"));
        }
        write(Path::new("unions/mod.rs"), &m);
    }
    for (name, path) in &batch.mounts {
        root.push_str(&format!("#[path = \"{path}\"]\npub mod {name};\n"));
    }
    let mut arms = String::new();
    for &i in &batch.members {
        let module = format!("p{i}");
        for (rel, content) in &splits[i].own {
            write(&Path::new(&module).join(rel), content);
        }
        root.push_str(&format!("#[path = \"{module}/main.rs\"]\npub mod {module};\n"));
        arms.push_str(&format!("        \"{i}\" => __run({module}::main()),\n"));
    }
    root.push_str(&format!(
        "\nfn __run<T: std::process::Termination>(t: T) -> std::process::ExitCode {{ t.report() }}\n\n\
         fn main() -> std::process::ExitCode {{\n    match std::env::var(\"SALVO_RUN_CASE\").as_deref().unwrap_or(\"\") {{\n{arms}        other => panic!(\"no program `{{other}}` in this batch\"),\n    }}\n}}\n"
    ));
    write(Path::new("main.rs"), &root);
    let bin = dir.join("program");
    let out = Command::new("rustc")
        .args(["--edition", "2021"])
        .arg(dir.join("main.rs"))
        .arg("-o")
        .arg(&bin)
        .output()
        .map_err(|e| format!("failed to run rustc: {e}"))?;
    if out.status.success() {
        Ok(bin)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

fn compile_rust<'p>(dir: &Path, pending: Vec<(Job<'p>, salvo_testkit::Stamp)>) -> (Vec<(Job<'p>, salvo_testkit::Stamp, Launch)>, Vec<String>) {
    let mut failures = Vec::new();
    let mut splits = Vec::new();
    let mut kept = Vec::new();
    for (i, (job, stamp)) in pending.into_iter().enumerate() {
        match rs_split(&job.files, &format!("p{i}")) {
            Ok(s) => {
                splits.push(s);
                kept.push((job, stamp));
            }
            Err(e) => {
                failures.push(format!("{}: {e}", job.label()));
                // Keep the indices of `splits` and `kept` in step.
                splits.push(RsSplit { own: Vec::new(), shared: BTreeMap::new(), mounts: BTreeMap::new(), arities: BTreeSet::new() });
                kept.push((job, stamp));
            }
        }
    }
    let mut batches: Vec<RsBatch> = Vec::new();
    for (i, s) in splits.iter().enumerate() {
        if s.own.is_empty() {
            continue;
        }
        match batches.iter_mut().find(|b| b.admits(s)) {
            Some(b) => b.add(i, s),
            None => {
                let mut b = RsBatch::default();
                b.add(i, s);
                batches.push(b);
            }
        }
    }
    let built = pool(&batches, |b| rs_build(&dir.join(format!("b{}", b.members[0])), b, &splits));
    eprintln!("[run-data] {} Rust programs in {} crates", kept.len(), batches.len());
    // A batch that failed: each program alone, so the failure is its own.
    let mut bins: HashMap<usize, PathBuf> = HashMap::new();
    let mut alone: Vec<RsBatch> = Vec::new();
    for (b, result) in batches.iter().zip(built) {
        match result {
            Ok(bin) => bins.extend(b.members.iter().map(|&i| (i, bin.clone()))),
            Err(_) if b.members.len() > 1 => {
                for &i in &b.members {
                    let mut one = RsBatch::default();
                    one.add(i, &splits[i]);
                    alone.push(one);
                }
            }
            Err(e) => failures.push(format!("{}: rustc failed:\n{e}", kept[b.members[0]].0.label())),
        }
    }
    let rebuilt = pool(&alone, |b| rs_build(&dir.join(format!("one{}", b.members[0])), b, &splits));
    for (b, result) in alone.iter().zip(rebuilt) {
        match result {
            Ok(bin) => {
                bins.insert(b.members[0], bin);
            }
            Err(e) => failures.push(format!("{}: rustc failed:\n{e}", kept[b.members[0]].0.label())),
        }
    }
    let mut launches = Vec::new();
    for (i, (job, stamp)) in kept.into_iter().enumerate() {
        if let Some(bin) = bins.remove(&i) {
            launches.push((job, stamp, Launch { program: bin, args: Vec::new(), env: vec![("SALVO_RUN_CASE".into(), i.to_string())] }));
        }
    }
    (launches, failures)
}

// ===== Kotlin: one library for std, a few kotlinc per area =====

/// [kt-std-library] std, compiled once into the test target and shared by
/// every area (a concurrent first build is settled by `prepare`'s rename).
fn kotlin_library() -> &'static salvo_backend_kotlin::stdlib::StdLibrary {
    static LIB: std::sync::OnceLock<salvo_backend_kotlin::stdlib::StdLibrary> = std::sync::OnceLock::new();
    LIB.get_or_init(|| {
        let program = salvo_testkit::front::clean_program("kt", &[]);
        let files = salvo_backend_kotlin::stdlib::std_library_files(&program)
            .unwrap_or_else(|e| panic!("std library emission failed:\n{}", e.join("\n")));
        salvo_backend_kotlin::stdlib::prepare(&Path::new(TMP).join("salvo-kt-std"), files).unwrap_or_else(|e| panic!("{e}"))
    })
}

/// [kt-std-library] Rewrites `packages` (a program's own) into the
/// namespace `pfx`, and leaves every other `salvo.` name — std's, which the
/// library declares — as it is. A package matches only as a whole dotted
/// name.
fn prefix_packages(content: &str, pfx: &str, packages: &[String]) -> String {
    let mut pkgs: Vec<&String> = packages.iter().collect();
    pkgs.sort_by_key(|p| std::cmp::Reverse(p.len()));
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = String::with_capacity(content.len());
    let mut i = 0;
    'outer: while i < content.len() {
        let before = content[..i].chars().next_back();
        if before.is_none_or(|c| !ident(c) && c != '.') {
            for p in &pkgs {
                if content[i..].starts_with(p.as_str()) {
                    let after = content[i + p.len()..].chars().next();
                    if after.is_none_or(|c| !ident(c)) {
                        out.push_str(pfx);
                        out.push('.');
                        out.push_str(p);
                        i += p.len();
                        continue 'outer;
                    }
                }
            }
        }
        let c = content[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// A program that cannot link the library: every `salvo` package rewritten
/// into `pfx`. Blind and textual — a string literal holding `salvo.` would
/// change, which is why an expected output mentioning it is refused.
fn prefix_all(content: &str, pfx: &str) -> String {
    content
        .split('\n')
        .map(|line| match line.strip_prefix("package salvo") {
            Some(rest) => format!("package {pfx}.salvo{rest}"),
            None => line.replace("salvo.", &format!("{pfx}.salvo.")),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn compile_kotlin<'p>(dir: &Path, pending: Vec<(Job<'p>, salvo_testkit::Stamp)>) -> (Vec<(Job<'p>, salvo_testkit::Stamp, Launch)>, Vec<String>) {
    let mut failures = Vec::new();
    if pending.is_empty() {
        return (Vec::new(), failures);
    }
    let library = kotlin_library();
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    // Each kotlinc pays ~2.5s to start and is itself multi-threaded; the
    // areas run beside one another, so a few per area.
    let chunk_count = pending.len().min((workers / 4).max(1));
    let per_chunk = pending.len().div_ceil(chunk_count);
    let mut chunks: Vec<Vec<(Job, salvo_testkit::Stamp)>> = Vec::new();
    let mut it = pending.into_iter().peekable();
    while it.peek().is_some() {
        chunks.push(it.by_ref().take(per_chunk).collect());
    }
    // (chunk dir, whole-program sources, library-linked sources, linked tags)
    let mut plans: Vec<(PathBuf, Vec<PathBuf>, Vec<PathBuf>, BTreeSet<String>)> = Vec::new();
    for (c, chunk) in chunks.iter().enumerate() {
        let cdir = dir.join(format!("c{c}"));
        let _ = std::fs::remove_dir_all(&cdir);
        let (mut whole, mut linked, mut tags) = (Vec::new(), Vec::new(), BTreeSet::new());
        let mut root_files: BTreeMap<PathBuf, String> = BTreeMap::new();
        for (job, _) in chunk {
            let tag = format!("k_{}", ident(&job.program.label));
            let write = |sub: &str, rel: &Path, content: &str| -> PathBuf {
                let path = cdir.join(sub).join(&tag).join(rel);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, content).unwrap();
                path
            };
            let own = library.own_files(job.files.iter().map(|(r, c)| (r.as_path(), c.as_str())));
            let Some(own) = own else {
                for (rel, content) in &job.files {
                    whole.push(write("src", rel, &prefix_all(content, &tag)));
                }
                continue;
            };
            tags.insert(tag.clone());
            let own: Vec<&(PathBuf, String)> = job.files.iter().filter(|(r, _)| own.contains(&r.as_path())).collect();
            let packages: Vec<String> = own
                .iter()
                .filter_map(|(_, c)| c.lines().find_map(|l| l.strip_prefix("package ")).map(str::to_string))
                .filter(|p| p != "salvo")
                .collect();
            for (rel, content) in own {
                if content.lines().any(|l| l == "package salvo") {
                    // A root-package file beyond the library's (an arity std
                    // does not use): once per chunk, the same everywhere.
                    if let Some(prev) = root_files.insert(rel.clone(), content.clone()) {
                        assert_eq!(prev, *content, "{}: `{}` differs from another program's", job.label(), rel.display());
                    }
                    continue;
                }
                linked.push(write("shared", rel, &prefix_packages(content, &tag, &packages)));
            }
        }
        for (rel, content) in root_files {
            let path = cdir.join("shared").join("__root").join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, content).unwrap();
            linked.push(path);
        }
        plans.push((cdir, whole, linked, tags));
    }
    // Every chunk's two compilations, in parallel.
    let compiles: Vec<(usize, bool)> = (0..plans.len()).flat_map(|c| [(c, false), (c, true)]).collect();
    let results = pool(&compiles, |&(c, linked)| {
        let (cdir, whole, shared, _) = &plans[c];
        let paths = if linked { shared } else { whole };
        if paths.is_empty() {
            return Ok(());
        }
        let out = cdir.join(if linked { "out_shared" } else { "out" });
        let mut cmd = Command::new("kotlinc");
        // [kt-wire] Every struct carries a codec: a batch of ~30 programs
        // outgrew kotlinc's default heap.
        cmd.env("JAVA_OPTS", "-Xmx3g").args(paths).arg("-d").arg(&out);
        if linked {
            cmd.arg("-cp").arg(&library.classes);
        }
        let result = cmd.output().map_err(|e| format!("failed to run kotlinc: {e}"))?;
        if result.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&result.stderr).to_string())
        }
    });
    let mut broken: BTreeSet<(usize, bool)> = BTreeSet::new();
    for (&(c, linked), result) in compiles.iter().zip(results) {
        if let Err(e) = result {
            // kotlinc names the files, whose paths carry the program's tag.
            failures.push(format!("kotlinc failed for chunk {c}{}:\n{e}", if linked { " (linked)" } else { "" }));
            broken.insert((c, linked));
        }
    }
    let mut launches = Vec::new();
    for (c, chunk) in chunks.into_iter().enumerate() {
        let (cdir, _, _, tags) = &plans[c];
        for (job, stamp) in chunk {
            let tag = format!("k_{}", ident(&job.program.label));
            let linked = tags.contains(&tag);
            if broken.contains(&(c, linked)) {
                continue;
            }
            let cp = if linked {
                std::env::join_paths([cdir.join("out_shared"), library.classes.clone()]).unwrap()
            } else {
                cdir.join("out").into_os_string()
            };
            let cp = cp.to_string_lossy().to_string();
            if job.program.outcome.stdout.contains("salvo.") {
                failures.push(format!("{}: the expected output mentions `salvo.`, which the package rewrite would change", job.label()));
                continue;
            }
            launches.push((job, stamp, Launch { program: "kotlin".into(), args: vec!["-cp".into(), cp, format!("{tag}.salvo.main.MainKt")], env: Vec::new() }));
        }
    }
    (launches, failures)
}
