//! Kotlin backend for the Salvo compiler.

mod emit;
mod intrinsics;

pub use emit::{
    emit_program, emit_program_reporting, host_package, platform_skeletons, EmittedFile,
};

use std::path::{Path, PathBuf};

use salvo_backend::{run_tool, Backend, BackendError, Emitted};
use salvo_core::{HostDeps, ModulePath, Program};

/// Where `kotlinc` puts the compiled classes inside the target directory
/// [kt-run]. Dot-prefixed so a target nested in the source tree stays
/// invisible to source discovery [mod-ignore].
const CLASSES_DIR: &str = ".salvo_classes";

/// [kt-gradle] The Gradle build written beside the emitted sources when
/// artifacts are declared, the Kotlin counterpart of Rust's `Cargo.toml`
/// [rs-cargo]: `settings.gradle.kts` makes the target its own Gradle root (so
/// no enclosing build is picked up), `build.gradle.kts` resolves the
/// artifacts. Both carry [`GRADLE_HEADER`], which is how a stale one is told
/// from a hand-written one.
const GRADLE_BUILD: &str = "build.gradle.kts";
const GRADLE_SETTINGS: &str = "settings.gradle.kts";
const GRADLE_HEADER: &str =
    "// Written by salvo from `[kotlin] artifacts` [kt-gradle]: resolves them for the\n\
     // Kotlin build. Regenerated on every build — edit salvo.toml, not this file.";
/// [kt-gradle] Where the resolved classpath lands, relative to the target
/// (hidden, like the classes directory, so source discovery skips it).
const GRADLE_CLASSPATH: &str = ".salvo_gradle/classpath.txt";

/// The name Kotlin gives a file's top-level facade class: the file name with
/// its first letter capitalized and `Kt` appended (`other.kt` ->
/// `OtherKt`). A module's emitted file is named after its last segment.
fn facade_class(module: &ModulePath) -> String {
    let stem = module.0.last().map(String::as_str).unwrap_or("main");
    let mut chars = stem.chars();
    let capitalized = match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    };
    format!("{capitalized}Kt")
}

pub struct KotlinBackend;

