//! Compile-time contract of the macros: what is accepted and how rejections
//! are reported.
//!
//! The expected diagnostics are pinned to the MSRV toolchain. Set
//! `TYPEDFLAKE_SKIP_UI` to skip this test on other compilers, and
//! `TRYBUILD=overwrite` to regenerate the `.stderr` files.

#[test]
fn ui() {
    if std::env::var_os("TYPEDFLAKE_SKIP_UI").is_some() {
        return;
    }

    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/*.rs");
    cases.compile_fail("tests/ui/fail/*.rs");

    #[cfg(all(feature = "serde", feature = "sqlx-postgres", feature = "postgres"))]
    cases.compile_fail("tests/ui/fail_all_features/*.rs");

    #[cfg(not(any(feature = "serde", feature = "sqlx-postgres", feature = "postgres")))]
    cases.compile_fail("tests/ui/fail_no_features/*.rs");
}
