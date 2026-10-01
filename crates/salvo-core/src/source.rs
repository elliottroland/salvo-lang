//! Source-file discovery and classification.
//!
//! Salvo modules correspond to files: `list/ext.sv` is module `list.ext`.
//! Module paths come from the directory layout, so a `.sv` file name may not
//! itself contain a dot [mod-file-name] — the double-extension spelling that
//! used to select a backend's `define` file is not part of the language any
//! more.

use std::fmt;
use std::path::{Path, PathBuf};

/// Dotted module path, e.g. `core.string`.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModulePath(pub Vec<String>);

impl ModulePath {
    /// The path a rendered module name spells (`core.list`).
    pub fn parse(text: &str) -> ModulePath {
        ModulePath(text.split('.').map(str::to_string).collect())
    }

    /// [mod-suffix] Whether a written module reference names this module: the
    /// full path, or any **suffix** of it (`list` for `core.list`, `mem` for
    /// `fs.mem`). One rule for every place a module is referenced — `@module`
    /// selectors, `by`, `import` — with a reference that fits two modules
    /// refused as ambiguous rather than defaulted (user decision 2026-09-29).
    pub fn matches_suffix(&self, segs: &[&str]) -> bool {
        !segs.is_empty()
            && self.0.len() >= segs.len()
            && self.0[self.0.len() - segs.len()..].iter().map(|s| s.as_str()).eq(segs.iter().copied())
    }

    /// [mod-suffix] The same over rendered text (`core.list` against `list`).
    pub fn text_matches_suffix(module: &str, reference: &str) -> bool {
        module == reference || module.ends_with(&format!(".{reference}"))
    }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("."))
    }
}

impl fmt::Debug for ModulePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[derive(Clone, Debug)]
pub struct SourceFile {
    /// Display name (relative path) for diagnostics.
    pub name: String,
    pub module: ModulePath,
    pub content: String,
    /// True for files that come from the embedded standard library.
    pub is_std: bool,
    /// [std-shadow] True for an **on-disk** file that shadows (or is) a
    /// std module — `is_std` for the checker's rules, but a real path the
    /// tooling can hover, diagnose and link, which is how the standard
    /// library is developed.
    pub is_shadow: bool,
    /// [test-file] True for a `<name>.test.sv` **test annex**: the
    /// companion holding module `<name>`'s tests. Loaded only by
    /// `salvo test` (and by `salvo analyze`, which checks everything), so a
    /// production build never sees one; never `is_std`, whatever tree it
    /// lives in, so a test cannot declare an `intrinsic`
    /// [intrinsic-std-only] (user decision 2026-09-23).
    pub is_test: bool,
    /// [manifest-deps] The name of the **dependency** this file came from,
    /// for a file loaded from another project through `[dependencies]`.
    /// Its `name` is then an absolute path (it sits outside the source
    /// root); its `main` is never an entry point, its protocols are not the
    /// project's to lock, and it is std only if its own manifest says so.
    pub dependency: Option<String>,
}

/// The directory host files are written under **in the output** [platform-tree]:
/// `platform/` mirrors the source tree, so the host file of module
/// `app.entry` is emitted as `platform/app/entry.kt`. Where the files are
/// *read from* is the project's platform root [platform-root].
pub const PLATFORM_DIR: &str = "platform";

/// [platform-root] Where each backend's platform files — host companions
/// (`app/entry.kt`) and generated files (`app/entry.sv.kt`) — are read from, as the
/// project manifest names them (`[build] platform`, `[kotlin] platform`,
/// `[rust] platform`). Both may be one directory, which is how the two
/// languages coexist. A backend with no root has no platform files: there is
/// no default.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlatformRoots {
    pub kotlin: Option<PathBuf>,
    pub rust: Option<PathBuf>,
}

/// [platform-root] A file found under a platform root.
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformFile {
    /// `kotlin` or `rust`.
    pub backend: &'static str,
    pub module: ModulePath,
    /// A generated file (`<m>.sv.kt`: the ABI and interface files
    /// [platform-abi]) rather than an implementation file (`<m>.kt`).
    pub generated: bool,
}