impl Backend for KotlinBackend {
    fn name(&self) -> &'static str {
        "kotlin"
    }

    fn file_extension(&self) -> &'static str {
        "kt"
    }

    /// Kotlin compiles a top-level `main` in every module that declares
    /// one, so the entry selection changes only the launch class
    /// ([`Backend::entry_hint`]) and never the emitted code.
    fn emit(
        &self,
        program: &Program,
        target_dir: &Path,
        _entry: Option<&ModulePath>,
    ) -> Result<Emitted, BackendError> {
        let (files, warnings) =
            emit::emit_program_reporting(program).map_err(BackendError::Codegen)?;
        let mut written = Vec::with_capacity(files.len());
        for file in files {
            let path = target_dir.join(&file.rel_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, &file.content)?;
            written.push(file.rel_path);
        }
        Ok(Emitted {
            files: written,
            warnings,
        })
    }

    /// The JVM entry class. Kotlin names a file's facade class after the
    /// **file**, not the function: module `other` is emitted as `other.kt`
    /// in package `salvo.other`, so its top-level `main` lands in
    /// `salvo.other.OtherKt`. (Before `salvo run` exercised it, this hint
    /// read `…MainKt` unconditionally — correct only because every entry
    /// file so far was `main.sv`.)
    ///
    /// [platform-tree] When the host owns `main` — a `main` that needs a
    /// platform effect [platform-effect] — the entry class is the *host's*
    /// facade instead: the generated one declares `salvoMain`, whose
    /// signature the JVM launcher cannot satisfy.
    ///
    /// The evidence is the generated module's own text: it declares
    /// `salvoMain` exactly when the entry moved. An emitted `platform/…`
    /// file is *not* evidence — a `platform handler` [platform-handler] puts
    /// a host companion beside a module whose `main` is still the program's
    /// entry point, and reading the host file instead would sniff
    /// customer-written code. An unreadable target file falls back to the
    /// generated facade, the answer for every program with no host entry.
    fn entry_hint(
        &self,
        target_dir: &Path,
        main_module: &ModulePath,
        emitted: &[PathBuf],
    ) -> String {
        let host = salvo_core::host_rel_path(main_module, self.file_extension());
        let mut generated = PathBuf::new();
        for part in &main_module.0 {
            generated.push(part);
        }
        generated.set_extension(self.file_extension());
        let entry_moved = std::fs::read_to_string(target_dir.join(&generated))
            .is_ok_and(|src| src.contains(&format!("fun {}(", emit::SALVO_ENTRY)));
        if entry_moved && emitted.iter().any(|p| *p == host) {
            format!(
                "{}.{}",
                emit::host_package(main_module),
                facade_class(main_module)
            )
        } else {
            format!("salvo.{main_module}.{}", facade_class(main_module))
        }
    }

    /// Kotlin needs no entry hint here: the host file for a module is the
    /// same file wherever the entry happens to be.
    fn platform_skeletons(
        &self,
        program: &Program,
        _entry: Option<&ModulePath>,
    ) -> Result<Vec<(PathBuf, String)>, BackendError> {
        let files = emit::platform_skeletons(program).map_err(BackendError::Codegen)?;
        Ok(files
            .into_iter()
            .map(|f| (f.rel_path, f.content))
            .collect())
    }

    /// [kt-gradle] With artifacts declared [platform-host-deps], a Gradle
    /// build beside the sources resolves them; without, a build a previous
    /// run wrote is removed (only ours, recognised by its header).
    fn write_host_manifest(
        &self,
        target_dir: &Path,
        _main_module: &ModulePath,
        host: &HostDeps,
    ) -> Result<Vec<PathBuf>, BackendError> {
        let files = [GRADLE_BUILD, GRADLE_SETTINGS];
        if !host.has_kotlin_artifacts() {
            for name in files {
                let path = target_dir.join(name);
                if std::fs::read_to_string(&path).is_ok_and(|text| text.starts_with(GRADLE_HEADER)) {
                    std::fs::remove_file(&path)?;
                }
            }
            return Ok(Vec::new());
        }
        std::fs::create_dir_all(target_dir)?;
        std::fs::write(
            target_dir.join(GRADLE_BUILD),
            host.gradle_build_script(GRADLE_HEADER, Path::new(GRADLE_CLASSPATH)),
        )?;
        std::fs::write(
            target_dir.join(GRADLE_SETTINGS),
            format!("{GRADLE_HEADER}\n\nrootProject.name = \"salvo-host\"\n"),
        )?;
        Ok(files.iter().map(PathBuf::from).collect())
    }

    /// [kt-run] `kotlinc` every emitted `.kt` file into a classes directory
    /// inside the target, then `kotlin -cp` that directory with the entry
    /// class. Companion files [backend-companion] are `.kt` too, so they
    /// are part of `emitted` and compile with the rest — including the
    /// platform host [platform-tree].
    /// [kt-classpath] The declared `libs` directories' jars
    /// [platform-host-deps] go on the classpath of both the compile and the
    /// run; with none declared the invocations are as before.
    fn program_command(
        &self,
        target_dir: &Path,
        main_module: &ModulePath,
        emitted: &[PathBuf],
        host: &HostDeps,
    ) -> Result<std::process::Command, BackendError> {
        use std::ffi::OsStr;

        let classes = target_dir.join(CLASSES_DIR);
        let sources: Vec<PathBuf> = emitted
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "kt"))
            .map(|p| target_dir.join(p))
            .collect();
        if sources.is_empty() {
            return Err(BackendError::Other(
                "nothing to run: no Kotlin sources were emitted".to_string(),
            ));
        }
        // [kt-gradle] Gradle resolves the declared artifacts first; their
        // jars come before the `libs` directories' on both classpaths.
        let mut jars = Vec::new();
        if host.has_kotlin_artifacts() {
            if !target_dir.join(GRADLE_BUILD).is_file() {
                return Err(BackendError::Other(format!(
                    "artifacts are declared but `{}` was not written [kt-gradle]",
                    target_dir.join(GRADLE_BUILD).display()
                )));
            }
            let out = target_dir.join(GRADLE_CLASSPATH);
            let _ = std::fs::remove_file(&out);
            let code = salvo_backend::run_host_tool(
                &host.gradle_program(),
                "[kotlin] gradle",
                &[
                    OsStr::new("--quiet"),
                    OsStr::new("--project-dir"),
                    target_dir.as_os_str(),
                    OsStr::new("salvoClasspath"),
                ],
            )?;
            if code != 0 {
                return Err(BackendError::Other(format!(
                    "Gradle failed with exit code {code} resolving `[kotlin] artifacts` [kt-gradle]"
                )));
            }
            let listed = std::fs::read_to_string(&out).map_err(|e| {
                BackendError::Other(format!(
                    "Gradle ran but wrote no classpath to `{}`: {e} [kt-gradle]",
                    out.display()
                ))
            })?;
            jars.extend(listed.lines().filter(|l| !l.trim().is_empty()).map(PathBuf::from));
        }
        jars.extend(host.kotlin_jars());
        let classpath = |leading: &Path| {
            std::env::join_paths(std::iter::once(leading.to_path_buf()).chain(jars.iter().cloned()))
                .map_err(|e| BackendError::Other(format!("cannot build a classpath: {e}")))
        };

        let mut args: Vec<&OsStr> = sources.iter().map(|p| p.as_os_str()).collect();
        args.push(OsStr::new("-d"));
        args.push(classes.as_os_str());
        let jar_cp;
        if !jars.is_empty() {
            jar_cp = std::env::join_paths(jars.iter().cloned())
                .map_err(|e| BackendError::Other(format!("cannot build a classpath: {e}")))?;
            args.push(OsStr::new("-cp"));
            args.push(jar_cp.as_os_str());
        }
        let code = run_tool("kotlinc", &args)?;
        if code != 0 {
            return Err(BackendError::Other(format!(
                "kotlinc failed with exit code {code}"
            )));
        }

        let entry = self.entry_hint(target_dir, main_module, emitted);
        let mut command = std::process::Command::new("kotlin");
        command.arg("-cp").arg(classpath(&classes)?).arg(&entry);
        Ok(command)
    }
}
