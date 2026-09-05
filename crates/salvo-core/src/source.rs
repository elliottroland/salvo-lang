//! Source-file discovery and classification.
//!
//! Salvo modules correspond to files: `list/ext.sv` is module `list.ext`.
//! Backend define files use a double extension: `string.kotlin.sv` holds the
//! Kotlin `define` templates for module `string`. Files for other backends
//! are skipped entirely.

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    /// A language file (`foo.sv`).
    Language,
    /// A backend define file for the active backend (`foo.<backend>.sv`).
    BackendDefine,
}

#[derive(Clone, Debug)]
pub struct SourceFile {
    /// Display name (relative path) for diagnostics.
    pub name: String,
    pub module: ModulePath,
    pub kind: SourceKind,
    pub content: String,
    /// True for files that come from the embedded standard library.
    pub is_std: bool,
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

/// The full set of sources for a compilation: user sources plus the
/// (backend-filtered) standard library.
#[derive(Debug, Default)]
pub struct SourceSet {
    pub files: Vec<SourceFile>,
    /// Backend-native companion files ([backend-companion]).
    pub companions: Vec<CompanionFile>,
}

impl SourceSet {
    /// Classifies a relative `.sv` path for the given backend.
    ///
    /// Returns `None` when the file belongs to a different backend and
    /// should be skipped. `prefix` is prepended to the module path (e.g.
    /// `["core"]` — already part of the relative path for std files).
    pub fn classify(rel_path: &Path, backend: &str) -> Option<(ModulePath, SourceKind)> {
        let file_name = rel_path.file_name()?.to_str()?;
        let stem = file_name.strip_suffix(".sv")?;

        let (module_stem, kind) = match stem.rsplit_once('.') {
            Some((module, be)) if be == backend => (module, SourceKind::BackendDefine),
            Some((_, _)) => return None, // another backend's define file
            None => (stem, SourceKind::Language),
        };

        let mut components: Vec<String> = rel_path
            .parent()
            .map(|p| {
                p.components()
                    .filter_map(|c| c.as_os_str().to_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        components.push(module_stem.to_string());
        Some((ModulePath(components), kind))
    }

    pub fn add(
        &mut self,
        name: impl Into<String>,
        module: ModulePath,
        kind: SourceKind,
        content: String,
        is_std: bool,
    ) {
        self.files.push(SourceFile {
            name: name.into(),
            module,
            kind,
            content,
            is_std,
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

    /// Walks `root` recursively, adding every `.sv` file that matches the
    /// backend, plus every companion file with the backend's native
    /// extension ([backend-companion], e.g. `.kt` for Kotlin). Returns
    /// the paths that failed to read.
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
        backend: &str,
        native_ext: &str,
        is_std: bool,
    ) -> Vec<(PathBuf, String)> {
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
                    errors.push((dir, err.to_string()));
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
                    Err(err) => errors.push((path, err.to_string())),
                }
                continue;
            }
            let Some((module, kind)) = Self::classify(rel, backend) else {
                continue;
            };
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    self.add(rel.display().to_string(), module, kind, content, is_std)
                }
                Err(err) => errors.push((path, err.to_string())),
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
        let (module, kind) = SourceSet::classify(Path::new("core/list.sv"), "kotlin").unwrap();
        assert_eq!(module.to_string(), "core.list");
        assert_eq!(kind, SourceKind::Language);
    }

    #[test]
    fn classifies_backend_define_files() {
        let (module, kind) =
            SourceSet::classify(Path::new("core/list.kotlin.sv"), "kotlin").unwrap();
        assert_eq!(module.to_string(), "core.list");
        assert_eq!(kind, SourceKind::BackendDefine);
    }

    #[test]
    fn skips_other_backend_files() {
        assert!(SourceSet::classify(Path::new("core/list.rust.sv"), "kotlin").is_none());
        assert!(SourceSet::classify(Path::new("core/list.kotlin.sv"), "rust").is_none());
    }

    #[test]
    fn nested_modules() {
        let (module, _) = SourceSet::classify(Path::new("list/ext.sv"), "kotlin").unwrap();
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
