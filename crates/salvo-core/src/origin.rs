//! [origin] Where a copied node came from, and where a diagnostic about it
//! lands.
//!
//! A node an expansion copied keeps the range of the code it was copied
//! from and carries the expansion's id (`Span::expn`); the declaration's
//! `Stamp` records each expansion (`ast::ExpansionRecord`): the template copied
//! whole, an unrolled copy for one field or arm (and the copy it was itself
//! made in), or nodes the expansion made up. A diagnostic at a copied node
//! is rendered by walking that chain — the same text and position for
//! every phase that reports one (the checker, the deduction pass, the IR
//! analyses), and no position is invented on the way.

use salvo_syntax::ast::{ExpansionKind, Item, Module, Stamp};
use salvo_syntax::Span;

/// [origin] Where a node of `module`'s file is shown outside a diagnostic
/// (a trap's `module:line:col`, a "moved at line N", a `salvo facts` row,
/// an editor range): a written span is where it is; a copied one lands
/// where a diagnostic about it would ([`render`]), through the stamp of the
/// fn of the file whose expansions include it — a file's expansion ids are
/// its own, so the id finds the fn. A copied span no stamp of the file
/// records stays as it is.
pub fn landing(module: &Module, span: Span) -> Span {
    match stamp_of(module, span) {
        Some((_, stamp)) => render(stamp, span, String::new()).0,
        None => span,
    }
}

/// [origin] The stamped fn of `module`'s file a copied `span` is in — its
/// item index and its stamp — found by the span's expansion id (the file's
/// own). `None` for a written span.
pub fn stamp_of(module: &Module, span: Span) -> Option<(usize, &Stamp)> {
    if span.is_written() {
        return None;
    }
    module.items.iter().enumerate().find_map(|(i, item)| match item {
        Item::Fn(f) => f.stamped.as_ref().filter(|s| s.expansion(span.expn).is_some()).map(|s| (i, s)),
        _ => None,
    })
}

/// [origin] [comptime-instantiate] [comptime-inline] Where a diagnostic at
/// `span`, raised in the fn `stamp` is on, lands, and its message:
///
/// - a written span stays where it is;
/// - in an **expanded-in-place** fn, a node of a copy of the author's own
///   text lands on the construct it is a copy of, through every copy it
///   was copied by, prefixed with which copies they were: "when `field` is
///   `groups` (`Map<Str, ArgGroup>`): …";
/// - in a stamp, a node of a copy lands on the field or arm the copy was
///   for, prefixed with what was being stamped: "in `cmp` from `auto` for
///   `Point.y: Double`: …";
/// - anything else the expansion copied or made lands on the `by` site,
///   "in `cmp` from `auto` for `Point`: …".
pub fn render(stamp: &Stamp, span: Span, msg: String) -> (Span, String) {
    if span.is_written() {
        return (span, msg);
    }
    let prefix_at = |label: &str| format!("in `{}` from `{}` for `{label}`: {msg}", stamp.compfn, stamp.from);
    let Some(record) = stamp.expansion(span.expn) else {
        return (stamp.site, prefix_at(&stamp.at_type));
    };
    match &record.kind {
        ExpansionKind::Copy { target, label, .. } => {
            if let Some((at, copies)) = written(stamp, span) {
                return (at, format!("when {}: {msg}", copies.join(" and ")));
            }
            (*target, prefix_at(label))
        }
        ExpansionKind::Member { label } => (stamp.site, prefix_at(label)),
        ExpansionKind::Template | ExpansionKind::Made => (stamp.site, prefix_at(&stamp.at_type)),
    }
}

/// [comptime-inline] [comptime-copy-warn] The written construct a node of a
/// copy in an **expanded-in-place** fn stands for, and the copies it went
/// through, outermost first: the chain of in-place copies down to the
/// author's own text. `None` for a written span, and for a node whose chain
/// leaves the fn's own text (a stamp's copies, or what an expansion made).
pub fn written(stamp: &Stamp, span: Span) -> Option<(Span, Vec<&str>)> {
    if span.is_written() {
        return None;
    }
    let mut copies: Vec<&str> = Vec::new();
    let mut expn = span.expn;
    let mut guard = 0;
    while expn != 0 {
        let record = stamp.expansion(expn)?;
        let ExpansionKind::Copy { written: Some(label), .. } = &record.kind else {
            return None;
        };
        copies.push(label);
        expn = record.parent;
        guard += 1;
        if guard > 64 {
            return None;
        }
    }
    copies.reverse();
    Some((span.in_expansion(0), copies))
}
