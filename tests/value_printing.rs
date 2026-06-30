//! Canonical printing of string and value edge cases.
//!
//! These inputs carry control characters, C1 controls, supplementary-plane
//! characters, and Unicode whitespace that a `| `-separated fixture file cannot
//! hold readably. Each case pins the exact canonical string.

use graphql_documents::{canonicalize, parse};

fn canonical(input: &str) -> String {
    canonicalize(&parse(input).unwrap())
}

#[test]
fn low_control_chars_escape_to_uppercase_hex() {
    assert_eq!(
        canonical("query A { f(x: \"\u{1}\") }"),
        "query A { f(x: \"\\u0001\") }"
    );
}

#[test]
fn del_escapes_to_uppercase_hex() {
    assert_eq!(
        canonical("query A { f(x: \"x\u{7f}y\") }"),
        "query A { f(x: \"x\\u007Fy\") }"
    );
}

#[test]
fn c1_controls_escape() {
    // U+0080 through U+009F are the C1 block. The canonical printer escapes
    // them. They are not part of the JS whitespace class, so without escaping
    // they would print raw and break key equality across producers.
    assert_eq!(
        canonical("query A { f(x: \"a\u{80}b\") }"),
        "query A { f(x: \"a\\u0080b\") }"
    );
    assert_eq!(
        canonical("query A { f(x: \"a\u{85}b\") }"),
        "query A { f(x: \"a\\u0085b\") }"
    );
    assert_eq!(
        canonical("query A { f(x: \"a\u{9f}b\") }"),
        "query A { f(x: \"a\\u009Fb\") }"
    );
}

#[test]
fn nbsp_inside_string_collapses() {
    // A no-break space is part of the JS whitespace class, so normalization
    // collapses it to a single space.
    assert_eq!(
        canonical("query A { f(x: \"a\u{a0}b\") }"),
        "query A { f(x: \"a b\") }"
    );
}

#[test]
fn inline_fragment_keys_order_by_utf16_code_units() {
    // U+10000 sorts before U+FFFF under UTF-16 because its leading surrogate is
    // U+D800, below U+FFFF. Scalar or byte order would reverse the two inline
    // fragments. The order below proves the key compares UTF-16 code units.
    let input = "query A { ... on Q { f(x: \"\u{10000}\") } ... on Q { f(x: \"\u{FFFF}\") } }";
    assert_eq!(canonical(input), input);
}
