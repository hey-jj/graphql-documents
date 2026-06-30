# graphql-documents

Turn a GraphQL executable document into a stable, single-line string. Two
operations that mean the same thing but differ in field order or whitespace map
to the same output. Use it as a key for persisted operations and allow-lists.

## Installation

```toml
[dependencies]
graphql-documents = "0.1"
```

The crate has no dependencies. It ships its own GraphQL parser and printer.

## Usage

```rust
use graphql_documents::{canonicalize, parse};

let document = parse("query A { c b a }").unwrap();
let canonical = canonicalize(&document);
assert_eq!(canonical, "query A { a b c }");
```

`sort_executable_document` returns the reordered AST if you want to work with the
tree directly:

```rust
use graphql_documents::{parse, sort_executable_document};

let document = parse("query A { c b a }").unwrap();
let sorted = sort_executable_document(&document);
assert_eq!(sorted.definitions.len(), 1);
```

## Rules

- Fragment definitions print before operations. Within each group, by name.
- Operations sort by name. An anonymous operation sorts after named ones.
- Variable definitions sort by variable name.
- Directive arguments sort by argument name.
- Directives sort by name on fragment spreads, inline fragments, and fragment
  definitions.
- Selection sets order fields first, then fragment spreads, then inline
  fragments. Fields and spreads sort by name. Inline fragments sort by type
  condition then by their inner selection set sorted one level deep. Only the
  immediate inner selections affect that ordering, not deeper field order.

What stays in source order:

- Field arguments and field directives.
- Operation directives and variable-definition directives.
- A mutation's top-level selections, since execution order matters. This carries
  into inline fragments nested directly inside.
- The directives and variable definitions of a fragment spread at a mutation's
  top level. The fragment body still sorts.

The printed output collapses every run of whitespace to one space and trims the
ends, so the result is always a single line.

## Limits

The canonical string is not a collision-free hash of the request text. Both
sides of a comparison must canonicalize. Whitespace runs collapse everywhere,
including inside string literals, so two string values that differ only in
whitespace map to the same output. To match the exact bytes a client sent, hash
the client string directly.

## License

Licensed under the [MIT license](LICENSE).
