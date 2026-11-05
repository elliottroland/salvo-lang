//! `salvo docs`: a project's documentation as GitHub-wiki pages [cli-docs].
//!
//! The pages are Markdown, because a GitHub wiki serves nothing else: it
//! renders Markdown with a restricted subset of HTML (no scripts, no `style`
//! or `class` attributes), so the generator writes plain Markdown and uses
//! HTML only where GitHub keeps it (`<details>`, tables). That decides three
//! things (user decisions 2026-10-30):
//!
//! * **Code is fenced `rust`.** GitHub cannot highlight Salvo, and Rust's
//!   highlighting fits it closely. Sources write ```` ```salvo ```` (or a bare
//!   fence in a doc comment, where code is Salvo by default) and the generator
//!   rewrites the fence.
//! * **Finding things is an index page.** There is nowhere to run a search
//!   box, so `Index` lists every documented name, A to Z, with what it is,
//!   where it lives and its first sentence; GitHub's wiki search covers the
//!   rest.
//! * **Anchors are GitHub's own**: a `### fn map` heading is `#fn-map`, so a
//!   heading names its kind and links are computed the way GitHub computes
//!   them (`anchor`).
//!
//! The pages: `Home` (the `[docs] home` file, then the module list), one page
//! per topic (each file of `[docs] topics`, named by its file), one page per
//! module (`std.core.list` is `std-core-list`: dots in a wiki page name are
//! not reliable), `Index`, `_Sidebar` and `_Footer`. Every page opens with a
//! comment naming its source, since editing a generated page is lost work.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use salvo_core::Program;
use salvo_syntax::ast::{FieldDecl, FnDecl, Item, Module};

use crate::analysis::Analysis;
use crate::lsp;

/// What `salvo docs` writes, and from what (`[docs]` in `salvo.toml`, with
/// the command line's `--out` over it).
pub struct Config {
    /// The directory the pages are written into.
    pub out: PathBuf,
    /// The page that becomes `Home`, when there is one.
    pub home: Option<PathBuf>,
    /// The directory of topic pages, when there is one.
    pub topics: Option<PathBuf>,
    /// The project's name, for the pages' titles.
    pub title: String,
}

/// The generated pages: file name (`std-core-list.md`) → contents.
pub type Pages = BTreeMap<String, String>;

/// What kind of declaration an entry documents; also its heading word.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Kind {
    Struct,
    Type,
    Effect,
    Handler,
    Qualifier,
    Group,
    Fn,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::Struct => "struct",
            Kind::Type => "type",
            Kind::Effect => "effect",
            Kind::Handler => "handler",
            Kind::Qualifier => "qualifier",
            Kind::Group => "fns",
            Kind::Fn => "fn",
        }
    }

    /// The section a module page lists this kind under.
    fn section(self) -> &'static str {
        match self {
            Kind::Struct | Kind::Type => "Types",
            Kind::Effect => "Effects",
            Kind::Handler => "Handlers",
            Kind::Qualifier => "Qualifiers",
            Kind::Group => "Function groups",
            Kind::Fn => "Functions",
        }
    }

    /// How the index describes one.
    fn noun(self) -> &'static str {
        match self {
            Kind::Struct => "struct",
            Kind::Type => "type",
            Kind::Effect => "effect",
            Kind::Handler => "handler",
            Kind::Qualifier => "qualifier",
            Kind::Group => "function group",
            Kind::Fn => "function",
        }
    }
}

/// One documented declaration (a fn's overloads are several entries under
/// one heading).
struct Entry {
    kind: Kind,
    name: String,
    /// The source-like signature, as hover shows it.
    signature: String,
    docs: Vec<String>,
    /// The names local to the declaration (parameters, generics, fields):
    /// a `[name]` of one of them is the name in code, not a link.
    locals: Vec<String>,
    /// Extra Markdown after the docs: a struct's fields, an effect's
    /// members, what a struct or type promises.
    extra: Vec<String>,
}

/// One module's page.
struct Page {
    /// `std.core.list`.
    module: String,
    /// `std-core-list`.
    page: String,
    docs: Vec<String>,
    entries: Vec<Entry>,
}

/// Where a documented name is: the page and the anchor of its heading.
#[derive(Clone)]
struct Target {
    page: String,
    anchor: String,
    kind: Kind,
}

/// The wiki page a module is documented on: its path with dashes.
pub fn module_page(module: &str) -> String {
    module.replace('.', "-")
}

