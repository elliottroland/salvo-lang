// [fn-overload-at] [diag-structured] A redundant selector is a **warning**:
// the program is correct, so it must still compile. (That the warning
// reaches the CLI's report and its JSON is `analyze_tests.rs`'s
// `a_redundant_selector_warns_without_failing`.)
fn main() [use] {
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    //~ warning[fn-overload-at:not-needed]
    let _n = size@core.list(xs)
}
