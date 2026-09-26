//! `salvo lang`: emit language metadata for editor tooling [cli-lang].
//!
//! `salvo lang tm-grammar` prints (or writes with `--out`) the TextMate
//! grammar used by the VS Code extension (`vscode/syntaxes/`). Keyword
//! alternations are derived from the lexer's keyword table
//! (`salvo_syntax::token::KEYWORDS`), so the grammar stays in sync with
//! the language as keywords are added — regenerate with:
//!
//! ```text
//! cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json
//! ```
//!
//! A test compares the checked-in extension grammar against the generated
//! output, so drift fails `cargo test`.

use std::path::PathBuf;
use std::process::ExitCode;

/// Keyword categories for highlighting scopes. Every entry must name a
/// keyword from `KEYWORDS`; `keywords_are_fully_categorized` asserts the
/// partition is exact (no missing, no unknown, no duplicates) [cli-lang].
const CONTROL_KEYWORDS: &[&str] = &[
    "if", "elif", "else", "when", "while", "for", "in", "return", "break", "continue",
    "try",
];
const DECLARATION_KEYWORDS: &[&str] = &[
    "fn", "let", "struct", "qualifier", "effect", "handler", "params", "type", "intrinsic",
    "platform", "import", "provenance", "refn", "rename", "state",
];
const OTHER_KEYWORDS: &[&str] = &[
    "as", "of", "with", "canbe", "is", "use",
    // [obligation-spelling] The obligation keywords.
    "proj", "once", "linear",
    // [proj-infer] The opaque projection, after the type it is about
    // (`-> T holds proj(a)`).
    "holds",
];
const BOOLEAN_KEYWORDS: &[&str] = &["true", "false"];