/// GitHub's anchor for a heading: lowercased, every character that is not a
/// letter, a digit, a space, `-` or `_` dropped, spaces made `-`.
pub fn anchor(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// The heading of an entry's name: `fn map`, `struct Mapping`.
fn heading(kind: Kind, name: &str) -> String {
    format!("{} {}", kind.word(), name)
}

/// The page's headings in the order they are rendered, one per name and
/// kind, each with its anchor. Two headings GitHub would anchor alike (`fn
/// ok` and `fn Ok`) are told apart the way GitHub does it: the second gets
/// `-1`, the third `-2`.
fn page_headings(page: &Page) -> Vec<(Kind, String, String)> {
    let mut order: Vec<(Kind, String)> = Vec::new();
    for e in &page.entries {
        if !order.iter().any(|(k, n)| *k == e.kind && *n == e.name) {
            order.push((e.kind, e.name.clone()));
        }
    }
    order.sort_by_key(|(k, _)| section_rank(*k));
    let mut seen: HashMap<String, usize> = HashMap::new();
    order
        .into_iter()
        .map(|(kind, name)| {
            let base = anchor(&heading(kind, &name));
            let n = seen.entry(base.clone()).or_insert(0);
            let a = if *n == 0 { base.clone() } else { format!("{base}-{n}") };
            *n += 1;
            (kind, name, a)
        })
        .collect()
}

/// The anchor of `name`'s heading of `kind` on `page`.
fn entry_anchor(page: &Page, kind: Kind, name: &str) -> String {
    page_headings(page).into_iter().find(|(k, n, _)| *k == kind && n == name).map(|(_, _, a)| a).unwrap_or_else(|| anchor(&heading(kind, name)))
}

/// The banner every generated page opens with.
fn banner(source: &str) -> String {
    format!("<!-- Generated by `salvo docs` from {source}. Edit the source, not this page. -->\n\n")
}

/// Which files are the project's own: std's tree when the project is std,
/// else every file that is not std or a dependency. Test annexes are never
/// documentation.
fn own_files(program: &Program, std_project: bool) -> Vec<usize> {
    program
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.name.ends_with(".sv") && crate::analysis::own_file(f, std_project))
        .map(|(i, _)| i)
        .collect()
}

/// Builds every page of `analysis`'s project under `cfg`. Errors name what
/// could not be read (a missing home file or topics directory).
pub fn generate(analysis: &Analysis, cfg: &Config) -> Result<Pages, String> {
    let program = &analysis.program;
    let std_project = analysis.project.as_ref().is_some_and(|p| p.manifest.project.std);
    let files = own_files(program, std_project);
    let mut pages: Vec<Page> = files.iter().map(|&i| collect_module(analysis, i)).collect();
    pages.retain(|p| !p.entries.is_empty() || !p.docs.iter().all(|l| l.trim().is_empty()));
    pages.sort_by(|a, b| a.module.cmp(&b.module));

    // Every documented name, for `[symbol]` links and the index.
    let mut targets: HashMap<String, Vec<Target>> = HashMap::new();
    for page in &pages {
        let headings = page_headings(page);
        for (kind, name, anchor) in headings {
            targets.entry(name.clone()).or_default().push(Target { page: page.page.clone(), anchor, kind });
        }
    }
    let modules: HashMap<String, String> = pages.iter().map(|p| (p.module.clone(), p.page.clone())).collect();
    let links = Links { targets: &targets, modules: &modules };

    let mut out = Pages::new();
    for page in &pages {
        out.insert(format!("{}.md", page.page), render_module(page, &links));
    }
    out.insert("Index.md".to_string(), render_index(&pages, &links));

    // The topics, in the order `home` links them, the rest by name.
    let home_text = match &cfg.home {
        Some(path) => Some(std::fs::read_to_string(path).map_err(|e| format!("cannot read the docs home `{}`: {e}", path.display()))?),
        None => None,
    };
    let mut topics: Vec<(String, PathBuf)> = Vec::new();
    if let Some(dir) = &cfg.topics {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("cannot read the docs topics `{}`: {e}", dir.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") && cfg.home.as_ref().is_none_or(|h| !same_file(h, &path)) {
                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                topics.push((stem, path));
            }
        }
    }
    let order: Vec<String> = home_text.as_deref().map(linked_pages).unwrap_or_default();
    topics.sort_by_key(|(stem, _)| (order.iter().position(|o| o == stem).unwrap_or(usize::MAX), stem.clone()));
    let topic_names: HashSet<String> = topics.iter().map(|(s, _)| s.clone()).collect();
    let src_root = analysis.root.clone();
    let module_files: HashMap<PathBuf, String> = files
        .iter()
        .filter_map(|&i| {
            let f = &program.files[i];
            let path = canonical(&src_root.join(&f.name));
            Some((path, module_page(&f.module.to_string())))
        })
        .collect();
    for (stem, path) in &topics {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
        let body = topic_page(&text, path, &topic_names, &module_files, true);
        out.insert(format!("{stem}.md"), format!("{}{body}", banner(&format!("`{}`", display_path(path)))));
    }

    // Home: the home file, then the library.
    let mut home = String::new();
    match (&cfg.home, &home_text) {
        (Some(path), Some(text)) => {
            home.push_str(&banner(&format!("`{}`", display_path(path))));
            home.push_str(&topic_page(text, path, &topic_names, &module_files, false));
        }
        _ => {
            home.push_str(&banner("the project's modules"));
            home.push_str(&format!("The documentation of **{}**.\n", cfg.title));
        }
    }
    if !home.ends_with('\n') {
        home.push('\n');
    }
    home.push_str("\n## Library\n\nEvery module, and an [index](Index) of every documented name.\n\n");
    home.push_str(&module_table(&pages));
    out.insert("Home.md".to_string(), home);

    out.insert("_Sidebar.md".to_string(), render_sidebar(&topics, &pages, cfg.home.is_some()));
    out.insert("_Footer.md".to_string(), format!("{}Generated by `salvo docs` from {}'s sources.\n", banner("the project"), cfg.title));
    Ok(out)
}

