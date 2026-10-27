//! Byte-offset source spans.

use std::fmt;

/// A half-open byte range into a source file, and the expansion that copied
/// the node there, if one did [origin].
///
/// A node as written has `expn` 0, and its range is in its own file. A node
/// an expansion copied (a comptime stamp's body, an unrolled `[for]` copy)
/// keeps the range of the code it was copied from — the template's, which
/// may be another file's — and carries the expansion's id, which the
/// declaration's `Stamp` records (`ast::Expansion`): what it is a copy of,
/// and where a diagnostic about it lands. Nothing invents a position. Two
/// copies of one template are told apart by their ids, so a table keyed by
/// span (`(file, Span)`) never sees them as one node; positions compare
/// meaningfully only between spans of one expansion (`order_key`), and only
/// a written span covers an offset of its file (`holds`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    /// [origin] The expansion this node was copied by: an id among its
    /// file's expansions, 0 for written code.
    pub expn: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Span { start, end, expn: 0 }
    }

    /// The smallest span covering both `self` and `other`, in `self`'s
    /// expansion.
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            expn: self.expn,
        }
    }

    /// Another range of the code `self` is in: the same expansion.
    pub fn at(self, start: u32, end: u32) -> Span {
        Span { start, end, expn: self.expn }
    }

    /// [origin] The same range, copied by expansion `expn` (0: as written).
    pub fn in_expansion(self, expn: u32) -> Span {
        Span { expn, ..self }
    }

    /// [origin] Whether this is a node as written in its own file.
    pub fn is_written(self) -> bool {
        self.expn == 0
    }

    /// [origin] Whether an expansion copied this node ([`Span::is_written`]'s
    /// negation).
    pub fn is_expanded(self) -> bool {
        self.expn != 0
    }

    /// [origin] The order of spans in one file: the written ones by
    /// position, then each expansion's in the order the expansions were
    /// made, by position within it.
    pub fn order_key(self) -> (u32, u32, u32) {
        (self.expn, self.start, self.end)
    }

    /// [origin] Whether this written span covers `offset` of its file
    /// (half-open). A copied node is never at an offset of the file it was
    /// copied into: its range is the template's.
    pub fn holds(self, offset: u32) -> bool {
        self.is_written() && self.start <= offset && offset < self.end
    }

    /// [origin] [`Span::holds`], with the end included.
    pub fn reaches(self, offset: u32) -> bool {
        self.is_written() && self.start <= offset && offset <= self.end
    }

    pub fn len(self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// [origin] Spans order as [`Span::order_key`] does.
impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Span {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order_key().cmp(&other.order_key())
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.expn == 0 {
            write!(f, "{}..{}", self.start, self.end)
        } else {
            write!(f, "{}..{}#{}", self.start, self.end, self.expn)
        }
    }
}

/// [origin] [`line_col`] of a span's start in `source`, the file the node is
/// in. A copied node's range is its template's, not a position of that
/// file, so it is placed at the end of the file — where a copy's synthetic
/// span past the end of the file landed before origins (ROADMAP 0ac: such a
/// location should name the origin's site).
pub fn span_line_col(source: &str, span: Span) -> (u32, u32) {
    if span.is_expanded() {
        return line_col(source, source.len() as u32);
    }
    line_col(source, span.start)
}

/// Computes 1-based line and column numbers for a byte offset.
pub fn line_col(source: &str, offset: u32) -> (u32, u32) {
    let offset = (offset as usize).min(source.len());
    let mut line = 1;
    let mut col = 1;
    for (i, c) in source.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
