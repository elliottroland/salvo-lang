//! [type-identity] The identity of a type declaration, as a string.
//!
//! Every struct, effect, type alias, intrinsic or platform type and handler
//! has a **key**: its name, when no other module declares that name, and
//! `Name§module.path` for every declaration of a clashing name but the first
//! (std's come first, so std keeps its plain names, which both emitters and
//! the hosts rely on). A `Ty::Named` carries the key of the declaration it
//! resolved to, and the program-wide tables in `Symbols` are keyed by it, so
//! two modules may declare one name and every stage after resolution still
//! knows which one a type means. A key never reaches a user: diagnostics show
//! [plain], and the emitters render the plain name, qualified by its module
//! where the target needs it.

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

/// Whether [key] names a clashing declaration other than the first.
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