fn same_file(a: &Path, b: &Path) -> bool {
    canonical(a) == canonical(b)
}

fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// A path for the banner: relative to the working directory where it can be.
fn display_path(p: &Path) -> String {
    let cwd = std::env::current_dir().ok().map(|c| canonical(&c));
    let full = canonical(p);
    // The repository root is the nearest ancestor holding `.git`, so a banner
    // reads the same wherever the command ran.
    let mut root = full.parent();
    while let Some(dir) = root {
        if dir.join(".git").exists() {
            return full.strip_prefix(dir).map(|r| r.display().to_string()).unwrap_or_else(|_| full.display().to_string());
        }
        root = dir.parent();
    }
    match cwd {
        Some(c) => full.strip_prefix(&c).map(|r| r.display().to_string()).unwrap_or_else(|_| full.display().to_string()),
        None => full.display().to_string(),
    }
}

/// The page stems a home file links, in order: `](Data-and-Types.md)`.
fn linked_pages(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("](") {
        rest = &rest[at + 2..];
        let end = rest.find(')').unwrap_or(rest.len());
        let target = &rest[..end];
        let file = target.split('#').next().unwrap_or("");
        if let Some(stem) = file.strip_suffix(".md") {
            if !stem.contains('/') && !out.iter().any(|s| s == stem) {
                out.push(stem.to_string());
            }
        }
    }
    out
}

/// A topic (or the home file) as a wiki page: its first `# Title` dropped
/// when `drop_title` (the wiki titles a page by its file), page links without
/// `.md`, links to a module's `.sv` file made links to its page, and `salvo`
/// fences made `rust`.
fn topic_page(text: &str, path: &Path, topics: &HashSet<String>, modules: &HashMap<PathBuf, String>, drop_title: bool) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    if drop_title {
        if let Some(first) = lines.iter().position(|l| !l.trim().is_empty()) {
            if lines[first].starts_with("# ") {
                lines.remove(first);
                while lines.first().is_some_and(|l| l.trim().is_empty()) {
                    lines.remove(0);
                }
            }
        }
    }
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut out = String::new();
    let mut fence = Fences::default();
    for line in lines {
        if let Some(rewritten) = fence.line(line) {
            out.push_str(&rewritten);
            out.push('\n');
            continue;
        }
        if fence.open() {
            out.push_str(line);
        } else {
            out.push_str(&rewrite_links(line, &dir, topics, modules));
        }
        out.push('\n');
    }
    out
}