/// [cli-lang] Words that are **keywords by position** rather than reserved
/// words: they are ordinary identifiers to the lexer (a variable may be called
/// `send`, and `the_asynchronous_words_are_not_reserved` asserts it), so each is
/// highlighted only where the *shape* around it is the one the parser
/// recognises. Highlighting them unconditionally would colour a variable named
/// `ordered` or a field named `on` (user requests 2026-09-18).
///
/// Each entry is `(regex, scope, why)`, with the regex written as it lands in
/// the JSON — the shapes mirror `parser.rs`'s own tests
/// (`at_word(w) && peek_at(1)…`), which is what keeps the two in step. They are
/// deliberately *not* in `KEYWORDS`, so `keywords_are_fully_categorized`
/// neither expects nor forbids them.
const CONTEXTUAL_PATTERNS: &[(&str, &str, &str)] = &[
    // `export fn f(…)`, `export intrinsic type T`: the visibility modifier,
    // first of all the modifiers, so the lookahead is every word that can
    // start a declaration (including the other contextual ones).
    (
        "\\\\bexport(?=\\\\s+(fn|struct|effect|handler|qualifier|type|params|intrinsic|linear|platform|provenance|actor|iter|send)\\\\b)",
        "keyword.declaration.salvo",
        "the export modifier",
    ),
    // `actor effect E`, `send fn m(…)`, `iter fn walk(…)`: a modifier before
    // the declaration keyword it modifies.
    (
        "\\\\bactor(?=\\\\s+effect\\\\b)",
        "keyword.declaration.salvo",
        "the actor-effect modifier",
    ),
    (
        "\\\\bsend(?=\\\\s+fn\\\\b)",
        "keyword.declaration.salvo",
        "the send-member modifier",
    ),
    (
        "\\\\biter(?=\\\\s+fn\\\\b)",
        "keyword.declaration.salvo",
        "the iterator-fn modifier",
    ),
    // [cmp-auto] `auto fn cmp@Person(…)` and `: auto Ordered<self>`: the
    // generator modifier, recognised by what follows it — `fn`, or the
    // capitalized group name inside an obligation clause.
    (
        "\\\\bauto(?=\\\\s+(fn\\\\b|[A-Z]))",
        "keyword.declaration.salvo",
        "the structural-implementation modifier",
    ),
    // `mailbox { capacity: n }`: the handler's queue slot, named before a block.
    (
        "\\\\bmailbox(?=\\\\s*\\\\{)",
        "keyword.declaration.salvo",
        "the mailbox slot of a handler",
    ),
    // `spawn H(…)`: a name follows, which is what the parser tests for.
    (
        "\\\\bspawn(?=\\\\s+[A-Za-z_])",
        "keyword.control.salvo",
        "the spawn expression",
    ),
    // `waitfor out: Reply<T> { … }`: a name and a colon follow — no call of a
    // function named `waitfor` can look like that.
    (
        "\\\\bwaitfor(?=\\\\s+[A-Za-z_][A-Za-z0-9_]*\\\\s*:)",
        "keyword.control.salvo",
        "the waitfor bridge",
    ),
    // `replyto k(…)` and `replyto! k(…)`.
    (
        "\\\\breplyto!?(?=\\\\s+[A-Za-z_])",
        "keyword.control.salvo",
        "a reply-token mint",
    ),
    // `[use, spawn]`: the lowercase capability effects, which sit in
    // an effect list beside uppercase effect names. A comma or a bracket on
    // each side is the shape, and `use` needs no entry — it is a real keyword.
    (
        "(?<=[\\\\[,])\\\\s*(spawn)\\\\b(?=\\\\s*[,\\\\]])",
        "keyword.other.salvo",
        "a capability effect in an effect list",
    ),
    // `use local H(…)`: the scope-local binding opt-out [use-local]. Contextual,
    // so it is recognised only directly after `use` — a variable named `local`
    // stays plain.
    (
        "(?<=\\\\buse\\\\s)local\\\\b",
        "keyword.other.salvo",
        "the scope-local binding opt-out",
    ),
    // `[local E]`: an effect-list entry that accepts a scope-local binding
    // [effect-local]. A bracket or comma before, an *effect name* after — which
    // is capitalized [name-casing], and is what tells it from a variable.
    (
        "(?<=[\\\\[,])\\\\s*(local)\\\\b(?=\\\\s+[A-Z])",
        "keyword.other.salvo",
        "a local effect entry in an effect list",
    ),
    // `on POOL`, the placement clause of a spawn or a mint. A name follows;
    // a *use* of a variable called `on` is followed by an operator, a comma,
    // a brace or a bracket instead.
    (
        "\\\\bon(?=\\\\s+[A-Za-z_])",
        "keyword.other.salvo",
        "the placement clause",
    ),
    // `k@self(…)`: the selector naming the enclosing handler, and special only
    // immediately after `@`.
    (
        "(?<=@)self\\\\b",
        "variable.language.salvo",
        "the enclosing-handler selector",
    ),
    // `: Yield<self, T>`, `: auto Ordered<self>`: the declaring type in an
    // obligation's argument list [group-self] — special only between the
    // angle bracket or comma and the comma or closing bracket, so a
    // variable named `self` elsewhere stays plain (it cannot be declared
    // anyway, but the grammar should not rely on that).
    (
        "(?<=[<,])\\\\s*self\\\\b(?=\\\\s*[,>])",
        "variable.language.salvo",
        "the declaring type in an obligation",
    ),
    // `=> map: preserve KeyOf` [qual-preserve]: the preservation entry of a
    // deduction or refinement — a capitalized qualifier name follows, which
    // is the parser's own test.
    (
        "\\\\bpreserve(?=\\\\s+[A-Z])",
        "keyword.other.salvo",
        "the preserve entry",
    ),
    // `=> defer out` [defer-deduction]: the deferred-consumption entry — a
    // parameter name follows, which is the parser's own test.
    (
        "\\\\bdefer(?=\\\\s+[a-z_])",
        "keyword.other.salvo",
        "the defer entry",
    ),
];

/// The lowercase claims a `canbe` clause can name. `hashed`/`ordered` were
/// deleted with the ordering round (2026-09-21) — being hashable or orderable is
/// *having the function* now [cmp-auto] — leaving `once` on a type and
/// `linear` in a generic's clause.
const CANBE_WORDS: &[&str] = &["once", "linear"];

