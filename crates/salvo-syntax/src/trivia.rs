//! [fmt-comments] [fmt-blank-lines] What a file holds besides its tree: its
//! comments that are no declaration's docs, and where its blank lines are.
//!
//! The parser keeps them beside the module (`Module::trivia`), not in the
//! tree: no phase after the parse reads them, a tree compared with another
//! never sees them, and only the printer writes them back, placing each
//! comment by its offset among the nodes it prints [ast-print].

/// One `//` comment of a file that is no declaration's docs [doc-comment]
/// and not the module's [doc-module].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriviaComment {
    /// The offset of its `//`.
    pub start: u32,
    /// The offset after its last character (the end of its line).
    pub end: u32,
    /// The comment as written, from `//` to the end of the line.
    pub text: String,
    /// Whether only whitespace precedes it on its line; an end-of-line
    /// comment follows code.
    pub own_line: bool,
}

/// [fmt-comments] [fmt-blank-lines] A file's comments and blank lines.
#[derive(Clone, Debug, Default)]
pub struct Trivia {
    /// The comments that are no docs, in source order.
    pub comments: Vec<TriviaComment>,
    /// The comments that are docs, in source order: the tree holds their
    /// text; these are how they were written, which the printer writes.
    pub doc_comments: Vec<TriviaComment>,
    /// The 0-based numbers of the lines holding nothing but whitespace,
    /// ascending.
    pub blank_lines: Vec<u32>,
    /// The offset each line starts at.
    pub line_starts: Vec<u32>,
    /// [fmt-comments] The module was lowered [syntax-lowered]: it prints as
    /// source with no comments (its printing is no file's, and what the
    /// printer writes as comments of its own would read back as the
    /// author's), and as a dump with them.
    pub lowered: bool,
    /// Every closing bracket (`)`, `]`, `}`, `>`) by offset, ascending: where
    /// a list or a body the tree gives no span of its own ends.
    pub closers: Vec<(u32, char)>,
}

/// Trivia is layout, not tree: two modules compare equal whatever comments
/// and blank lines they were written with.
impl PartialEq for Trivia {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Trivia {
    /// Whether the file had any trivia recorded (a parse records it; a
    /// module a pass built has none).
    pub fn is_empty(&self) -> bool {
        self.line_starts.is_empty()
    }

    /// [fmt-comments] Forgets the comments written inside `span`: a pass
    /// that removes the declaration there removes them with it, so a dump
    /// does not print them where it stood.
    pub fn forget_comments_in(&mut self, span: crate::span::Span) {
        if span.is_written() {
            self.comments.retain(|c| !(span.start <= c.start && c.start < span.end));
        }
    }

    /// The 0-based line `offset` is on.
    pub fn line_of(&self, offset: u32) -> u32 {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line as u32,
            Err(next) => next.saturating_sub(1) as u32,
        }
    }

    /// [fmt-blank-lines] Whether a blank line lies between the line `from`
    /// ends on and the line `to` starts on.
    pub fn blank_between(&self, from: u32, to: u32) -> bool {
        let (a, b) = (self.line_of(from), self.line_of(to));
        if b <= a + 1 {
            return false;
        }
        let i = self.blank_lines.partition_point(|&l| l <= a);
        self.blank_lines.get(i).is_some_and(|&l| l < b)
    }

    /// The end of the last line before the one `offset` is on that holds
    /// code: not blank, and not an own-line comment's.
    pub fn code_end_before(&self, offset: u32) -> u32 {
        let mut line = self.line_of(offset);
        while line > 0 {
            line -= 1;
            let start = self.line_starts[line as usize];
            let end = self.line_starts.get(line as usize + 1).map_or(u32::MAX, |&s| s.saturating_sub(1));
            let i = self.comments.partition_point(|c| c.start < start);
            let comment_line = self.comments.get(i).is_some_and(|c| c.own_line && c.start < end);
            if !comment_line && self.blank_lines.binary_search(&line).is_err() {
                return end.min(offset);
            }
        }
        offset
    }

    /// [doc-comment] The doc comments `docs` holds the text of, as written:
    /// the run on the lines directly above the one `offset` is on (the
    /// module's: the file's first run, for `offset` 0 with `module`).
    /// `None` when they are not there as `docs` reads them.
    pub fn docs_as_written(&self, docs: &[String], offset: u32, module: bool) -> Option<Vec<&str>> {
        let run: &[TriviaComment] = if module {
            self.doc_comments.get(..docs.len())?
        } else {
            let line = self.line_of(offset);
            let first = line.checked_sub(docs.len() as u32)?;
            let i = self.doc_comments.partition_point(|c| self.line_of(c.start) < first);
            let run = self.doc_comments.get(i..i + docs.len())?;
            if run.iter().enumerate().any(|(k, c)| self.line_of(c.start) != first + k as u32) {
                return None;
            }
            run
        };
        let same = run.iter().zip(docs).all(|(c, d)| {
            let raw = &c.text[2..];
            raw.strip_prefix(' ').unwrap_or(raw) == d
        });
        same.then(|| run.iter().map(|c| c.text.as_str()).collect())
    }

    /// The offset of the first `close` bracket at or after `offset`.
    pub fn close_after(&self, offset: u32, close: char) -> Option<u32> {
        let i = self.closers.partition_point(|&(o, _)| o < offset);
        self.closers[i..].iter().find(|&&(_, c)| c == close).map(|&(o, _)| o)
    }

    /// The trivia of `source`: `comments` (the lexer's) less those on the
    /// lines `doc_lines` holds.
    pub fn new(source: &str, comments: Vec<crate::lexer::Comment>, line_starts: Vec<u32>, doc_lines: &std::collections::HashSet<u32>, closers: Vec<(u32, char)>) -> Trivia {
        let line_of = |offset: u32| match line_starts.binary_search(&offset) {
            Ok(line) => line as u32,
            Err(next) => next.saturating_sub(1) as u32,
        };
        let (docs, comments): (Vec<_>, Vec<_>) = comments
            .into_iter()
            .map(|c| {
                let doc = c.own_line && doc_lines.contains(&line_of(c.span.start));
                let comment = TriviaComment {
                    start: c.span.start,
                    end: c.span.end,
                    text: source[c.span.start as usize..c.span.end as usize].to_string(),
                    own_line: c.own_line,
                };
                (doc, comment)
            })
            .partition(|(doc, _)| *doc);
        let comments = comments.into_iter().map(|(_, c)| c).collect();
        let doc_comments = docs.into_iter().map(|(_, c)| c).collect();
        let blank_lines = source
            .split('\n')
            .enumerate()
            .filter(|(_, l)| l.trim().is_empty())
            .map(|(i, _)| i as u32)
            .collect();
        Trivia { comments, doc_comments, blank_lines, line_starts, closers, lowered: false }
    }
}