/// Rewrites the targets of `](…)` links in one line of a topic.
fn rewrite_links(line: &str, dir: &Path, topics: &HashSet<String>, modules: &HashMap<PathBuf, String>) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find("](") {
        out.push_str(&rest[..at + 2]);
        rest = &rest[at + 2..];
        let Some(end) = rest.find(')') else { break };
        let target = &rest[..end];
        let (file, frag) = match target.split_once('#') {
            Some((f, a)) => (f, Some(a)),
            None => (target, None),
        };
        let frag_text = frag.map(|a| format!("#{a}")).unwrap_or_default();
        let rewritten = if let Some(stem) = file.strip_suffix(".md").filter(|s| !s.contains('/') && topics.contains(*s)) {
            format!("{stem}{frag_text}")
        } else if file.ends_with(".sv") {
            match modules.get(&canonical(&dir.join(file))) {
                Some(page) => format!("{page}{frag_text}"),
                None => target.to_string(),
            }
        } else {
            target.to_string()
        };
        out.push_str(&rewritten);
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// The documented declarations of file `i`: exported ones, in source order.
fn collect_module(analysis: &Analysis, i: usize) -> Page {
    let program = &analysis.program;
    let file = &program.files[i];
    let module: &Module = &program.modules[i];
    let path = file.module.to_string();
    let mut entries: Vec<Entry> = Vec::new();
    for (idx, item) in module.items.iter().enumerate() {
        match item {
            Item::Fn(f) if f.exported && f.stamped.is_none() && !lsp::is_generated_next(f) && !f.name.name.starts_with("__") => {
                let signature = analysis
                    .checked
                    .as_ref()
                    .and_then(|c| c.decls.item(i, idx).and_then(|key| lsp::fn_signature(program, c, key)))
                    .or_else(|| lsp::iter_fn_signature(module, f))
                    .unwrap_or_else(|| lsp::fn_decl_signature(f, None));
                entries.push(Entry { kind: Kind::Fn, name: display_name(f), signature, docs: f.docs.clone(), locals: fn_locals(f), extra: Vec::new() });
            }
            Item::Struct(s) if s.exported && !s.name.name.starts_with("__") => {
                let mut signature = lsp::struct_signature(s);
                if !s.obligations.is_empty() {
                    let obs: Vec<String> = s.obligations.iter().map(|o| o.group.to_string()).collect();
                    signature.push_str(&format!(" : {}", obs.join(", ")));
                }
                let mut locals: Vec<String> = s.generics.iter().map(|g| g.name.clone()).collect();
                locals.extend(s.fields.iter().map(|f| f.name.name.clone()));
                let extra = if s.opaque { vec!["*Opaque: its fields are visible only in its own module.*".to_string()] } else { field_list(&s.fields) };
                entries.push(Entry { kind: Kind::Struct, name: s.name.name.clone(), signature, docs: s.docs.clone(), locals, extra });
            }
            Item::Type(t) if t.exported => {
                let mut signature = lsp::type_signature(t);
                if t.platform {
                    signature = format!("platform {signature}");
                }
                if !t.obligations.is_empty() {
                    let obs: Vec<String> = t.obligations.iter().map(|o| o.group.to_string()).collect();
                    signature.push_str(&format!(" : {}", obs.join(", ")));
                }
                let locals = t.generics.iter().map(|g| g.name.clone()).collect();
                entries.push(Entry { kind: Kind::Type, name: t.name.name.clone(), signature, docs: t.docs.clone(), locals, extra: Vec::new() });
            }
            Item::Effect(e) if e.exported => {
                let mut signature = lsp::effect_signature(e);
                if e.is_actor {
                    signature = format!("actor {signature}");
                }
                let locals = e.generics.iter().map(|g| g.name.clone()).collect();
                let extra = member_list("Members", &e.fns);
                entries.push(Entry { kind: Kind::Effect, name: e.name.name.clone(), signature, docs: e.docs.clone(), locals, extra });
            }
            Item::Handler(h) if h.exported => {
                let mut signature = lsp::handler_signature(h);
                if h.platform {
                    signature = format!("{}platform {signature}", if h.threadsafe { "threadsafe " } else { "" });
                }
                let mut locals: Vec<String> = h.generics.iter().map(|g| g.name.clone()).collect();
                locals.extend(h.params.iter().map(|p| p.name.name.clone()));
                entries.push(Entry { kind: Kind::Handler, name: h.name.name.clone(), signature, docs: h.docs.clone(), locals, extra: Vec::new() });
            }
            Item::Qualifier(q) if q.exported => {
                let locals = q.generics.iter().map(|g| g.name.clone()).collect();
                entries.push(Entry { kind: Kind::Qualifier, name: q.name.name.clone(), signature: lsp::qualifier_signature(q), docs: q.docs.clone(), locals, extra: Vec::new() });
            }
            Item::Fns(g) if g.exported => {
                let locals = g.generics.iter().map(|g| g.name.clone()).collect();
                entries.push(Entry { kind: Kind::Group, name: g.name.name.clone(), signature: lsp::fns_signature(g), docs: g.docs.clone(), locals, extra: Vec::new() });
            }
            _ => {}
        }
    }
    // A `comptime fn` is gone from the expanded module (the stamping consumed
    // it), so it is read from the file as written. Its signature is the
    // declaration's own.
    if !module.items.iter().any(|it| matches!(it, Item::Fn(f) if f.compfn.is_some())) {
        let (written, _) = salvo_syntax::parse_module(&file.content);
        for item in &written.items {
            if let Item::Fn(f) = item {
                if f.exported && f.compfn.is_some() {
                    let mut signature = format!("comptime {}", lsp::fn_decl_signature(f, None));
                    if let Some((kind, param)) = f.compfn.as_ref().and_then(|c| c.bound.as_ref()) {
                        let p = &param.name;
                        let bounded = format!("<{p} is {}", kind.word());
                        signature = signature.replacen(&format!("<{p}>"), &format!("{bounded}>"), 1).replacen(&format!("<{p},"), &format!("{bounded},"), 1);
                    }
                    entries.push(Entry { kind: Kind::Fn, name: display_name(f), signature, docs: f.docs.clone(), locals: fn_locals(f), extra: Vec::new() });
                }
            }
        }
    }
    Page { page: module_page(&path), module: path, docs: module.docs.clone(), entries }
}

/// A fn's documented name: a type-named constructor reads as its type.
fn display_name(f: &FnDecl) -> String {
    match &f.type_name {
        Some(salvo_syntax::ast::Type::Named { base, .. }) => base.name.name.clone(),
        _ => f.name.name.clone(),
    }
}

fn fn_locals(f: &FnDecl) -> Vec<String> {
    let mut out: Vec<String> = f.params.iter().map(|p| p.name.name.clone()).collect();
    out.extend(f.generics.iter().map(|g| g.name.clone()));
    out
}

/// A struct's fields, as a list: name, type, and the field's own first
/// paragraph.
fn field_list(fields: &[FieldDecl]) -> Vec<String> {
    let visible: Vec<&FieldDecl> = fields.iter().filter(|f| !f.name.name.starts_with("__")).collect();
    if visible.is_empty() {
        return Vec::new();
    }
    let mut out = vec!["**Fields**".to_string(), String::new()];
    for f in visible {
        let ty = f.ty.as_ref().map(|t| format!(": {t}")).unwrap_or_default();
        let doc = first_paragraph(&prose(&f.docs));
        if doc.is_empty() {
            out.push(format!("- `{}{ty}`", f.name.name));
        } else {
            out.push(format!("- `{}{ty}` — {doc}", f.name.name));
        }
    }
    out
}

/// An effect's members: each signature and its docs.
fn member_list(title: &str, fns: &[FnDecl]) -> Vec<String> {
    if fns.is_empty() {
        return Vec::new();
    }
    let mut out = vec![format!("**{title}**"), String::new()];
    for f in fns {
        let doc = first_paragraph(&prose(&f.docs));
        let sig = lsp::fn_decl_signature(f, None);
        if doc.is_empty() {
            out.push(format!("- `{sig}`"));
        } else {
            out.push(format!("- `{sig}` — {doc}"));
        }
    }
    out
}

/// A doc block's prose, its tags removed.
fn prose(docs: &[String]) -> Vec<String> {
    salvo_syntax::doc::split(docs).prose
}

/// The first paragraph, joined into one line, without fences.
fn first_paragraph(lines: &[String]) -> String {
    let mut out: Vec<&str> = Vec::new();
    for line in lines.iter().skip_while(|l| l.trim().is_empty()) {
        if line.trim().is_empty() || line.trim_start().starts_with("```") {
            break;
        }
        out.push(line.trim());
    }
    out.join(" ")
}

/// The first sentence of a doc block, for a summary: up to the first `. `
/// outside backticks, or the first paragraph.
fn summary(docs: &[String]) -> String {
    let para = first_paragraph(&prose(docs));
    let mut in_code = false;
    let bytes: Vec<char> = para.chars().collect();
    for (i, c) in bytes.iter().enumerate() {
        if *c == '`' {
            in_code = !in_code;
        }
        if *c == '.' && !in_code && bytes.get(i + 1).is_none_or(|n| *n == ' ') {
            return bytes[..=i].iter().collect();
        }
    }
    para
}

/// Where `[symbol]` references may point.
struct Links<'a> {
    targets: &'a HashMap<String, Vec<Target>>,
    modules: &'a HashMap<String, String>,
}

