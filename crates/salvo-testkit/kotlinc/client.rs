// [test-warm-kotlinc] The `kotlinc` the test suite runs: a client of a warm
// compiler (`KotlincServer.kt`), installed by `salvo-testkit` into
// `target/tmp/kotlinc-warm/<key>/bin/kotlinc` and put first on the tests' PATH.
// Built with `rustc` alone (std only) at install time.
//
// It takes the arguments `kotlinc` takes, makes the paths among them
// absolute (the server's working directory is not the caller's), runs the
// compile on a free server — starting one if its slot has none — and prints
// what `kotlinc` would have printed on stderr, exiting with its code. Anything
// it does not understand (`-version`, `-script`, JVM options, an option it
// does not know), and any failure to reach a server, runs the real `kotlinc`
// instead: the client may be slower than intended, never different.
//
// The install directory holds: `real` (the real kotlinc), `java` and
// `jvm-args` (the JVM the real kotlinc runs on, and the flags its launcher
// gives it), `classpath` (the server's classes and the compiler jar), `slots`
// (how many servers may run), and `run/` (0700: the slots' locks, sockets
// and logs).

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long a server waits for its next request before it exits.
const IDLE_SECS: &str = "120";
/// How many compiles a server runs before it exits (a fresh one follows).
const MAX_COMPILES: &str = "250";

fn main() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let root = std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .and_then(|p| p.parent()?.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let read = |name: &str| std::fs::read_to_string(root.join(name)).unwrap_or_default().trim().to_string();
    let real = read("real");
    if std::env::var_os("SALVO_WARM_KOTLINC").is_some_and(|v| v == "0") {
        exec_real(&real, &args);
    }
    let Some(argv) = translate(&args) else { exec_real(&real, &args) };
    let slots: usize = read("slots").parse().unwrap_or(2).max(1);
    let run = root.join("run");
    // A free slot if there is one; otherwise wait for one, chosen by pid so
    // waiting clients spread out.
    let mut held: Option<(usize, File)> = None;
    for i in 0..slots {
        let Ok(f) = OpenOptions::new().create(true).write(true).open(run.join(format!("slot-{i}.lock"))) else { continue };
        if f.try_lock().is_ok() {
            held = Some((i, f));
            break;
        }
    }
    let (slot, _lock) = match held {
        Some(h) => h,
        None => {
            let i = std::process::id() as usize % slots;
            let Ok(f) = OpenOptions::new().create(true).write(true).open(run.join(format!("slot-{i}.lock"))) else {
                exec_real(&real, &args)
            };
            if f.lock().is_err() {
                exec_real(&real, &args);
            }
            (i, f)
        }
    };
    let socket = run.join(format!("slot-{slot}.sock"));
    // One retry with a fresh server: a server may exit (idle, or its compile
    // budget spent) just as a client connects.
    for attempt in 0..2 {
        let stream = match UnixStream::connect(&socket) {
            Ok(s) => Some(s),
            Err(_) => start(&root, &run, slot, &socket),
        };
        if let Some(stream) = stream {
            if let Some((code, messages)) = request(stream, &argv) {
                let _ = std::io::stderr().write_all(&messages);
                std::process::exit(code);
            }
        }
        if attempt == 0 {
            let _ = std::fs::remove_file(&socket);
        }
    }
    exec_real(&real, &args)
}

/// Runs the real `kotlinc` in this process's place.
fn exec_real(real: &str, args: &[OsString]) -> ! {
    let err = Command::new(if real.is_empty() { "kotlinc" } else { real }).args(args).exec();
    eprintln!("kotlinc (warm client): cannot run the real kotlinc `{real}`: {err}");
    std::process::exit(2)
}

/// The arguments with their paths made absolute, or `None` for a call the
/// server should not take.
fn translate(args: &[OsString]) -> Option<Vec<String>> {
    // Options whose value is a path (or a path list), and options whose value
    // is anything else.
    const PATH_VALUED: &[&str] = &["-d", "-cp", "-classpath", "-kotlin-home", "-jdk-home"];
    const VALUED: &[&str] = &["-module-name", "-jvm-target", "-api-version", "-language-version", "-opt-in", "-jvm-default"];
    const FLAGS: &[&str] = &["-nowarn", "-no-stdlib", "-no-reflect", "-no-jdk", "-include-runtime", "-Werror", "-verbose", "-progressive", "-java-parameters"];
    let cwd = std::env::current_dir().ok()?;
    let abs = |p: &str| -> String {
        let path = Path::new(p);
        if path.is_absolute() { p.to_string() } else { cwd.join(path).to_string_lossy().into_owned() }
    };
    if args.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    let mut it = args.iter().map(|a| a.to_str()).peekable();
    while let Some(arg) = it.next() {
        let arg = arg?;
        if PATH_VALUED.contains(&arg) {
            let value = it.next()??;
            out.push(arg.to_string());
            out.push(if arg == "-cp" || arg == "-classpath" {
                value.split(':').filter(|p| !p.is_empty()).map(abs).collect::<Vec<_>>().join(":")
            } else {
                abs(value)
            });
        } else if VALUED.contains(&arg) {
            out.push(arg.to_string());
            out.push(it.next()??.to_string());
        } else if FLAGS.contains(&arg) || (arg.starts_with("-X") && arg.len() > 2) {
            out.push(arg.to_string());
        } else if arg.starts_with('-') || arg.starts_with('@') {
            return None;
        } else {
            out.push(abs(arg));
        }
    }
    Some(out)
}

/// Starts the server of `slot` and waits for its socket.
fn start(root: &Path, run: &Path, slot: usize, socket: &Path) -> Option<UnixStream> {
    let read = |name: &str| std::fs::read_to_string(root.join(name)).ok();
    let java = read("java")?.trim().to_string();
    let classpath = read("classpath")?.trim().to_string();
    let jvm_args: Vec<String> = read("jvm-args").unwrap_or_default().lines().map(str::to_string).filter(|l| !l.is_empty()).collect();
    let _ = std::fs::remove_file(socket);
    let log = File::create(run.join(format!("slot-{slot}.log"))).ok()?;
    let mut child = Command::new(java)
        .args(&jvm_args)
        .arg("-cp")
        .arg(classpath)
        .arg("salvo.testkit.KotlincServerKt")
        .arg(socket)
        .args([IDLE_SECS, MAX_COMPILES])
        .stdin(Stdio::null())
        .stdout(log.try_clone().ok()?)
        .stderr(log)
        // Its own process group: a test runner ending a test's group must
        // not take the shared server with it.
        .process_group(0)
        .spawn()
        .ok()?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(120) {
        if let Ok(s) = UnixStream::connect(socket) {
            return Some(s);
        }
        if let Ok(Some(_)) = child.try_wait() {
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    None
}

/// One compile on `stream`: the exit code and the messages.
fn request(mut stream: UnixStream, argv: &[String]) -> Option<(i32, Vec<u8>)> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(argv.len() as u32).to_be_bytes());
    for a in argv {
        buf.extend_from_slice(&(a.len() as u32).to_be_bytes());
        buf.extend_from_slice(a.as_bytes());
    }
    stream.write_all(&buf).ok()?;
    stream.flush().ok()?;
    let mut word = [0u8; 4];
    stream.read_exact(&mut word).ok()?;
    let code = i32::from_be_bytes(word);
    stream.read_exact(&mut word).ok()?;
    let mut messages = vec![0u8; u32::from_be_bytes(word) as usize];
    stream.read_exact(&mut messages).ok()?;
    Some((code, messages))
}
