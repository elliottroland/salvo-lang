//! [doc-tags] Tags in doc comments: `@param`, `@returns`, `@throws`, `@see`,
//! `@todo`, `@whynot` and `@compilerrules`, read out of a declaration's doc
//! block
//! [doc-comment] (user decisions 2026-10-20).
//!
//! A tag is a line of the block that starts with `@name`. Its text runs over
//! the lines below it until the next tag, a bare `//` line, or the end of the
//! block; the indentation the editor gives a continuation
//! [lsp-comment-continuation] is stripped. What precedes the first tag, and
//! what follows a bare line after one, is prose.
//!
//! ```text
//! // The words of a command line, split the shell's way.
//! //
//! // @param line  the command line; quotes group words and a
//! //              backslash escapes the next character
//! // @returns     the words, quotes removed
//! ```
//!
//! Only `@whynot` changes what a program does: it is the message a failed
//! predicate test answers [qual-whynot], a template whose `[name]`s are the
//! parameters of the `fn qualifies` it documents.

/// The tags this version knows. Any other `@name` at the start of a line is
/// a warning naming these.
pub const KNOWN_TAGS: &[&str] = &["param", "returns", "throws", "see", "todo", "whynot", "compilerrules"];

/// One tag of a doc block.
#[derive(Clone, Debug, PartialEq)]
pub struct DocTag {
    /// The tag's name, without the `@`: `param`.
    pub name: String,
    /// `@param`'s parameter name, the first word after the tag.
    pub target: Option<String>,
    /// The text, its lines joined by single spaces.
    pub text: String,
    /// The index of the tag's own line in the block, for diagnostics.
    pub line: usize,
}

/// A doc block split into its prose and its tags.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocParts {
    /// The prose lines, as written, tags removed.
    pub prose: Vec<String>,
    pub tags: Vec<DocTag>,
}

impl DocParts {
    /// The first tag of `name`.
    pub fn tag(&self, name: &str) -> Option<&DocTag> {
        self.tags.iter().find(|t| t.name == name)
    }
}

/// The tag a line opens, when it opens one: `@` and a name of letters, at
/// the start of the line.
fn opens_tag(line: &str) -> Option<(String, &str)> {
    let rest = line.trim_start().strip_prefix('@')?;
    let name: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    if name.is_empty() {
        return None;
    }
    let after = &rest[name.len()..];
    if !(after.is_empty() || after.starts_with(' ') || after.starts_with('\t')) {
        return None;
    }
    Some((name, after.trim()))
}

/// [doc-tags] Splits a doc block into prose and tags.
pub fn split(docs: &[String]) -> DocParts {
    let mut out = DocParts::default();
    let mut open: Option<(DocTag, Vec<String>)> = None;
    let close = |open: &mut Option<(DocTag, Vec<String>)>, out: &mut DocParts| {
        if let Some((mut tag, lines)) = open.take() {
            let text = lines.iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
            if tag.name == "param" {
                let mut words = text.splitn(2, char::is_whitespace);
                tag.target = words.next().filter(|w| !w.is_empty()).map(str::to_string);
                tag.text = words.next().unwrap_or("").trim().to_string();
            } else {
                tag.text = text;
            }
            out.tags.push(tag);
        }
    };
    for (i, line) in docs.iter().enumerate() {
        if let Some((name, first)) = opens_tag(line) {
            close(&mut open, &mut out);
            open = Some((DocTag { name, target: None, text: String::new(), line: i }, vec![first.to_string()]));
            continue;
        }
        if line.trim().is_empty() {
            close(&mut open, &mut out);
            out.prose.push(line.clone());
            continue;
        }
        match &mut open {
            Some((_, lines)) => lines.push(line.clone()),
            None => out.prose.push(line.clone()),
        }
    }
    close(&mut open, &mut out);
    while out.prose.last().is_some_and(|l| l.trim().is_empty()) {
        out.prose.pop();
    }
    out
}

