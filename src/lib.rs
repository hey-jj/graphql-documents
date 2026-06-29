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
//! use graphql_executable_documents::{parse, print_executable_graphql_document};
//!
//! let document = parse("query A { c b a }").unwrap();
//! assert_eq!(print_executable_graphql_document(&document), "query A { a b c }");
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

/// Print an executable document in a stable, single-line form.
///
/// The document is sorted into canonical order, printed, then every run of
/// whitespace is collapsed to a single space and the ends are trimmed.
///
/// # Example
///
/// ```
/// use graphql_executable_documents::{parse, print_executable_graphql_document};
///
/// let document = parse("query A { ...B ...A c b a }").unwrap();
/// assert_eq!(
///     print_executable_graphql_document(&document),
///     "query A { a b c ...A ...B }"
/// );
/// ```
pub fn print_executable_graphql_document(document: &Document) -> String {
    let sorted = sort_executable_document(document);
    let printed = printer::print(&sorted);
    normalize_whitespace(&printed)
}

/// Print a sorted document directly. Convenience over parse then print.
///
/// This is the printer used internally. It is exposed so a caller holding a
/// [`Document`] can render it without re-parsing.
pub fn print(document: &Document) -> String {
    printer::print(document)
}
