//! Behavior tests for parsing, the AST shape, and structural invariants.

use graphql_executable_documents::{
    parse, print_executable_graphql_document, sort_executable_document, Definition, OperationType,
    Selection,
};

#[test]
fn parse_rejects_empty_source() {
    assert!(parse("").is_err());
    assert!(parse("   ").is_err());
    assert!(parse("# only a comment\n").is_err());
}

#[test]
fn parse_rejects_unterminated_selection_set() {
    assert!(parse("query A { a").is_err());
    assert!(parse("query A {").is_err());
}

#[test]
fn parse_rejects_unknown_definition() {
    assert!(parse("type Foo { a: Int }").is_err());
    assert!(parse("nonsense A { a }").is_err());
}

#[test]
fn comments_and_commas_are_ignored() {
    let out = print_executable_graphql_document(
        &parse("query A { # leading comment\n c, b, a }").unwrap(),
    );
    assert_eq!(out, "query A { a b c }");
}

#[test]
fn anonymous_query_keeps_shorthand() {
    let out = print_executable_graphql_document(&parse("{ b a }").unwrap());
    assert_eq!(out, "{ a b }");
}

#[test]
fn anonymous_subscription_keeps_keyword() {
    // Only an anonymous query drops its keyword. Other operations keep it.
    let out = print_executable_graphql_document(&parse("subscription { b a }").unwrap());
    assert_eq!(out, "subscription { a b }");
}

#[test]
fn sort_returns_owned_document() {
    let doc = parse("query A { c b a }").unwrap();
    let sorted = sort_executable_document(&doc);
    assert_eq!(sorted.definitions.len(), 1);
    match &sorted.definitions[0] {
        Definition::Operation(op) => {
            assert_eq!(op.operation, OperationType::Query);
            assert_eq!(op.name.as_deref(), Some("A"));
            let names: Vec<&str> = op
                .selection_set
                .selections
                .iter()
                .map(|s| match s {
                    Selection::Field(f) => f.name.as_str(),
                    _ => panic!("expected fields"),
                })
                .collect();
            assert_eq!(names, ["a", "b", "c"]);
        }
        _ => panic!("expected an operation"),
    }
}

#[test]
fn fragments_print_before_operations() {
    let out = print_executable_graphql_document(
        &parse("query Z { a } fragment Y on Q { a } query A { a } fragment X on Q { a }").unwrap(),
    );
    assert_eq!(
        out,
        "fragment X on Q { a } fragment Y on Q { a } query A { a } query Z { a }"
    );
}

#[test]
fn stable_order_for_equal_keys() {
    // Two inline fragments with the same type condition and identical inner
    // selection set keep input order under a stable sort.
    let out = print_executable_graphql_document(
        &parse("query A { ... on Q { a } ... on Q { a } }").unwrap(),
    );
    assert_eq!(out, "query A { ... on Q { a } ... on Q { a } }");
}

#[test]
fn variable_definitions_and_default_values_round_trip() {
    let out = print_executable_graphql_document(
        &parse("query A($b: [Int!] = [1, 2], $a: String!) { f }").unwrap(),
    );
    assert_eq!(out, "query A($a: String!, $b: [Int!] = [1, 2]) { f }");
}