/// [doc-compilerrules] An `@compilerrules` tag's text, `[rule-a, rule-b]
/// how they apply here`: the rule labels, and the comment after them (empty
/// when there is none). `None` when the text does not open with a bracketed,
/// non-empty list of labels (lowercase words joined by `-`).
pub fn compiler_rules(text: &str) -> Option<(Vec<String>, String)> {
    let rest = text.trim_start().strip_prefix('[')?;
    let close = rest.find(']')?;
    let rules: Vec<String> = rest[..close].split(',').map(|r| r.trim().to_string()).collect();
    let well_formed = |r: &String| {
        !r.is_empty()
            && r.split('-').all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
    };
    if !rules.iter().all(well_formed) {
        return None;
    }
    Some((rules, rest[close + 1..].trim().to_string()))
}

/// One piece of a `@whynot` template: text, or a `[name]` reference.
#[derive(Clone, Debug, PartialEq)]
pub enum TemplatePart {
    Text(String),
    Param(String),
}

/// [qual-whynot] A `@whynot` template's pieces: `[name]` is a reference,
/// `[[` a literal `[`, and everything else text. A `[` with no `]` after it
/// is text.
pub fn template(text: &str) -> Vec<TemplatePart> {
    let mut out: Vec<TemplatePart> = Vec::new();
    let mut buf = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        buf.push_str(&rest[..open]);
        rest = &rest[open..];
        if rest.starts_with("[[") {
            buf.push('[');
            rest = &rest[2..];
            continue;
        }
        let Some(close) = rest.find(']') else { break };
        let name = &rest[1..close];
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            buf.push_str(&rest[..=close]);
            rest = &rest[close + 1..];
            continue;
        }
        if !buf.is_empty() {
            out.push(TemplatePart::Text(std::mem::take(&mut buf)));
        }
        out.push(TemplatePart::Param(name.to_string()));
        rest = &rest[close + 1..];
    }
    buf.push_str(rest);
    if !buf.is_empty() {
        out.push(TemplatePart::Text(buf));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &[&str]) -> Vec<String> {
        s.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn tags_follow_the_prose_and_run_over_lines() {
        let d = split(&lines(&[
            "The words of a line.",
            "",
            "@param line  the command line; quotes group words and a",
            "             backslash escapes the next character",
            "@returns     the words",
            "@see [lex]",
        ]));
        assert_eq!(d.prose, lines(&["The words of a line."]));
        assert_eq!(d.tags.len(), 3);
        assert_eq!(d.tags[0].target.as_deref(), Some("line"));
        assert_eq!(d.tags[0].text, "the command line; quotes group words and a backslash escapes the next character");
        assert_eq!(d.tags[1].text, "the words");
        assert_eq!(d.tags[2].text, "[lex]");
    }

    #[test]
    fn a_bare_line_ends_a_tag_and_returns_to_prose() {
        let d = split(&lines(&["@todo cover the empty case", "", "More prose."]));
        assert_eq!(d.tags[0].text, "cover the empty case");
        assert_eq!(d.prose, lines(&["", "More prose."]));
    }

    #[test]
    fn an_at_inside_a_line_is_prose() {
        let d = split(&lines(&["Mail rolae@example.com, or @ someone."]));
        assert!(d.tags.is_empty());
    }

    #[test]
    fn a_template_has_text_and_references() {
        assert_eq!(
            template("[n] is not in [lo]..[hi]"),
            vec![
                TemplatePart::Param("n".into()),
                TemplatePart::Text(" is not in ".into()),
                TemplatePart::Param("lo".into()),
                TemplatePart::Text("..".into()),
                TemplatePart::Param("hi".into()),
            ]
        );
        assert_eq!(template("a [[b] c"), vec![TemplatePart::Text("a [b] c".into())]);
    }

    /// [doc-compilerrules] The rules, and the comment after them; a list that
    /// is missing, empty or holds a word that is not a label is no list.
    #[test]
    fn compiler_rules_are_a_bracketed_list_then_a_comment() {
        assert_eq!(compiler_rules("[op-arith, kt-pow] lowered inline"), Some((vec!["op-arith".into(), "kt-pow".into()], "lowered inline".into())));
        assert_eq!(compiler_rules("[op-arith]"), Some((vec!["op-arith".into()], String::new())));
        for bad in ["op-arith", "[]", "[op-arith,]", "[Op-Arith]", "[op arith]", "[op-arith"] {
            assert_eq!(compiler_rules(bad), None, "{bad}");
        }
    }

}
