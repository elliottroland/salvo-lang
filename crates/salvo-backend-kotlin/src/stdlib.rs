//! [kt-std-library] The standard library compiled once and linked by every
//! program (user decision 2026-10-05).
//!
//! Since names are per module [fn-emit-name] and unions sit one arity to a
//! file, std's generated Kotlin no longer depends on the program that uses
//! it. So the library is std emitted on its own — every std module a root —
//! and compiled into a classes directory under a cache root, keyed by the
//! content of what was compiled and the `kotlinc` that compiled it. A program
//! then compiles only the files the library does not cover, with the library
//! on the classpath.
//!
//! The reuse is checked, never assumed: a program file whose path the
//! library also has must be **byte-identical** to the library's, or the
//! program compiles whole, as before. (A std a dependency shadows, say, still
//! builds; it just does not share.)

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use salvo_core::program::Program;

use crate::EmittedFile;

/// A compiled standard library: its sources by path, and the classes.
pub struct StdLibrary {
    pub classes: PathBuf,
    sources: BTreeMap<PathBuf, String>,
}

/// The library a previous `prepare` installed at `entry`, read back.
pub fn load(entry: &Path) -> Option<StdLibrary> {
    if !entry.join("done").is_file() {
        return None;
    }
    let src = entry.join("src");
    let mut sources = BTreeMap::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).ok()? {
            let path = e.ok()?.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(&src).ok()?.to_path_buf();
                sources.insert(rel, std::fs::read_to_string(&path).ok()?);
            }
        }
    }
    Some(StdLibrary { classes: entry.join("classes"), sources })
}

/// The standard library's own build: std alone, every std module a root.
pub fn std_library_files(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    let keep: Vec<usize> = program
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.is_std && !f.is_test)
        .map(|(i, _)| i)
        .collect();
    let std_program = Program {
        files: keep.iter().map(|&i| program.files[i].clone()).collect(),
        modules: keep.iter().map(|&i| program.modules[i].clone()).collect(),
        companions: program
            .companions
            .iter()
            .filter(|c| keep.iter().any(|&i| program.files[i].module == c.module))
            .cloned()
            .collect(),
    };
    crate::emit::emit_program(&std_program)
}

/// FNV-1a, 64 bits: stable across builds of the compiler, which a cache key
/// written to disk needs.
fn fnv(bytes: &[u8], mut h: u64) -> u64 {
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Who `kotlinc` is: the resolved executable and its modification time.
fn kotlinc_identity() -> String {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("kotlinc");
        if let Ok(real) = std::fs::canonicalize(&candidate) {
            let stamp = std::fs::metadata(&real)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            return format!("{}@{stamp}", real.display());
        }
    }
    "kotlinc".to_string()
}

/// The cache entry's directory name for `files`.
fn key(files: &[EmittedFile]) -> String {
    let mut h = fnv(kotlinc_identity().as_bytes(), 0xcbf29ce484222325);
    let mut sorted: Vec<&EmittedFile> = files.iter().collect();
    sorted.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    for f in sorted {
        h = fnv(f.rel_path.to_string_lossy().as_bytes(), h);
        h = fnv(&[0], h);
        h = fnv(f.content.as_bytes(), h);
        h = fnv(&[0], h);
    }
    format!("{h:016x}")
}

/// The library for `files` under `cache_root`, compiling it when no entry
/// holds it yet. An entry is built under a private name and renamed into
/// place, so concurrent builders never see half of one; the loser of a race
/// discards its copy.
pub fn prepare(cache_root: &Path, files: Vec<EmittedFile>) -> Result<StdLibrary, String> {
    let entry = cache_root.join(format!("kotlin-std-{}", key(&files)));
    let sources: BTreeMap<PathBuf, String> =
        files.into_iter().map(|f| (f.rel_path, f.content)).collect();
    let classes = entry.join("classes");
    if entry.join("done").is_file() {
        return Ok(StdLibrary { classes, sources });
    }
    std::fs::create_dir_all(cache_root).map_err(|e| format!("{}: {e}", cache_root.display()))?;
    let staging = cache_root.join(format!(
        ".building-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let src = staging.join("src");
    let mut paths = Vec::new();
    for (rel, content) in &sources {
        let path = src.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
        paths.push(path);
    }
    let out = Command::new("kotlinc")
        .env("JAVA_OPTS", "-Xmx3g")
        .args(&paths)
        .arg("-nowarn")
        // Its own module name, so the program's `META-INF/main.kotlin_module`
        // does not shadow the library's.
        .args(["-module-name", "salvo-std"])
        .arg("-d")
        .arg(staging.join("classes"))
        .output()
        .map_err(|e| format!("cannot run kotlinc: {e}"))?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(format!(
            "kotlinc failed compiling the standard library [kt-std-library]:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    std::fs::write(staging.join("done"), "").map_err(|e| e.to_string())?;
    if std::fs::rename(&staging, &entry).is_err() {
        // Another builder got there first; theirs is the same library.
        let _ = std::fs::remove_dir_all(&staging);
        if !entry.join("done").is_file() {
            return Err(format!("could not install the standard library at {}", entry.display()));
        }
    }
    Ok(StdLibrary { classes, sources })
}

impl StdLibrary {
    /// The cache entry holding this library.
    pub fn entry(&self) -> PathBuf {
        self.classes.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    /// The paths of `files` the library does not cover, when every file it
    /// does cover is byte-identical to the library's; `None` when one
    /// differs, and the program must compile whole.
    pub fn own_files<'f>(&self, files: impl IntoIterator<Item = (&'f Path, &'f str)>) -> Option<Vec<&'f Path>> {
        let mut own = Vec::new();
        for (rel, content) in files {
            match self.sources.get(rel) {
                Some(lib) if lib == content => {}
                Some(_) => return None,
                None => own.push(rel),
            }
        }
        Some(own)
    }
}

/// [kt-std-library] Where compiled libraries are kept: `$SALVO_CACHE_DIR`,
/// else `$XDG_CACHE_HOME/salvo`, else `~/.cache/salvo`; `None` when
/// `SALVO_NO_STD_CACHE` is set or no home is known.
pub fn default_cache_root() -> Option<PathBuf> {
    if std::env::var_os("SALVO_NO_STD_CACHE").is_some() {
        return None;
    }
    if let Some(dir) = std::env::var_os("SALVO_CACHE_DIR") {
        return Some(PathBuf::from(dir));
    }
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(dir).join("salvo"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache").join("salvo"))
}