impl Links<'_> {
    /// The link for `name` from `page`, or `None` when it names nothing
    /// documented (or several things, none of them on this page).
    fn resolve(&self, name: &str, page: &str) -> Option<String> {
        if let Some(target) = self.modules.get(name) {
            return Some(target.clone());
        }
        let all = self.targets.get(name)?;
        let here: Vec<&Target> = all.iter().filter(|t| t.page == page).collect();
        let pick = match here.as_slice() {
            [one] => Some(*one),
            [] => {
                let pages: HashSet<&str> = all.iter().map(|t| t.page.as_str()).collect();
                if pages.len() == 1 {
                    all.first()
                } else {
                    // A type wins over a fn of the same name elsewhere: `[List]`
                    // is the type, and each module's `size` is its own.
                    let types: Vec<&Target> = all.iter().filter(|t| t.kind != Kind::Fn).collect();
                    match types.as_slice() {
                        [one] => Some(*one),
                        _ => None,
                    }
                }
            }
            _ => here.iter().find(|t| t.kind != Kind::Fn).copied().or(here.first().copied()),
        }?;
        Some(if pick.page == page { format!("#{}", pick.anchor) } else { format!("{}#{}", pick.page, pick.anchor) })
    }
}

/// A doc block as wiki Markdown: `[symbol]` references linked or made code,
/// code fences in Salvo made `rust`, and the tags as sections (the compiler's
/// own tags, `@compilerrules` and `@compilerknown`, left out).
fn render_docs(docs: &[String], locals: &[String], page: &str, links: &Links) -> String {
    let parts = salvo_syntax::doc::split(docs);
    let mut out: Vec<String> = Vec::new();
    let mut fence = Fences { bare_is_salvo: true, ..Fences::default() };
    for line in &parts.prose {
        if let Some(rewritten) = fence.line(line) {
            out.push(rewritten);
            continue;
        }
        out.push(if fence.open() { line.clone() } else { link_refs(line, locals, page, links) });
    }
    while out.first().is_some_and(|l| l.trim().is_empty()) {
        out.remove(0);
    }
    let sections: [(&str, &str); 6] =
        [("param", "Parameters"), ("returns", "Returns"), ("throws", "Throws"), ("whynot", "Why not"), ("see", "See also"), ("todo", "To do")];
    for (tag, title) in sections {
        let items: Vec<String> = parts
            .tags
            .iter()
            .filter(|t| t.name == tag)
            .map(|t| match &t.target {
                Some(p) => format!("- `{p}` — {}", link_refs(&t.text, locals, page, links)),
                None => format!("- {}", link_refs(&t.text, locals, page, links)),
            })
            .collect();
        if !items.is_empty() {
            out.push(String::new());
            out.push(format!("**{title}**"));
            out.push(String::new());
            out.extend(items);
        }
    }
    out.join("\n").trim_end().to_string()
}