impl PlatformRoots {
    pub fn get(&self, backend: &str) -> Option<&Path> {
        match backend {
            "kotlin" => self.kotlin.as_deref(),
            "rust" => self.rust.as_deref(),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.kotlin.is_none() && self.rust.is_none()
    }

    /// The distinct roots, each once.
    pub fn dirs(&self) -> Vec<&Path> {
        let mut out: Vec<&Path> = Vec::new();
        for r in [&self.kotlin, &self.rust].into_iter().flatten() {
            if !out.contains(&r.as_path()) {
                out.push(r);
            }
        }
        out
    }

    /// The roots made absolute and canonical, so a walked path compares.
    pub fn canonical(&self) -> PlatformRoots {
        let c = |p: &Option<PathBuf>| p.as_ref().map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()));
        PlatformRoots { kotlin: c(&self.kotlin), rust: c(&self.rust) }
    }

    /// What the file at `path` is, when it lies under a root: the backend its
    /// extension names — which must be a backend that root is configured for
    /// — its module (the path under the root, the way `.sv` files map), and
    /// whether it is a generated file. `Err` for a host file under a root that is
    /// not its language's. `Ok(None)` outside every root, or for a file that
    /// is not a host file.
    pub fn classify(&self, path: &Path) -> Result<Option<PlatformFile>, String> {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { return Ok(None) };
        let Some((stem, ext)) = name.rsplit_once('.') else { return Ok(None) };
        let backend = match ext {
            "kt" => "kotlin",
            "rs" => "rust",
            _ => return Ok(None),
        };
        let (stem, generated) = match stem.strip_suffix(".sv") {
            Some(s) => (s, true),
            None => (stem, false),
        };
        match self.get(backend).and_then(|root| path.strip_prefix(root).ok()) {
            Some(rel) => {
                let mut module: Vec<String> = rel
                    .parent()
                    .map(|p| p.components().filter_map(|c| c.as_os_str().to_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                module.push(stem.to_string());
                Ok(Some(PlatformFile { backend, module: ModulePath(module), generated }))
            }
            None => {
                let other = if backend == "kotlin" { "rust" } else { "kotlin" };
                if self.get(other).is_some_and(|root| path.starts_with(root)) {
                    return Err(format!(
                        "`{}` is a {} file under the {} platform root, which is not where {} \
                         platform files are read from [platform-root]",
                        path.display(),
                        if backend == "kotlin" { "Kotlin" } else { "Rust" },
                        if other == "kotlin" { "Kotlin" } else { "Rust" },
                        if backend == "kotlin" { "Kotlin" } else { "Rust" },
                    ));
                }
                Ok(None)
            }
        }
    }
}

/// A backend-native source file living next to a Salvo module
/// (`complicated.kt` beside `complicated.sv`): copied verbatim into the
/// output when its module is needed [backend-companion].
#[derive(Clone, Debug)]
pub struct CompanionFile {
    /// Path relative to the source root (also the output path).
    pub rel_path: PathBuf,
    /// The module the companion belongs to (same directory + stem).
    pub module: ModulePath,
    pub content: String,
    /// True for a file under the source root's `platform/` directory
    /// [platform-tree]: the host implementation of that module's platform
    /// effects, which is generated once by `salvo platform generate` and
    /// owned by the customer afterwards. It is a companion in every other
    /// respect — copied verbatim, gated on its module being reachable —
    /// but the backends single it out: it holds the program's real entry
    /// point.
    pub platform: bool,
}

/// [test-file] The file-name suffix marking a **test annex**: `heap.test.sv`
/// holds the tests of module `heap`, and becomes module `heap.test`.
pub const TEST_SUFFIX: &str = ".test";

/// The full set of sources for a compilation: user sources plus the
/// standard library.
#[derive(Debug, Default)]
pub struct SourceSet {
    pub files: Vec<SourceFile>,
    /// Backend-native companion files ([backend-companion]).
    pub companions: Vec<CompanionFile>,
    /// [platform-root] Where `add_dir` reads platform files from; empty
    /// means none are loaded (a `platform/` directory at the source root is
    /// then reported, never reinterpreted).
    pub platform: PlatformRoots,
}

impl SourceSet {
    /// The module a relative `.sv` path declares: the directory components
    /// plus the file stem (`list/ext.sv` -> `list.ext`).
    ///
    /// [test-file] One dot in a stem is permitted, and only one: the
    /// `.test` suffix of a test annex, which becomes a trailing path
    /// segment — `heap.test.sv` is module `heap.test`, the annex of module
    /// `heap` (user decision 2026-09-23). The narrowness is the point: the
    /// carve-out must not reopen the per-backend companion spelling
    /// (`string.kotlin.sv`), which this rule deliberately killed.
    ///
    /// `Err` carries a message for a name that cannot be a module
    /// [mod-file-name]: any other stem containing a dot, which is how a
    /// module path is spelled, so `list.ext.sv` would be indistinguishable
    /// from `list/ext.sv`. That spelling used to select a backend's `define`
    /// file and was skipped in silence; the define files are gone, and a
    /// leftover one now says so rather than being ignored.
    pub fn classify(rel_path: &Path) -> Result<ModulePath, String> {
        let file_name = rel_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("`{}` has no usable file name", rel_path.display()))?;
        let full_stem = file_name
            .strip_suffix(".sv")
            .ok_or_else(|| format!("`{file_name}` is not a `.sv` source file"))?;
        // [test-file] The annex: its stem without `.test`, plus `test` as a
        // trailing segment. That stem may not carry a further dot.
        let (stem, annex) = match full_stem.strip_suffix(TEST_SUFFIX) {
            Some(base) => (base, true),
            None => (full_stem, false),
        };
        if stem.contains('.') {
            // Name the path it collides with, in full: for
            // `core/list.kotlin.sv` that is `core/list/kotlin.sv`, and the
            // prefix is the part that makes the collision concrete.
            let mut nested = rel_path.with_file_name("");
            nested.push(stem.replace('.', "/"));
            nested.set_extension("sv");
            return Err(format!(
                "`{}`: a source file name may not contain a dot — a module path \
                 comes from the directory layout, so this is ambiguous with `{}` \
                 (the one exception is the `.test.sv` test annex)",
                rel_path.display(),
                nested.display()
            ));
        }
        if stem.is_empty() {
            return Err(format!(
                "`{}`: a test annex is named after the module it tests, so there \
                 has to be a name in front of `.test.sv` [test-file]",
                rel_path.display()
            ));
        }
        let mut components: Vec<String> = rel_path
            .parent()
            .map(|p| {
                p.components()
                    .filter_map(|c| c.as_os_str().to_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        components.push(stem.to_string());
        if annex {
            components.push(TEST_SUFFIX.trim_start_matches('.').to_string());
        }
        Ok(ModulePath(components))
    }

    /// [test-file] Whether a relative `.sv` path is a test annex
    /// (`heap.test.sv`).
    pub fn is_test_path(rel_path: &Path) -> bool {
        rel_path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".sv"))
            .is_some_and(|stem| stem.ends_with(TEST_SUFFIX))
    }

    pub fn add(
        &mut self,
        name: impl Into<String>,
        module: ModulePath,
        content: String,
        is_std: bool,
    ) {
        self.files.push(SourceFile {
            name: name.into(),
            module,
            content,
            is_std,
            is_shadow: false,
            is_test: false,
            dependency: None,
        });
    }

    /// [test-file] `add` for a test annex: never `is_std`, whatever tree it
    /// was loaded from (user decision 2026-09-23 — a test declares no
    /// `intrinsic`).
    pub fn add_test(&mut self, name: impl Into<String>, module: ModulePath, content: String) {
        self.files.push(SourceFile {
            name: name.into(),
            module,
            content,
            is_std: false,
            is_shadow: false,
            is_test: true,
            dependency: None,
        });
    }

    /// Classifies a backend-native companion file (`complicated.kt` for
    /// native extension `kt`): the module is the directory path plus the
    /// file stem [backend-companion]. A **platform** file is classified by
    /// its root instead ([`PlatformRoots::classify`]) [platform-root].
    pub fn classify_companion(rel_path: &Path, native_ext: &str) -> Option<ModulePath> {
        let file_name = rel_path.file_name()?.to_str()?;
        let stem = file_name.strip_suffix(&format!(".{native_ext}"))?;
        let mut components: Vec<String> = rel_path
            .parent()
            .map(|p| {
                p.components()
                    .filter_map(|c| c.as_os_str().to_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        components.push(stem.to_string());
        Some(ModulePath(components))
    }

    /// [platform-root] Where a platform file of `module` goes in the output:
    /// `platform/<module path>.<ext>`, whatever root it was read from.
    pub fn platform_output_path(module: &ModulePath, file_name_ext: &str) -> PathBuf {
        let mut path = PathBuf::from(PLATFORM_DIR);
        for part in &module.0 {
            path.push(part);
        }
        let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
        name.push(".");
        name.push(file_name_ext);
        path.set_file_name(name);
        path
    }

    pub fn add_companion(
        &mut self,
        rel_path: impl Into<PathBuf>,
        module: ModulePath,
        content: String,
        platform: bool,
    ) {
        self.companions.push(CompanionFile {
            rel_path: rel_path.into(),
            module,
            content,
            platform,
        });
    }

    /// Walks `root` recursively, adding every `.sv` source file plus every
    /// companion file with the backend's native extension
    /// ([backend-companion], e.g. `.kt` for Kotlin). Returns one rendered
    /// message per file that could not be read or could not be a module
    /// [mod-file-name] — a `.sv` file is never skipped in silence.
    ///
    /// [test-file] `tests` decides whether `<name>.test.sv` annexes are
    /// loaded: `salvo test` and `salvo analyze` load them, `compile` and
    /// `run` do not — which is the whole of how tests stay out of a
    /// production build (there is nothing to strip). A loaded annex is
    /// checked for its module: `heap.test.sv` needs a `heap.sv`, and cannot
    /// coexist with a `heap/test.sv`.
    ///
    /// Skipped during the walk [mod-ignore]:
    /// - hidden directories (`.git`, `.vscode`, ...),
    /// - cache directories carrying a `CACHEDIR.TAG` marker (the cachedir
    ///   spec; Cargo writes one into `target/`),
    /// - entries listed in `<root>/.svignore`: one path per line, relative
    ///   to the root, matching a file or a whole directory subtree;
    ///   blank lines and `#` comments are ignored.
    ///
    /// The `root` itself is exempt from the hidden/cache rules: explicitly
    /// selecting such a directory still works.
    pub fn add_dir(
        &mut self,
        root: &Path,
        native_ext: &str,
        is_std: bool,
        tests: bool,
    ) -> Vec<String> {
        let root = &root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        // [platform-root] std's tree carries its own manifest, which names its
        // platform root like any project's — for this tree only: the roots
        // are not kept for the next one loaded.
        let std_roots = (is_std && self.platform.is_empty())
            .then(|| crate::Project::load(&root.join(crate::MANIFEST_FILE)).ok())
            .flatten()
            .map(|project| project.platform_roots());
        let roots = std_roots.as_ref().unwrap_or(&self.platform).canonical();
        let ignored = read_svignore(root);
        let is_ignored = |path: &Path| {
            path.strip_prefix(root)
                .is_ok_and(|rel| ignored.iter().any(|entry| rel == entry))
        };

        let mut errors = Vec::new();
        // The source tree, then every platform root outside it.
        let mut stack = vec![root.to_path_buf()];
        for dir in roots.dirs() {
            if !dir.starts_with(root) && dir.is_dir() {
                stack.push(dir.to_path_buf());
            }
        }
        let mut paths = Vec::new();
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(err) => {
                    errors.push(format!("failed to read `{}`: {err}", dir.display()));
                    continue;
                }
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let hidden = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with('.'));
                    // [manifest-discovery] A nested project's tree is its own
                    // program, not part of this one.
                    if hidden
                        || path.join("CACHEDIR.TAG").is_file()
                        || is_ignored(&path)
                        || crate::manifest::is_nested_project(root, &path)
                    {
                        continue;
                    }
                    stack.push(path);
                } else if path
                    .extension()
                    .is_some_and(|e| e == "sv" || e == native_ext || e == "kt" || e == "rs")
                    && !is_ignored(&path)
                {
                    paths.push(path);
                }
            }
        }
        paths.sort();
        paths.dedup();
        for path in paths {
            // A file outside the source tree (a root beside it) is named by
            // its absolute path, as a dependency's files are.
            let rel = path.strip_prefix(root).unwrap_or(&path);
            // [platform-root] A platform file: classified by its root.
            match roots.classify(&path) {
                Err(msg) => {
                    errors.push(msg);
                    continue;
                }
                Ok(Some(pf)) => {
                    let content = match std::fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(err) => {
                            errors.push(format!("failed to read `{}`: {err}", path.display()));
                            continue;
                        }
                    };
                    let ext = if pf.backend == "kotlin" { "kt" } else { "rs" };
                    // [platform-abi] A generated file (`<m>.sv.kt`, the ABI
                    // and interface files) is for the host project's tooling
                    // only: the build generates its own (ABI.md D4).
                    if !pf.generated && ext == native_ext {
                        let out = Self::platform_output_path(&pf.module, ext);
                        self.add_companion(out, pf.module, content, true);
                    }
                    continue;
                }
                Ok(None) => {}
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            // [platform-root] A host file in a `platform/` directory at the
            // source root that no root covers: reported, never read as a
            // companion of a module called `platform.…`.
            if (ext == "kt" || ext == "rs")
                && rel.components().next().is_some_and(|c| c.as_os_str() == PLATFORM_DIR)
            {
                let backend = if ext == "kt" { "kotlin" } else { "rust" };
                errors.push(match roots.get(backend) {
                    None => format!(
                        "`{}` is a platform file, but no platform root is set for {backend}: name it in \
                         `salvo.toml` — `[build] platform = \"…\"` for every backend, or `[{backend}] \
                         platform` for this one [platform-root]",
                        rel.display()
                    ),
                    Some(r) => format!(
                        "`{}` is outside the {backend} platform root `{}`, so it is not read [platform-root]",
                        rel.display(),
                        r.display()
                    ),
                });
                continue;
            }
            if ext != "sv" && ext != native_ext {
                continue;
            }
            if ext == native_ext {
                let Some(module) = Self::classify_companion(rel, native_ext) else {
                    continue;
                };
                match std::fs::read_to_string(&path) {
                    Ok(content) => self.add_companion(rel.to_path_buf(), module, content, false),
                    Err(err) => {
                        errors.push(format!("failed to read `{}`: {err}", path.display()))
                    }
                }
                continue;
            }
            if !path.starts_with(root) {
                continue;
            }
            let module = match Self::classify(rel) {
                Ok(module) => module,
                Err(msg) => {
                    errors.push(msg);
                    continue;
                }
            };
            // [test-file] An annex is loaded only where tests are wanted.
            let annex = Self::is_test_path(rel);
            if annex && !tests {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(content) if annex => {
                    self.add_test(rel.display().to_string(), module, content)
                }
                Ok(content) => self.add(rel.display().to_string(), module, content, is_std),
                Err(err) => {
                    errors.push(format!("failed to read `{}`: {err}", path.display()))
                }
            }
        }
        if tests {
            errors.extend(self.check_annexes());
        }
        errors
    }

    /// [std-shadow] Lets a source tree **replace** modules of the embedded
    /// standard library: for every module a loaded file declares that an
    /// embedded std file also declares, the embedded copy is dropped and the
    /// file on disk takes over — std-ness included, so its `intrinsic`
    /// declarations stay legal [intrinsic-std-only].
    ///
    /// This is what makes `salvo test --src std` test *the checkout* rather
    /// than the std compiled into the binary (user decision 2026-09-23), and
    /// without it the two copies would collide as duplicate declarations
    /// [mod-collision] — a confusing error for what is a reasonable thing to
    /// do. Returns one note per replaced module, for a driver to report.
    /// [manifest] [std-shadow] Every non-std file becomes a std file — the
    /// tree declared `std = true` in its manifest. A shadowing file already
    /// is; this reaches the modules the embedded copy does not have.
    /// [manifest-deps] A dependency's files are left alone: they are std
    /// only if *their* manifest says so, and `add_dependency` has already
    /// settled that.
    pub fn mark_std_tree(&mut self) {
        for file in &mut self.files {
            if !file.is_std && file.dependency.is_none() {
                file.is_std = true;
                file.is_shadow = true;
            }
        }
    }

    pub fn apply_std_shadow(&mut self) -> Vec<String> {
        // [manifest-deps] Dependency files never shadow from here: a std
        // dependency was already merged as std, and a non-std one colliding
        // with std was already refused.
        let shadowed: Vec<ModulePath> = self
            .files
            .iter()
            .filter(|f| !f.is_std && !f.is_test && f.dependency.is_none())
            .map(|f| f.module.clone())
            .filter(|m| self.files.iter().any(|f| f.is_std && f.module == *m))
            .collect();
        if shadowed.is_empty() {
            return Vec::new();
        }
        self.files
            .retain(|f| !f.is_std || !shadowed.contains(&f.module));
        let mut notes = Vec::new();
        for file in &mut self.files {
            if !file.is_test && file.dependency.is_none() && shadowed.contains(&file.module) {
                file.is_std = true;
                file.is_shadow = true;
                notes.push(format!(
                    "`{}` shadows the embedded standard library's module `{}`",
                    file.name, file.module
                ));
            }
        }
        notes.sort();
        notes
    }

    /// [manifest-deps] Loads one **dependency**: the source tree of another
    /// project, found by `Project::dependencies`. Its files are walked like
    /// the project's own (companions included — a dependency's platform
    /// effects need their host files as much as the project's do) but with
    /// tests off, named by absolute path (they sit outside the source root),
    /// and tagged with the dependency's name.
    ///
    /// Std-ness follows the dependency's *own* manifest (user decision
    /// 2026-09-29): a `std = true` dependency is std and replaces the embedded
    /// modules it declares, as [std-shadow] lets a source tree do; any other
    /// dependency declaring a module the embedded standard library also
    /// declares is refused, naming the file — a library must not be able to
    /// redefine `core.list` on its users. Call this **before** the project's
    /// own tree is added, so the project's [std-shadow] sees the merged std.
    ///
    /// Returns one rendered message per problem.
    pub fn add_dependency(&mut self, dep: &crate::manifest::Dependency, native_ext: &str) -> Vec<String> {
        let src = dep.project.src();
        if !src.is_dir() {
            return vec![format!(
                "dependency `{}`: source directory `{}` does not exist [manifest-deps]",
                dep.name,
                src.display()
            )];
        }
        let root = src.canonicalize().unwrap_or(src);
        let mut loaded = SourceSet { platform: dep.project.platform_roots(), ..SourceSet::default() };
        let mut errors = loaded.add_dir(&root, native_ext, dep.project.is_std(), false);
        let is_std = dep.project.is_std();
        for file in &mut loaded.files {
            file.name = root.join(&file.name).display().to_string();
            file.dependency = Some(dep.name.clone());
            let embedded: Vec<usize> = self
                .files
                .iter()
                .enumerate()
                .filter(|(_, f)| f.is_std && !f.is_shadow && f.module == file.module)
                .map(|(i, _)| i)
                .collect();
            if embedded.is_empty() {
                continue;
            }
            if is_std {
                // [std-shadow] The dependency's copy takes over.
                file.is_shadow = true;
                for i in embedded.into_iter().rev() {
                    self.files.remove(i);
                }
            } else {
                errors.push(format!(
                    "`{}`: dependency `{}` declares module `{}`, which is the standard \
                     library's — a dependency may replace std modules only if its own \
                     manifest says `[project] std = true` [manifest-deps]",
                    file.name, dep.name, file.module
                ));
            }
        }
        self.files.extend(loaded.files);
        self.companions.extend(loaded.companions);
        errors
    }

    /// [test-file] The two things a test annex needs of its surroundings: a
    /// module to be the annex *of*, and no other file claiming its module
    /// path.
    ///
    /// The orphan case is a real mistake rather than an empty module — a
    /// renamed or deleted production file with its tests left behind — and
    /// the collision case is what the dot carve-out costs: `heap.test.sv`
    /// and `heap/test.sv` are one module path written two ways.
    fn check_annexes(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (idx, annex) in self.files.iter().enumerate() {
            if !annex.is_test {
                continue;
            }
            let mut tested = annex.module.0.clone();
            tested.pop();
            let tested = ModulePath(tested);
            if !self
                .files
                .iter()
                .any(|f| !f.is_test && f.module == tested)
            {
                errors.push(format!(
                    "`{}`: no module `{tested}` to test — a test annex is named after \
                     the module it belongs to, so this one is looking for a \
                     `{}.sv` beside it [test-file]",
                    annex.name,
                    tested.0.last().map(String::as_str).unwrap_or_default()
                ));
            }
            if let Some(other) = self
                .files
                .iter()
                .enumerate()
                .find(|(i, f)| *i != idx && f.module == annex.module)
                .map(|(_, f)| f)
            {
                errors.push(format!(
                    "`{}` and `{}` are both module `{}`: a test annex takes the \
                     module path of the file it is named after plus `test`, so the \
                     two spellings collide — rename one [test-file]",
                    annex.name, other.name, annex.module
                ));
            }
        }
        errors
    }
}

