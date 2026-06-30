//! Canonical, deterministic sorting and printing of GraphQL executable
//! documents.
//!
//! Two semantically equivalent operations written with different field order
//! or whitespace should map to the same string. This crate provides that
//! canonical string, useful as a stable key for persisted operations and
//! allow-lists.
//!
//! # Example
//!
//! ```
//! use graphql_documents::{canonicalize, parse};
//!
//! let document = parse("query A { c b a }").unwrap();
//! assert_eq!(canonicalize(&document), "query A { a b c }");
//! ```
//!
//! # What gets sorted
//!
//! - Fragment definitions print before operations. Within each group, by name.
//! - Operations sort by name.
//! - Variable definitions sort by variable name.
//! - Directive arguments sort by argument name.
//! - Directives sort by name on fragment spreads, inline fragments, and
//!   fragment definitions.
//! - Selection sets order fields first, then fragment spreads, then inline
//!   fragments. Fields and spreads sort by name. Inline fragments sort by type
//!   condition then by their recursively sorted inner selection set.
//!
//! # What is left alone
//!
//! - Field arguments and field directives keep source order.
//! - Operation directives and variable-definition directives keep source order.
//! - A mutation's top-level selections keep source order, since execution order
//!   matters. That carries into inline fragments nested directly inside.
//! - A fragment spread at a mutation's top level keeps that fragment
//!   definition's directives and variable definitions in source order. The
//!   fragment body still sorts.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod ast;
mod normalize;
mod parser;
mod printer;
mod sort;

pub use ast::{
    Argument, Definition, Directive, Document, Field, FragmentDefinition, FragmentSpread,
    InlineFragment, ObjectField, OperationDefinition, OperationType, Selection, SelectionSet, Type,
    Value, VariableDefinition,
};
pub use normalize::normalize_whitespace;
pub use parser::{parse, ParseError};
pub use sort::sort_executable_document;

/// Reduce an executable document to its canonical single-line string.
///
/// The document is sorted into canonical order, printed, then every run of
/// whitespace is collapsed to a single space and the ends are trimmed. Two
/// documents that mean the same thing produce the same string, so the result
/// works as a stable key.
///
/// # Example
///
/// ```
/// use graphql_documents::{canonicalize, parse};
///
/// let document = parse("query A { ...B ...A c b a }").unwrap();
/// assert_eq!(canonicalize(&document), "query A { a b c ...A ...B }");
/// ```
pub fn canonicalize(document: &Document) -> String {
    let sorted = sort_executable_document(document);
    let printed = printer::print(&sorted);
    normalize_whitespace(&printed)
}

/// Render a document as multi-line GraphQL with two-space indent.
///
/// This is the raw pretty-printer. It does not sort and does not collapse
/// whitespace, so the output keeps source order and spans many lines. For the
/// stable single-line key use [`canonicalize`]. This entry point exists for a
/// caller that holds a [`Document`] and wants readable GraphQL back.
pub fn print_pretty(document: &Document) -> String {
    printer::print(document)
}