/// The code fences of a page, line by line: a fence opens with three or
/// more backticks and closes on a line of at least as many and nothing else,
/// so a longer fence can show a shorter one. A Salvo fence is written `rust`
/// (GitHub cannot highlight Salvo): ```` ```salvo ````, ```` ```no-check ````
/// [doc-example], and in a doc comment a bare fence.
#[derive(Default)]
struct Fences {
    ticks: usize,
    bare_is_salvo: bool,
}

impl Fences {
    fn open(&self) -> bool {
        self.ticks > 0
    }

    /// The line as written to the page when it opens or closes a fence;
    /// `None` for any other line.
    fn line(&mut self, line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        let ticks = trimmed.chars().take_while(|c| *c == '`').count();
        if ticks < 3 {
            return None;
        }
        if self.open() {
            if ticks >= self.ticks && trimmed.trim_end().len() == ticks {
                self.ticks = 0;
                return Some(line.to_string());
            }
            return None;
        }
        self.ticks = ticks;
        let lang = trimmed[ticks..].trim();
        let salvo = lang == "salvo" || lang == "no-check" || (self.bare_is_salvo && lang.is_empty());
        let indent = &line[..line.len() - trimmed.len()];
        Some(if salvo { format!("{indent}{}rust", "`".repeat(ticks)) } else { line.to_string() })
    }
}