/// [platform-host-deps] The dependencies whose **platform code** a program can
/// reach: every module reachable by import from the project's own files (and
/// through the dependencies' files in turn) that belongs to a dependency and
/// has a companion. By imports rather than by use, so it can only include too
/// much — a dependency imported but unused keeps its libraries — never too
/// little.
pub fn dependencies_with_reached_platform(
    files: &[SourceFile],
    modules: &[salvo_syntax::ast::Module],
    companions: &[CompanionFile],
) -> std::collections::HashSet<String> {
    use salvo_syntax::ast::Item;
    let by_path: std::collections::HashMap<&ModulePath, usize> =
        files.iter().enumerate().map(|(i, f)| (&f.module, i)).collect();
    let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut stack: Vec<usize> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| !f.is_std && f.dependency.is_none())
        .map(|(i, _)| i)
        .collect();
    while let Some(i) = stack.pop() {
        if !seen.insert(i) {
            continue;
        }
        let Some(ast) = modules.get(i) else { continue };
        for item in &ast.items {
            let Item::Import(imp) = item else { continue };
            let segs: Vec<String> = imp.path.iter().map(|p| p.name.clone()).collect();
            // A whole-module import names the module; a named one, its parent.
            for cut in [segs.len(), segs.len().saturating_sub(1)] {
                if let Some(&j) = by_path.get(&ModulePath(segs[..cut].to_vec())) {
                    stack.push(j);
                }
            }
        }
    }
    let mut out = std::collections::HashSet::new();
    for &i in &seen {
        let f = &files[i];
        if let Some(dep) = &f.dependency {
            if companions.iter().any(|c| c.module == f.module) {
                out.insert(dep.clone());
            }
        }
    }
    out
}

