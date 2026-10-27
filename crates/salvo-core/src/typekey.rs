//! [type-identity] [decl-id] The identity of a type declaration, as a string.
//!
//! Every struct, effect, type alias, intrinsic or platform type and handler
//! has a **key**, the spelling of its `DeclId` that `Ty::Named` carries: std's
//! first declaration of a name is the name itself (std's text is fixed, and
//! both emitters and the hosts name std's types directly), and every other
//! declaration is `Name§module.path`, whether or not another module declares
//! the name. So a key is a function of the declaration alone: adding a file
//! that declares the same name changes no existing key. The program-wide
//! tables in `Symbols` are keyed by it (`Symbols::key_of_id` maps an id to
//! its key), so two modules may declare one name and every stage after
//! resolution still knows which one a type means. A key never reaches a user:
//! diagnostics show [plain] ([shown_text] where a message is made), and the
//! emitters render the plain name, qualified by its module where the name
//! clashes and the target needs it: clashes are decided when rendering.

use std::collections::HashSet;
use std::sync::Mutex;

/// What separates a clashing name from its module in a key. Not a character
/// an identifier can hold, so a key that leaks into generated code is a host
/// compiler error rather than a silently wrong name.
pub const SEP: char = '§';

/// The name a key was written as.
pub fn plain(key: &str) -> &str {
    key.split(SEP).next().unwrap_or(key)
}

/// The module a keyed name belongs to (dot-joined), or `None` for a plain
/// key.
pub fn module_of(key: &str) -> Option<&str> {
    key.split_once(SEP).map(|(_, m)| m)
}

/// [decl-id] Text with every key in it shown as the name it was written as:
/// how a key reads in a diagnostic. A key is `Name§module.path`, and a
/// module path is identifiers and dots, so what follows the separator up to
/// the first other character (a backtick, a space, a `<`) is dropped.
pub fn shown_text(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.contains(SEP) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != SEP {
            out.push(c);
            continue;
        }
        // The module path, and a final `.` only when an identifier follows.
        while let Some(&n) = chars.peek() {
            if n.is_alphanumeric() || n == '_' {
                chars.next();
            } else if n == '.' {
                let mut ahead = chars.clone();
                ahead.next();
                if ahead.peek().is_some_and(|a| a.is_alphanumeric() || *a == '_') {
                    chars.next();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Whether [key] names a declaration of its module (every one but std's
/// first of its name).
pub fn is_keyed(key: &str) -> bool {
    key.contains(SEP)
}

/// The key of a clashing declaration of [name] in module [module].
pub fn keyed(name: &str, module: &str) -> &'static str {
    intern(format!("{name}{SEP}{module}"))
}

/// Keys live for the process: there are few of them, and every table that
/// holds one borrows for the program's lifetime.
fn intern(s: String) -> &'static str {
    static POOL: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut pool = POOL.lock().unwrap();
    let pool = pool.get_or_insert_with(HashSet::new);
    if let Some(&found) = pool.get(s.as_str()) {
        return found;
    }
    let leaked: &'static str = Box::leak(s.into_boxed_str());
    pool.insert(leaked);
    leaked
}