/// Rewrites the `[symbol]` references of one prose line. A reference is a
/// bracketed name not followed by `(` or `[` (which would make it a Markdown
/// link already) and outside inline code; anything else is left as written.
fn link_refs(line: &str, locals: &[String], page: &str, links: &Links) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    let mut in_code = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            in_code = !in_code;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '[' && !in_code && (i == 0 || chars[i - 1] != '!') {
            if let Some(len) = chars[i + 1..].iter().position(|c| *c == ']') {
                let name: String = chars[i + 1..i + 1 + len].iter().collect();
                let after = chars.get(i + 2 + len).copied();
                let is_name = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') && name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_');
                if is_name && after != Some('(') && after != Some('[') {
                    if locals.iter().any(|l| *l == name) {
                        out.push_str(&format!("`{name}`"));
                    } else if let Some(target) = links.resolve(&name, page) {
                        out.push_str(&format!("[`{name}`]({target})"));
                    } else {
                        out.push_str(&format!("`{name}`"));
                    }
                    i += len + 2;
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// One module's page.
fn render_module(page: &Page, links: &Links) -> String {
    let mut out = banner(&format!("module `{}`", page.module));
    out.push_str(&format!("Module **`{}`**\n\n", page.module));
    let docs = render_docs(&page.docs, &[], &page.page, links);
    if !docs.is_empty() {
        out.push_str(&docs);
        out.push_str("\n\n");
    }
    if page.entries.is_empty() {
        return out;
    }
    // The contents: one row per name and kind, in the order of the sections.
    let headings = page_headings(page);
    let order: Vec<(Kind, String, &Entry, String)> = headings
        .into_iter()
        .filter_map(|(k, n, a)| page.entries.iter().find(|e| e.kind == k && e.name == n).map(|e| (k, n, e, a)))
        .collect();
    out.push_str("## Contents\n\n| | Name | Summary |\n|---|---|---|\n");
    for (kind, name, e, anchor) in &order {
        let sum = link_refs(&summary(&e.docs), &e.locals, &page.page, links).replace('|', "\\|");
        out.push_str(&format!("| {} | [`{name}`](#{anchor}) | {sum} |\n", kind.word()));
    }
    out.push('\n');
    let mut section = "";
    for (kind, name, _, _) in &order {
        if kind.section() != section {
            section = kind.section();
            out.push_str(&format!("## {section}\n\n"));
        }
        out.push_str(&format!("### {}\n\n", heading(*kind, name)));
        for e in page.entries.iter().filter(|e| e.kind == *kind && e.name == *name) {
            out.push_str(&format!("```rust\n{}\n```\n\n", e.signature));
            let docs = render_docs(&e.docs, &e.locals, &page.page, links);
            if !docs.is_empty() {
                out.push_str(&docs);
                out.push_str("\n\n");
            }
            if !e.extra.is_empty() {
                out.push_str(&e.extra.iter().map(|l| link_refs(l, &e.locals, &page.page, links)).collect::<Vec<_>>().join("\n"));
                out.push_str("\n\n");
            }
        }
    }
    out
}

fn section_rank(k: Kind) -> u8 {
    match k {
        Kind::Struct | Kind::Type => 0,
        Kind::Effect => 1,
        Kind::Handler => 2,
        Kind::Qualifier => 3,
        Kind::Group => 4,
        Kind::Fn => 5,
    }
}

/// The modules as a table: name and the first sentence of its docs.
fn module_table(pages: &[Page]) -> String {
    let mut out = String::from("| Module | Summary |\n|---|---|\n");
    for p in pages {
        let sum = summary(&p.docs).replace('|', "\\|");
        let sum = strip_refs(&sum);
        out.push_str(&format!("| [`{}`]({}) | {sum} |\n", p.module, p.page));
    }
    out
}

/// `[name]` made `` `name` `` (a summary away from its page links nothing).
fn strip_refs(text: &str) -> String {
    let empty: HashMap<String, Vec<Target>> = HashMap::new();
    let none: HashMap<String, String> = HashMap::new();
    link_refs(text, &[], "", &Links { targets: &empty, modules: &none })
}

/// Every documented name, A to Z, grouped by first letter.
fn render_index(pages: &[Page], links: &Links) -> String {
    let mut rows: Vec<(String, Kind, &Page, &Entry)> = Vec::new();
    for p in pages {
        let mut seen: HashSet<(Kind, String)> = HashSet::new();
        for e in &p.entries {
            if seen.insert((e.kind, e.name.clone())) {
                rows.push((e.name.clone(), e.kind, p, e));
            }
        }
    }
    rows.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()).then(a.0.cmp(&b.0)).then(a.2.module.cmp(&b.2.module)));
    let mut out = banner("the project's modules");
    out.push_str("Every documented name, A to Z: what it is, the module it is in, and the first sentence of its documentation. GitHub's wiki search finds the rest.\n\n");
    let mut letters: Vec<char> = rows.iter().filter_map(|r| r.0.chars().next()).map(|c| c.to_ascii_uppercase()).collect();
    letters.dedup();
    let nav: Vec<String> = letters.iter().map(|l| format!("[{l}](#{})", l.to_ascii_lowercase())).collect();
    out.push_str(&nav.join(" · "));
    out.push_str("\n\n");
    let mut current: Option<char> = None;
    for (name, kind, page, entry) in rows {
        let letter = name.chars().next().map(|c| c.to_ascii_uppercase());
        if letter != current {
            current = letter;
            out.push_str(&format!("\n## {}\n\n| Name | Kind | Module | Summary |\n|---|---|---|---|\n", letter.unwrap_or('?')));
        }
        let sum = link_refs(&summary(&entry.docs), &entry.locals, &page.page, links).replace('|', "\\|");
        let sum = absolute_links(&sum, &page.page);
        out.push_str(&format!(
            "| [`{name}`]({}#{}) | {} | [`{}`]({}) | {sum} |\n",
            page.page,
            entry_anchor(page, kind, &name),
            kind.noun(),
            page.module,
            page.page
        ));
    }
    out
}

/// A link rendered for `page` (`(#anchor)`), made to work from another page.
fn absolute_links(text: &str, page: &str) -> String {
    text.replace("](#", &format!("]({page}#"))
}