/// Parses `<root>/.svignore` [mod-ignore]: one entry per line, `/`-separated
/// and relative to the root, naming a file or a directory subtree to skip.
/// Blank lines and lines starting with `#` are ignored; trailing `/` on
/// directory entries is allowed. Missing file means no ignores.
fn read_svignore(root: &Path) -> Vec<PathBuf> {
    let Ok(content) = std::fs::read_to_string(root.join(".svignore")) else {
        return Vec::new();
    };
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.trim_end_matches('/').split('/').collect::<PathBuf>())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_language_files() {
        let module = SourceSet::classify(Path::new("core/list.sv")).unwrap();
        assert_eq!(module.to_string(), "core.list");
    }

    /// [mod-file-name] A dotted stem is an error, not a skip: this is the
    /// spelling that used to select a backend's `define` file, and a
    /// leftover one must say so rather than vanish from the build.
    #[test]
    fn a_dotted_file_name_is_an_error() {
        let err = SourceSet::classify(Path::new("core/list.kotlin.sv")).unwrap_err();
        assert!(
            err.contains("may not contain a dot") && err.contains("core/list/kotlin.sv"),
            "unexpected message: {err}"
        );
        assert!(SourceSet::classify(Path::new("core/list.txt")).is_err());
    }

    /// [test-file] The one dot that is allowed: a test annex takes the module
    /// path of the file it is named after, plus `test`.
    #[test]
    fn a_test_annex_is_its_module_plus_test() {
        let module = SourceSet::classify(Path::new("heap.test.sv")).unwrap();
        assert_eq!(module.to_string(), "heap.test");
        let nested = SourceSet::classify(Path::new("core/list.test.sv")).unwrap();
        assert_eq!(nested.to_string(), "core.list.test");
        assert!(SourceSet::is_test_path(Path::new("core/list.test.sv")));
        assert!(!SourceSet::is_test_path(Path::new("core/list.sv")));
    }

    /// [test-file] The carve-out is exactly one suffix wide: it must not
    /// reopen the per-backend companion spelling, and an annex needs a name in
    /// front of it.
    #[test]
    fn the_annex_carve_out_stays_narrow() {
        assert!(SourceSet::classify(Path::new("list.kotlin.test.sv")).is_err());
        let err = SourceSet::classify(Path::new(".test.sv")).unwrap_err();
        assert!(err.contains("[test-file]"), "unexpected message: {err}");
    }

    /// [test-file] An annex is never `is_std`, whatever tree it came from, so a
    /// test cannot declare an `intrinsic` [intrinsic-std-only].
    #[test]
    fn an_annex_is_never_std() {
        let mut sources = SourceSet::default();
        sources.add_test(
            "heap.test.sv",
            SourceSet::classify(Path::new("heap.test.sv")).unwrap(),
            String::new(),
        );
        let file = &sources.files[0];
        assert!(file.is_test && !file.is_std);
    }

    /// [std-shadow] A tree declaring std's own modules replaces them, std-ness
    /// included — which is what `salvo test --src std` rests on.
    #[test]
    fn a_source_tree_can_shadow_std() {
        let mut sources = SourceSet::default();
        sources.add("std/heap.sv", ModulePath(vec!["heap".into()]), "embedded".into(), true);
        sources.add("std/core/list.sv", ModulePath(vec!["core".into(), "list".into()]), "embedded".into(), true);
        sources.add("heap.sv", ModulePath(vec!["heap".into()]), "on disk".into(), false);
        let notes = sources.apply_std_shadow();
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("`heap`"), "{notes:?}");
        let heap: Vec<&SourceFile> = sources
            .files
            .iter()
            .filter(|f| f.module.to_string() == "heap")
            .collect();
        assert_eq!(heap.len(), 1);
        assert_eq!(heap[0].content, "on disk");
        // It takes over std-ness, so its `intrinsic` declarations stay legal.
        assert!(heap[0].is_std);
        // An unshadowed std module is untouched.
        assert!(sources
            .files
            .iter()
            .any(|f| f.module.to_string() == "core.list" && f.is_std));
    }

    #[test]
    fn nested_modules() {
        let module = SourceSet::classify(Path::new("list/ext.sv")).unwrap();
        assert_eq!(module.to_string(), "list.ext");
    }

    /// [backend-companion] An ordinary companion is attributed to the
    /// module beside it.
    #[test]
    fn classifies_companion_files() {
        let module = SourceSet::classify_companion(Path::new("app/geometry.kt"), "kt").unwrap();
        assert_eq!(module.to_string(), "app.geometry");
        assert!(SourceSet::classify_companion(Path::new("app/geometry.rs"), "kt").is_none());
    }

    /// [platform-root] A platform file is attributed by its root to the
    /// module whose platform declarations it implements — which is what
    /// makes it reachable at all — and a generated file is told apart by its
    /// `.sv.<ext>` suffix. Two backends may share a root; a file of the
    /// other language under a backend's own root is an error.
    #[test]
    fn platform_files_are_classified_by_their_root() {
        let shared = PlatformRoots { kotlin: Some("/p/platform".into()), rust: Some("/p/platform".into()) };
        let f = shared.classify(Path::new("/p/platform/app/entry.kt")).unwrap().unwrap();
        assert_eq!((f.backend, f.module.to_string(), f.generated), ("kotlin", "app.entry".to_string(), false));
        let f = shared.classify(Path::new("/p/platform/main.sv.rs")).unwrap().unwrap();
        assert_eq!((f.backend, f.module.to_string(), f.generated), ("rust", "main".to_string(), true));
        assert_eq!(shared.classify(Path::new("/p/salvo/app/entry.kt")), Ok(None));
        let split = PlatformRoots { kotlin: Some("/p/kotlin".into()), rust: Some("/p/rust".into()) };
        assert!(split.classify(Path::new("/p/kotlin/main.rs")).is_err());
        assert_eq!(split.classify(Path::new("/p/rust/main.rs")).unwrap().unwrap().module.to_string(), "main");
        assert_eq!(PlatformRoots::default().classify(Path::new("/p/platform/main.kt")), Ok(None));
        assert_eq!(
            SourceSet::platform_output_path(&ModulePath(vec!["app".into(), "entry".into()]), "kt"),
            PathBuf::from("platform/app/entry.kt")
        );
    }
}
