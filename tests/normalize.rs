//! Unit tests for whitespace normalization.
//!
//! These mirror `str.replace(/\s+/g, ' ').trim()`, including the wider Unicode
//! whitespace that the ECMAScript `\s` class matches.

use graphql_documents::normalize_whitespace;

#[test]
fn collapses_runs_and_trims() {
    assert_eq!(
        normalize_whitespace("  query   A {\n a\t b }\n"),
        "query A { a b }"
    );
    assert_eq!(normalize_whitespace("a b c"), "a b c");
    assert_eq!(normalize_whitespace("   spaces"), "spaces");
    assert_eq!(normalize_whitespace("trailing   "), "trailing");
    assert_eq!(normalize_whitespace("no_ws"), "no_ws");
}

#[test]
fn empty_and_all_whitespace() {
    assert_eq!(normalize_whitespace(""), "");
    assert_eq!(normalize_whitespace("   "), "");
    assert_eq!(normalize_whitespace("\n\t\r"), "");
}

#[test]
fn ascii_control_whitespace() {
    // Vertical tab and form feed are part of the JS whitespace class.
    assert_eq!(
        normalize_whitespace("\u{0b}vtab\u{0c}ff nbsp"),
        "vtab ff nbsp"
    );
}

#[test]
fn unicode_whitespace() {
    // Ideographic space and the byte-order mark both collapse to one space.
    assert_eq!(normalize_whitespace("x\u{3000}y\u{feff}z"), "x y z");
    // No-break space and narrow no-break space.
    assert_eq!(normalize_whitespace("a\u{00a0}b\u{202f}c"), "a b c");
    // Line and paragraph separators.
    assert_eq!(normalize_whitespace("p\u{2028}q\u{2029}r"), "p q r");
}