/// The sidebar: the overview in reading order, the library by package
/// (collapsed), and the index.
fn render_sidebar(topics: &[(String, PathBuf)], pages: &[Page], has_home: bool) -> String {
    let mut out = banner("the docs configuration");
    out.push_str("**Overview**\n\n* [Home](Home)\n");
    for (stem, _) in topics {
        out.push_str(&format!("* [{}]({stem})\n", stem.replace('-', " ")));
    }
    let _ = has_home;
    out.push_str("\n**Library**\n\n");
    let mut groups: BTreeMap<String, Vec<&Page>> = BTreeMap::new();
    for p in pages {
        let parts: Vec<&str> = p.module.split('.').collect();
        let group = if parts.len() > 2 { parts[..2].join(".") } else { parts[..1].join(".") };
        groups.entry(group).or_default().push(p);
    }
    for (group, members) in groups {
        out.push_str(&format!("<details><summary><code>{group}</code></summary>\n\n"));
        for p in members {
            out.push_str(&format!("* [`{}`]({})\n", p.module, p.page));
        }
        out.push_str("\n</details>\n\n");
    }
    out.push_str("**[Index](Index)**\n");
    out
}

/// Writes `pages` into `out`, removing the generated pages it no longer has
/// (every `.md` there is the generator's), or, with `check`, reports what
/// differs. Answers the number of pages written or checked.
pub fn write(pages: &Pages, out: &Path, check: bool) -> Result<usize, Vec<String>> {
    let existing: Vec<String> = std::fs::read_dir(out)
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".md")).collect())
        .unwrap_or_default();
    if check {
        let mut stale = Vec::new();
        for (name, text) in pages {
            match std::fs::read_to_string(out.join(name)) {
                Ok(have) if have == *text => {}
                _ => stale.push(format!("out of date: {name}")),
            }
        }
        for name in &existing {
            if !pages.contains_key(name) {
                stale.push(format!("not generated: {name}"));
            }
        }
        return if stale.is_empty() { Ok(pages.len()) } else { Err(stale) };
    }
    std::fs::create_dir_all(out).map_err(|e| vec![format!("cannot create `{}`: {e}", out.display())])?;
    for name in &existing {
        if !pages.contains_key(name) {
            let _ = std::fs::remove_file(out.join(name));
        }
    }
    for (name, text) in pages {
        std::fs::write(out.join(name), text).map_err(|e| vec![format!("cannot write `{name}`: {e}")])?;
    }
    Ok(pages.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A longer fence shows a shorter one, and the Salvo spellings become
    /// `rust` with the fence's own length [doc-example].
    #[test]
    fn fences_nest_and_salvo_is_written_rust() {
        let mut f = Fences::default();
        let lines = ["````salvo", "// ```", "// let x = 1", "// ```", "````", "```no-check", "x", "```", "```", "y", "```"];
        let out: Vec<Option<String>> = lines.iter().map(|l| f.line(l)).collect();
        assert_eq!(out[0].as_deref(), Some("````rust"));
        assert!(out[1].is_none() && out[3].is_none(), "inside a longer fence: {out:?}");
        assert_eq!(out[4].as_deref(), Some("````"));
        assert_eq!(out[5].as_deref(), Some("```rust"));
        assert_eq!(out[8].as_deref(), Some("```"), "a bare topic fence is left as written");
    }
    #[test]
    fn anchors_are_githubs() {
        assert_eq!(anchor("fn map"), "fn-map");
        assert_eq!(anchor("struct Arg.Type"), "struct-argtype");
        assert_eq!(anchor("fn to_str"), "fn-to_str");
    }

    #[test]
    fn a_home_file_orders_the_topics_it_links() {
        assert_eq!(linked_pages("| [A](Data-and-Types.md) | x |\n[B](Collections.md#lists) [C](../x/Y.md)"), vec!["Data-and-Types", "Collections"]);
    }

    #[test]
    fn a_summary_is_the_first_sentence() {
        let docs = vec!["Builds a list. Then more.".to_string()];
        assert_eq!(summary(&docs), "Builds a list.");
        let docs = vec!["Calls `a.b()` once.".to_string(), "".to_string(), "Second.".to_string()];
        assert_eq!(summary(&docs), "Calls `a.b()` once.");
    }

    #[test]
    fn references_link_or_become_code() {
        let mut targets = HashMap::new();
        targets.insert("List".to_string(), vec![Target { page: "std-core-list".into(), anchor: "type-list".into(), kind: Kind::Type }]);
        let modules = HashMap::new();
        let links = Links { targets: &targets, modules: &modules };
        assert_eq!(link_refs("a [List] of [n] and [x](y) or `[q]`", &["n".to_string()], "std-core-seq", &links), "a [`List`](std-core-list#type-list) of `n` and [x](y) or `[q]`");
        assert_eq!(link_refs("[List] here", &[], "std-core-list", &links), "[`List`](#type-list) here");
    }
}
