//! [test-run] The Salvo test runner: discovery, harness synthesis, and the
//! report.
//!
//! `salvo test` is a thin CLI subcommand over this crate — the split
//! `salvo run` already has between the driver and what it drives (user
//! decision 2026-09-23, TF-7 option A). What lives here is everything that is
//! *about testing* and nothing that is about parsing a command line:
//!
//! * [`harness`] — the generated Salvo program that runs the tests,
//! * [`report`] — the report it prints, coloured and timed,
//! * [`select`] — which tests a run includes.
//!
//! Not to be confused with `salvo-testkit`, which is the *compiler's* own test
//! toolbox (toolchain probing, the e2e content cache). The two never depend on
//! each other.

pub mod harness;
pub mod report;

pub use harness::{harness_source, HARNESS_MODULE};
pub use report::{died_detail, print_summary, render, render_stream, Color, Summary};

use salvo_core::TestCase;

/// [test-filter] The tests a run includes: every one whose id contains
/// `filter` as a substring, or all of them when there is no filter.
///
/// Substring rather than a pattern language, and over the whole id
/// (`heap :: pops come out in order`), so one word selects a module, a test,
/// or a family of tests without a syntax to learn — Go's `-run`, simplified.
pub fn select(tests: &[TestCase], filter: Option<&str>) -> Vec<TestCase> {
    tests
        .iter()
        .filter(|t| filter.is_none_or(|f| t.id().contains(f)))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use salvo_core::ModulePath;

    fn case(module: &str, name: &str) -> TestCase {
        TestCase {
            file: 0,
            tested: ModulePath(vec![module.to_string()]),
            name: name.to_string(),
            fn_name: "__salvo_test_0".to_string(),
        }
    }

    /// [test-filter] The filter matches anywhere in the full id — the module
    /// name included.
    #[test]
    fn filters_by_substring_of_the_id() {
        let tests = vec![case("heap", "pops in order"), case("list", "size of empty")];
        assert_eq!(select(&tests, None).len(), 2);
        assert_eq!(select(&tests, Some("heap")).len(), 1);
        assert_eq!(select(&tests, Some("size")).len(), 1);
        assert_eq!(select(&tests, Some("nothing")).len(), 0);
    }
}
