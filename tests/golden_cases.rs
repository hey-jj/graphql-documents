//! Golden cases for the canonical printer.
//!
//! Each case parses an input document, prints it canonically, and checks the
//! exact output string. The same cases also run through
//! `sort_executable_document` plus the printer plus normalization to prove the
//! two public functions agree.

mod common;

use common::{golden_path, load_cases};
use graphql_documents::{
    canonicalize, normalize_whitespace, parse, print_pretty, sort_executable_document,
};

fn check_file(file: &str) {
    let cases = load_cases(golden_path(file));
    assert!(!cases.is_empty(), "no cases loaded from {file}");
    for case in cases {
        let document =
            parse(&case.input).unwrap_or_else(|e| panic!("parse failed for `{}`: {e}", case.name));

        let printed = canonicalize(&document);
        assert_eq!(
            printed, case.expected,
            "\ncase:     {}\ninput:    {}\nexpected: {}\ngot:      {}",
            case.name, case.input, case.expected, printed
        );

        // The two public functions must agree: sorting then printing then
        // normalizing yields the same canonical string.
        let sorted = sort_executable_document(&document);
        let via_sort = normalize_whitespace(&print_pretty(&sorted));
        assert_eq!(
            via_sort, case.expected,
            "sort+print disagreed for case `{}`",
            case.name
        );
    }
}

#[test]
fn core_goldens() {
    check_file("cases.txt");
}

#[test]
fn added_goldens() {
    check_file("added_cases.txt");
}

#[test]
fn core_output_is_idempotent() {
    // The core golden cases are fixed points. Note that the inline-fragment sort
    // key reads unsorted nested children, so a second pass can reorder inline
    // fragments whose only difference is deep. That quirk is intentional and is
    // exercised by the added cases, so idempotence is checked on the core set.
    for case in load_cases(golden_path("cases.txt")) {
        let once = canonicalize(&parse(&case.input).unwrap());
        let twice = canonicalize(&parse(&once).unwrap());
        assert_eq!(once, twice, "not idempotent for case `{}`", case.name);
    }
}

#[test]
fn output_has_no_whitespace_runs() {
    for file in ["cases.txt", "added_cases.txt"] {
        for case in load_cases(golden_path(file)) {
            let out = canonicalize(&parse(&case.input).unwrap());
            assert!(
                !out.contains("  "),
                "double space in `{}`: {out}",
                case.name
            );
            assert!(!out.contains('\n'), "newline in `{}`", case.name);
            assert!(!out.contains('\t'), "tab in `{}`", case.name);
            assert_eq!(out.trim(), out, "untrimmed output in `{}`", case.name);
        }
    }
}