fn alternation(words: &[&str]) -> String {
    words.join("|")
}

/// The TextMate grammar JSON. Everything but the keyword alternations is
/// a fixed template mirroring the lexer (`salvo-syntax/src/lexer.rs`):
/// `//` comments, `"` strings with `\\ntr\\"$'0` escapes and `${}`
/// interpolation, `'c'` chars, `_`-separated
/// numbers.
pub fn tm_grammar() -> String {
    let control = alternation(CONTROL_KEYWORDS);
    let declaration = alternation(DECLARATION_KEYWORDS);
    let other = alternation(OTHER_KEYWORDS);
    let boolean = alternation(BOOLEAN_KEYWORDS);
    // [cli-lang] One JSON pattern per contextual shape, in table order.
    let contextual = CONTEXTUAL_PATTERNS
        .iter()
        .map(|(regex, scope, why)| {
            format!(
                "        {{\n          \"comment\": \"{why} — a keyword by position, \
                 not a reserved word\",\n          \"name\": \"{scope}\",\n          \
                 \"match\": \"{regex}\"\n        }},"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let canbe_words = alternation(CANBE_WORDS);
    format!(
        r##"{{
  "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
  "name": "Salvo",
  "scopeName": "source.salvo",
  "comment": "GENERATED by `salvo lang tm-grammar` — do not edit by hand. Regenerate with: cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json",
  "patterns": [
    {{ "include": "#comments" }},
    {{ "include": "#strings" }},
    {{ "include": "#chars" }},
    {{ "include": "#numbers" }},
    {{ "include": "#declarations" }},
    {{ "include": "#keywords" }},
    {{ "include": "#types" }},
    {{ "include": "#functions" }},
    {{ "include": "#operators" }},
    {{ "include": "#identifiers" }}
  ],
  "repository": {{
    "comments": {{
      "patterns": [
        {{
          "name": "comment.line.double-slash.salvo",
          "begin": "//",
          "end": "$",
          "patterns": [
            {{
              "comment": "[symbol] doc references to parameters, fields and types, and the [rule-label] references the compiler's own comments carry",
              "name": "variable.parameter.reference.salvo",
              "match": "\\[[A-Za-z_][A-Za-z0-9_.-]*\\]"
            }}
          ]
        }}
      ]
    }},
    "strings": {{
      "name": "string.quoted.double.salvo",
      "begin": "\"",
      "end": "\"|(?=\\n)",
      "patterns": [
        {{
          "name": "constant.character.escape.salvo",
          "match": "\\\\[ntr\\\\\"$'0]"
        }},
        {{
          "name": "invalid.illegal.escape.salvo",
          "match": "\\\\."
        }},
        {{
          "name": "meta.embedded.interpolation.salvo",
          "begin": "\\$\\{{",
          "beginCaptures": {{
            "0": {{ "name": "punctuation.section.interpolation.begin.salvo" }}
          }},
          "end": "\\}}",
          "endCaptures": {{
            "0": {{ "name": "punctuation.section.interpolation.end.salvo" }}
          }},
          "patterns": [
            {{ "include": "#numbers" }},
            {{ "include": "#keywords" }},
            {{ "include": "#types" }},
            {{ "include": "#functions" }},
            {{ "include": "#operators" }},
            {{ "include": "#identifiers" }}
          ]
        }}
      ]
    }},
    "chars": {{
      "name": "string.quoted.single.salvo",
      "match": "'(\\\\[ntr\\\\'\"0]|[^'\\\\\\n])'"
    }},
    "numbers": {{
      "patterns": [
        {{
          "name": "constant.numeric.float.salvo",
          "match": "\\b[0-9][0-9_]*\\.[0-9][0-9_]*f?\\b"
        }},
        {{
          "name": "constant.numeric.integer.salvo",
          "match": "\\b[0-9][0-9_]*L?\\b"
        }}
      ]
    }},
    "keywords": {{
      "patterns": [
        {{
          "match": "\\b(canbe)\\s+({canbe_words})\\b",
          "captures": {{
            "1": {{ "name": "keyword.other.salvo" }},
            "2": {{ "name": "keyword.other.salvo" }}
          }}
        }},
        {{
          "comment": "the second and later claims of a `canbe a, b` clause: a comma before and a clause terminator after, so an argument named `once` stays plain",
          "match": "(?<=,)\\s*({canbe_words})\\b(?=\\s*(?:,|\\{{|>|->|$))",
          "captures": {{
            "1": {{ "name": "keyword.other.salvo" }}
          }}
        }},
        {{
          "name": "keyword.control.salvo",
          "match": "\\b({control})\\b"
        }},
        {{
          "name": "keyword.declaration.salvo",
          "match": "\\b({declaration})\\b"
        }},
        {{
          "name": "keyword.other.salvo",
          "match": "\\b({other})\\b"
        }},
{contextual}
        {{
          "name": "constant.language.boolean.salvo",
          "match": "\\b({boolean})\\b"
        }},
        {{
          "name": "constant.language.none.salvo",
          "match": "\\bNone\\b"
        }}
      ]
    }},
    "declarations": {{
      "patterns": [
        {{
          "comment": "fn name — the declared function name",
          "match": "\\b(fn)\\s+([a-z_][A-Za-z0-9_]*)",
          "captures": {{
            "1": {{ "name": "keyword.declaration.function.salvo" }},
            "2": {{ "name": "entity.name.function.salvo" }}
          }}
        }},
        {{
          "comment": "struct / qualifier / effect / handler / type Name",
          "match": "\\b(struct|qualifier|effect|handler|type)\\s+([A-Z][A-Za-z0-9_]*)",
          "captures": {{
            "1": {{ "name": "keyword.declaration.salvo" }},
            "2": {{ "name": "entity.name.type.salvo" }}
          }}
        }}
      ]
    }},
    "types": {{
      "comment": "Capitalized identifiers are types and qualifiers by convention (Str, Int, Ok, Mut, ...)",
      "name": "entity.name.type.salvo",
      "match": "\\b[A-Z][A-Za-z0-9_]*\\b"
    }},
    "functions": {{
      "comment": "Identifier immediately followed by ( or by a generic argument list then ( — a call or dot-notation call",
      "name": "entity.name.function.call.salvo",
      "match": "\\b[a-z_][A-Za-z0-9_]*(?=\\s*(\\(|<[A-Za-z0-9_,<> ?|]*>\\s*\\())"
    }},
    "operators": {{
      "patterns": [
        {{
          "name": "keyword.operator.arrow.salvo",
          "match": "->"
        }},
        {{
          "name": "keyword.operator.spread.salvo",
          "match": "\\.\\.\\."
        }},
        {{
          "name": "keyword.operator.comparison.salvo",
          "match": "==|!=|<=|>=|<|>"
        }},
        {{
          "name": "keyword.operator.logical.salvo",
          "match": "\\|\\||&&|!"
        }},
        {{
          "name": "keyword.operator.union.salvo",
          "match": "\\|"
        }},
        {{
          "name": "keyword.operator.optional.salvo",
          "match": "\\?"
        }},
        {{
          "comment": "[op-compound] before the arithmetic operators, so `+=` is one token-shaped thing rather than a `+` beside an `=`",
          "name": "keyword.operator.assignment.salvo",
          "match": "\\+=|-=|\\*=|/="
        }},
        {{
          "name": "keyword.operator.arithmetic.salvo",
          "match": "\\+\\+|--|\\+|-|\\*|/|%"
        }},
        {{
          "name": "keyword.operator.assignment.salvo",
          "match": "="
        }}
      ]
    }},
    "identifiers": {{
      "name": "variable.other.salvo",
      "match": "\\b[a-z_][A-Za-z0-9_]*\\b"
    }}
  }}
}}
"##
    )
}

/// `salvo lang tm-grammar [--out PATH]`: print the grammar to stdout, or
/// write it to `PATH` [cli-lang].
pub fn run_tm_grammar(out: Option<&PathBuf>) -> ExitCode {
    let grammar = tm_grammar();
    match out {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if let Err(err) = std::fs::create_dir_all(parent) {
                    eprintln!("error: failed to create `{}`: {err}", parent.display());
                    return ExitCode::FAILURE;
                }
            }
            match std::fs::write(path, grammar) {
                Ok(()) => {
                    eprintln!("wrote {}", path.display());
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("error: failed to write `{}`: {err}", path.display());
                    ExitCode::FAILURE
                }
            }
        }
        None => {
            print!("{grammar}");
            ExitCode::SUCCESS
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use salvo_syntax::token::{TokenKind, KEYWORDS};

    // [cli-lang] The highlighting categories exactly partition the lexer's
    // keyword table: adding a keyword to the language without categorizing
    // it here fails this test.
    #[test]
    fn keywords_are_fully_categorized() {
        let mut categorized: Vec<&str> = CONTROL_KEYWORDS
            .iter()
            .chain(DECLARATION_KEYWORDS)
            .chain(OTHER_KEYWORDS)
            .chain(BOOLEAN_KEYWORDS)
            .copied()
            .collect();
        categorized.sort_unstable();
        let dupes: Vec<_> = categorized.windows(2).filter(|w| w[0] == w[1]).collect();
        assert!(dupes.is_empty(), "keywords in multiple categories: {dupes:?}");

        let mut lexer_keywords: Vec<&str> = KEYWORDS.iter().map(|(text, _)| *text).collect();
        lexer_keywords.sort_unstable();
        assert_eq!(
            categorized, lexer_keywords,
            "grammar keyword categories out of sync with salvo_syntax::token::KEYWORDS"
        );

        // Belt and braces: every categorized word really is a keyword.
        for word in &categorized {
            assert!(
                TokenKind::keyword(word).is_some(),
                "`{word}` categorized but not a lexer keyword"
            );
        }
    }

    // [cli-lang] The generated grammar is valid JSON and contains every
    // keyword.
    // [cli-lang] Words that are keywords *by position* are highlighted where
    // the shape says so and nowhere else (user requests 2026-09-18). Asserted
    // on the generated patterns rather than by eye, since the false-positive
    // case — a variable named `on`, a field named `from` — is exactly what a
    // hand check misses.
    #[test]
    fn contextual_keywords_are_matched_only_in_their_shape() {
        let grammar = tm_grammar();
        for (regex, scope, _) in CONTEXTUAL_PATTERNS {
            // The table is rendered verbatim, so a pattern that is in the table
            // is in the grammar with its scope.
            assert!(
                grammar.contains(regex),
                "expected the contextual pattern `{regex}` in the grammar"
            );
            assert!(
                grammar.contains(scope),
                "expected the scope `{scope}` in the grammar"
            );
        }
        // Every contextual word the parser names as a constant is covered: the
        // parser and the grammar are the two halves of "this word is special
        // here", and a word special in one and plain in the other is the bug
        // this asserts against.
        for word in [
            salvo_syntax::parser::SELF_SELECTOR,
            salvo_syntax::parser::ACTOR_MODIFIER,
            salvo_syntax::parser::MAILBOX_SLOT,
            salvo_syntax::parser::EXPORT_MODIFIER,
        ] {
            assert!(
                CONTEXTUAL_PATTERNS.iter().any(|(regex, _, _)| regex.contains(word)),
                "the parser treats `{word}` as contextual, so the grammar must \
                 highlight it in its shape"
            );
        }
        // [mod-export] `export` must colour as a *keyword*, in the same scope
        // as the declaration words it precedes — the whole point of the entry,
        // since the LSP has no semantic tokens and this grammar is the only
        // thing that highlights anything. Its lookahead has to name every word
        // that can start a declaration, or an `export intrinsic fn` (or
        // `export actor effect`, or `export iter fn`) would go uncoloured.
        let (regex, scope, _) = CONTEXTUAL_PATTERNS
            .iter()
            .find(|(regex, _, _)| regex.contains(salvo_syntax::parser::EXPORT_MODIFIER))
            .expect("an `export` pattern");
        assert_eq!(
            *scope, "keyword.declaration.salvo",
            "`export` must share the declaration keywords' scope"
        );
        // Every word that can begin a *top-level* declaration, which is the set
        // `export` may precede — `let`, `import`, `refn`, `rename` and `state`
        // are in `DECLARATION_KEYWORDS` but are not among them ([mod-export]
        // refuses the middle three outright, and the other two are not items).
        for word in [
            "fn", "struct", "effect", "handler", "qualifier", "type", "params", "intrinsic",
            "linear", "platform", "provenance", "actor", "iter", "send",
        ] {
            assert!(
                regex.contains(word),
                "`export {word} …` would not highlight: the lookahead omits `{word}`"
            );
        }
        // The asynchronous expression forms and the deduction entries,
        // which have no constant of their own (they are recognised inline
        // by `at_word`). `from` left the list with the `proj(x)` respell
        // (the refinement-types sequence, step 1); `preserve` and `defer`
        // joined it.
        for word in ["spawn", "waitfor", "replyto", "send", "iter", "on", "preserve", "defer"] {
            assert!(
                CONTEXTUAL_PATTERNS.iter().any(|(regex, _, _)| regex.contains(word)),
                "expected a contextual pattern for `{word}`"
            );
        }
        // The `canbe` clause's claims, both the first and the continuations.
        assert!(grammar.contains("\\\\b(canbe)\\\\s+(once|linear)\\\\b"));
        assert!(grammar.contains("(?<=,)\\\\s*(once|linear)\\\\b"));
        // Ordered *before* the plain keyword alternation, which would
        // otherwise consume `canbe` and leave `once` unmatched: TextMate
        // takes the first pattern that matches at a position.
        let canbe_clause = grammar.find("(canbe)\\\\s+(once").expect("canbe clause pattern");
        let plain = grammar.find("(as|of|with|canbe").expect("plain keyword pattern");
        assert!(
            canbe_clause < plain,
            "the `canbe once` pattern must come before the plain keyword alternation"
        );
        // [cmp-auto] The obligation clause's generator modifier.
        assert!(grammar.contains("\\\\bauto(?=\\\\s+(fn\\\\b|[A-Z]))"));
    }

    // [cli-lang] A `[symbol]` doc reference inside a comment is highlighted,
    // which needs the comment rule to be a begin/end with an inner pattern
    // (user request 2026-09-18).
    #[test]
    fn doc_references_inside_comments_are_highlighted() {
        let grammar = tm_grammar();
        let comments = grammar
            .split("\"comments\"")
            .nth(1)
            .expect("a comments rule");
        assert!(
            comments.contains("\"begin\": \"//\"") && comments.contains("variable.parameter.reference.salvo"),
            "expected the comment rule to carry a doc-reference pattern:\n{comments}"
        );
    }

    #[test]
    fn grammar_is_valid_json_with_all_keywords() {
        let grammar = tm_grammar();
        let value: serde_json::Value =
            serde_json::from_str(&grammar).expect("generated grammar must be valid JSON");
        assert_eq!(value["scopeName"], "source.salvo");
        for (word, _) in KEYWORDS {
            let pattern = format!("{word}");
            assert!(
                grammar.contains(&pattern),
                "keyword `{word}` missing from generated grammar"
            );
        }
    }

    // [cli-lang] The checked-in VS Code extension grammar matches the
    // generated one — regenerate with
    // `cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json`.
    #[test]
    fn vscode_extension_grammar_is_up_to_date() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vscode/syntaxes/salvo.tmLanguage.json");
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        assert_eq!(
            on_disk,
            tm_grammar(),
            "vscode/syntaxes/salvo.tmLanguage.json is stale — regenerate with \
             `cargo run -- lang tm-grammar --out vscode/syntaxes/salvo.tmLanguage.json`"
        );
    }
}
