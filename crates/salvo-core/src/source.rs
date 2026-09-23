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
    /// [test-file] True for a `<name>.test.sv` **test annex**: the
    /// companion holding module `<name>`'s tests. Loaded only by
    /// `salvo test` (and by `salvo analyze`, which checks everything), so a
    /// production build never sees one; never `is_std`, whatever tree it
    /// lives in, so a test cannot declare an `intrinsic`
    /// [intrinsic-std-only] (user decision 2026-09-23).
    pub is_test: bool,
}

/// The source-root directory holding host implementations of platform
/// effects [platform-tree]: `platform/` mirrors the source tree, so
/// `platform/app/entry.kt` belongs to module `app.entry`.
pub const PLATFORM_DIR: &str = "platform";

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
            is_test: false,
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
            is_test: true,
        });
    }

    /// Classifies a backend-native companion file (`complicated.kt` for
    /// native extension `kt`): the module is the directory path plus the
    /// file stem [backend-companion].
    ///
    /// A leading `platform/` segment is *stripped* and reported as the
    /// second element [platform-tree]: `platform/app/entry.kt` implements
    /// the platform effects of module `app.entry`, so it must be attributed
    /// to that module — a companion is only copied when its module is
    /// reachable, and no Salvo module is ever called `platform.app.entry`.
    pub fn classify_companion(rel_path: &Path, native_ext: &str) -> Option<(ModulePath, bool)> {
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
        // Only at the source root: a nested `platform/` directory is an
        // ordinary one, and a module *named* `platform` keeps its name.
        let platform = components.first().map(String::as_str) == Some(PLATFORM_DIR);
        if platform {
            components.remove(0);
        }
        components.push(stem.to_string());
        Some((ModulePath(components), platform))
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
        let ignored = read_svignore(root);
        let is_ignored = |path: &Path| {
            path.strip_prefix(root)
                .is_ok_and(|rel| ignored.iter().any(|entry| rel == entry))
        };

        let mut errors = Vec::new();
        let mut stack = vec![root.to_path_buf()];
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
                    if hidden || path.join("CACHEDIR.TAG").is_file() || is_ignored(&path) {
                        continue;
                    }
                    stack.push(path);
                } else if path
                    .extension()
                    .is_some_and(|e| e == "sv" || e == native_ext)
                    && !is_ignored(&path)
                {
                    paths.push(path);
                }
            }
        }
        paths.sort();
        for path in paths {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            if path.extension().is_some_and(|e| e == native_ext) {
                let Some((module, platform)) = Self::classify_companion(rel, native_ext)
                else {
                    continue;
                };
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        self.add_companion(rel.to_path_buf(), module, content, platform)
                    }
                    Err(err) => {
                        errors.push(format!("failed to read `{}`: {err}", path.display()))
                    }
                }
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
    pub fn apply_std_shadow(&mut self) -> Vec<String> {
        let shadowed: Vec<ModulePath> = self
            .files
            .iter()
            .filter(|f| !f.is_std && !f.is_test)
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
            if !file.is_test && shadowed.contains(&file.module) {
                file.is_std = true;
                notes.push(format!(
                    "`{}` shadows the embedded standard library's module `{}`",
                    file.name, file.module
                ));
            }
        }
        notes.sort();
        notes
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
    /// module beside it and carries no platform flag.
    #[test]
    fn classifies_companion_files() {
        let (module, platform) =
            SourceSet::classify_companion(Path::new("app/geometry.kt"), "kt").unwrap();
        assert_eq!(module.to_string(), "app.geometry");
        assert!(!platform);
        assert!(SourceSet::classify_companion(Path::new("app/geometry.rs"), "kt").is_none());
    }

    /// [platform-tree] The leading `platform/` is stripped, so the host
    /// file is attributed to the module whose platform effects it
    /// implements — which is what makes it reachable at all.
    #[test]
    fn platform_companions_are_attributed_to_their_module() {
        let (module, platform) =
            SourceSet::classify_companion(Path::new("platform/main.kt"), "kt").unwrap();
        assert_eq!(module.to_string(), "main");
        assert!(platform);

        let (module, platform) =
            SourceSet::classify_companion(Path::new("platform/app/entry.rs"), "rs").unwrap();
        assert_eq!(module.to_string(), "app.entry");
        assert!(platform);
    }

    /// [platform-tree] Only the source root's `platform/` is special: a
    /// nested one is an ordinary directory, so a module *named* `platform`
    /// keeps its own companions.
    #[test]
    fn only_the_root_platform_directory_is_special() {
        let (module, platform) =
            SourceSet::classify_companion(Path::new("app/platform/host.kt"), "kt").unwrap();
        assert_eq!(module.to_string(), "app.platform.host");
        assert!(!platform);

        let (module, platform) =
            SourceSet::classify_companion(Path::new("platform.kt"), "kt").unwrap();
        assert_eq!(module.to_string(), "platform");
        assert!(!platform);
    }
}
