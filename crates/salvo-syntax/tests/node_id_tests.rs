//! [node-id] Numbering: every span a module holds is a node, numbered once,
//! and each declaration's nodes are one run of the file's ordinals.

use salvo_syntax::ast::{Item, NodeRange};
use salvo_syntax::span::{Span, NO_NODE};
use salvo_syntax::visit_mut::{self as vm, MutVisitor};

const SRC: &str = "\
import std.core.list
struct Point canbe Mut { x: Int = 1, y: Int }
type Shape = Point | Int
effect Log { fn log(line: Str) -> None }
handler Quiet(level: Int) of Log {
    state { n: Int = level }
    fn log(line: Str) -> None { n = n + 1 }
}
qualifier Small of Int { fn qualifies(v: Int) -> Bool { return v < 10 } }
fn area(p: Point, k: Int = 2) -> Int => p { return p.x * p.y * k }
";

struct Collect(Vec<u32>);
impl MutVisitor for Collect {
    fn visit_span(&mut self, span: &mut Span) {
        self.0.push(span.node);
    }
}

#[test]
fn every_node_is_numbered_once_and_each_declaration_is_one_run() {
    let (mut module, diagnostics) = salvo_syntax::parse_module(SRC);
    assert!(diagnostics.iter().all(|d| !d.is_error()), "{diagnostics:?}");
    let ranges = module.node_ranges.clone();
    // The file's own nodes first, then the declarations in order, a member
    // after its container.
    assert_eq!(ranges[0].item, NodeRange::FILE);
    assert!(ranges.windows(2).all(|w| w[0].first + w[0].len == w[1].first), "{ranges:?}");
    let total = ranges.last().map(|r| r.first + r.len).unwrap();
    let handler = module.items.iter().position(|i| matches!(i, Item::Handler(_))).unwrap() as u32;
    assert!(ranges.iter().any(|r| r.item == handler && r.sub == 1), "the member's own run: {ranges:?}");
    // Every span a function walk reaches carries an ordinal, and no ordinal
    // is given twice.
    let mut seen = Collect(Vec::new());
    for item in &mut module.items {
        match item {
            Item::Fn(f) => vm::walk_fn(&mut seen, f),
            Item::Handler(h) => h.fns.iter_mut().for_each(|f| vm::walk_fn(&mut seen, f)),
            Item::Qualifier(q) => q.fns.iter_mut().for_each(|f| vm::walk_fn(&mut seen, f)),
            Item::Struct(s) => s.fields.iter_mut().for_each(|f| vm::walk_field_decl(&mut seen, f)),
            _ => {}
        }
    }
    assert!(!seen.0.is_empty());
    assert!(seen.0.iter().all(|n| *n != NO_NODE && *n < total), "{:?}", seen.0);
    let mut sorted = seen.0.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), seen.0.len(), "an ordinal given twice");
}

/// [node-id] A node's identity takes no part in what a span is: two copies
/// of one range compare equal, whatever their ordinals.
#[test]
fn the_ordinal_is_not_part_of_a_spans_equality() {
    let a = Span { node: 3, ..Span::new(4, 9) };
    let b = Span { node: 7, ..Span::new(4, 9) };
    assert_eq!(a, b);
    assert!(!Span::new(4, 9).is_numbered());
    assert!(!a.to(b).is_numbered(), "a covering span is no node's");
}
