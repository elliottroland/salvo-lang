//! [expr-terminal-lambda] `map(xs, (e) -> ...)` — the last argument's lambda body
//! written `...` becomes the rest of the enclosing block, so a call that takes
//! a lambda can read as a block without nesting.

use salvo_syntax::ast::{Expr, LambdaBody, Stmt};
use salvo_syntax::parse_module;

/// Parse a single-function body and return its statements.
fn body_stmts(src: &str) -> Vec<Stmt> {
    let (module, diags) = parse_module(src);
    let errs: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let Some(salvo_syntax::ast::Item::Fn(f)) = module.items.into_iter().next() else {
        panic!("expected a fn");
    };
    f.body.expect("a body").stmts
}

/// Find the first lambda anywhere in an expression tree.
fn find_lambda(e: &Expr) -> Option<&LambdaBody> {
    match e {
        Expr::Lambda { body, .. } => Some(body),
        Expr::Call { callee, args, .. } => {
            find_lambda(callee).or_else(|| args.iter().find_map(find_lambda))
        }
        Expr::Return { value: Some(v), .. } => find_lambda(v),
        _ => None,
    }
}

// [expr-terminal-lambda] The trailing `(e) -> ...` lambda's body is filled with the
// statements that follow the call — a `Block`, not the transient `Terminal`
// placeholder, by the time parsing is done.
#[test]
fn a_trailing_lambda_captures_the_rest_of_the_block() {
    let stmts = body_stmts(
        "fn f() [] -> None {\n    \
         let out = map(xs, (e) -> ...)\n    \
         discard(e)\n    \
         discard(out)\n\
         }\n",
    );
    // The first statement is `let out = map(xs, (e) -> ...)`; its value holds
    // the lambda, whose body is now the rest of the block (two `discard`s).
    let Stmt::Let { value, .. } = &stmts[0] else {
        panic!("expected a let, got {:?}", stmts[0]);
    };
    let body = find_lambda(value).expect("a lambda in the call");
    match body {
        LambdaBody::Block(b) => assert_eq!(
            b.stmts.len(),
            2,
            "the rest-of-block should be the two trailing statements, got {}",
            b.stmts.len()
        ),
        other => panic!("expected a filled Block body, got {other:?}"),
    }
    // And nothing follows the first statement at the block's own level: the
    // rest became the lambda body.
    assert_eq!(stmts.len(), 1, "the trailing statements moved into the lambda");
}

// [expr-terminal-lambda] Only one terminal lambda per statement — a second `...`
// body is refused.
#[test]
fn two_terminal_lambdas_in_one_statement_are_refused() {
    let (_m, diags) = parse_module(
        "fn f() [] -> None {\n    \
         let x = both(map(a, (p) -> ...), map(b, (q) -> ...))\n    \
         discard(p)\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.is_error()),
        "two `(…) -> ...` lambdas in one statement must be refused"
    );
}
