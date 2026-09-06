//! Kotlin backend for the Salvo compiler.

mod emit;
mod intrinsics;

pub use emit::{
    emit_program, emit_program_reporting, host_package, platform_skeletons, EmittedFile,
};

use std::path::{Path, PathBuf};

use salvo_backend::{run_tool, Backend, BackendError, Emitted};
use salvo_core::{ModulePath, Program};

/// Where `kotlinc` puts the compiled classes inside the target directory
/// [kt-run]. Dot-prefixed so a target nested in the source tree stays
/// invisible to source discovery [mod-ignore].
const CLASSES_DIR: &str = ".salvo_classes";

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
    /// [platform-tree] When the host owns `main` — which the emitted
    /// `platform/…` file is the evidence for — the entry class is the
    /// *host's* facade instead: the generated one declares `salvoMain`,
    /// whose signature the JVM launcher cannot satisfy.
    fn entry_hint(
        &self,
        _target_dir: &Path,
        main_module: &ModulePath,
        emitted: &[PathBuf],
    ) -> String {
        let host = salvo_core::host_rel_path(main_module, self.file_extension());
        if emitted.iter().any(|p| *p == host) {
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

    /// [kt-run] `kotlinc` every emitted `.kt` file into a classes directory
    /// inside the target, then `kotlin -cp` that directory with the entry
    /// class. Companion files [backend-companion] are `.kt` too, so they
    /// are part of `emitted` and compile with the rest — including the
    /// platform host [platform-tree].
    fn run(
        &self,
        target_dir: &Path,
        main_module: &ModulePath,
        emitted: &[PathBuf],
    ) -> Result<i32, BackendError> {
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

        let mut args: Vec<&OsStr> = sources.iter().map(|p| p.as_os_str()).collect();
        args.push(OsStr::new("-d"));
        args.push(classes.as_os_str());
        let code = run_tool("kotlinc", &args)?;
        if code != 0 {
            return Err(BackendError::Other(format!(
                "kotlinc failed with exit code {code}"
            )));
        }

        let entry = self.entry_hint(target_dir, main_module, emitted);
        run_tool(
            "kotlin",
            &[OsStr::new("-cp"), classes.as_os_str(), OsStr::new(&entry)],
        )
    }
}
