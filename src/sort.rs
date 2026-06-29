//! Canonical sorting of an executable document.
//!
//! The transform reorders definitions, selections, arguments, variable
//! definitions, and directives into a stable, name-based order. Mutation
//! top-level selections keep source order because mutation field execution
//! order is significant. That "do not sort" property propagates into inline
//! fragments nested directly inside the preserved selection set. A fragment
//! spread at that preserved level also pins the referenced fragment
//! definition's directive and variable-definition order.

use crate::ast::*;
use crate::printer::print_selection;
use std::collections::HashSet;

const PREFIX_FIELD: &str = "0";
const PREFIX_FRAGMENT_SPREAD: &str = "1";
const PREFIX_INLINE_FRAGMENT: &str = "2";

/// Return a new document with all nodes reordered into canonical order.
///
/// Rules:
/// - Fragment definitions print before operation definitions. Within each
///   group, definitions sort by name.
/// - Variable definitions sort by variable name.
/// - Directive arguments sort by argument name.
/// - Directives sort by name on fragment spreads, inline fragments, and
///   fragment definitions. Directives on operations, fields, and variable
///   definitions keep source order.
/// - Selection sets order fields first, then fragment spreads, then inline
///   fragments. Fields and spreads sort by name. Inline fragments sort by type
///   condition then by their recursively sorted, printed inner selection set.
/// - Field arguments are never sorted.
/// - A mutation's top-level selection set keeps source order, and so does any
///   inline fragment nested directly within it, recursively.
pub fn sort_executable_document(document: &Document) -> Document {
    let ignored_fragments = collect_ignored_fragments(document);
    let mut definitions: Vec<Definition> = document.definitions.clone();
    sort_definitions(&mut definitions);
    let definitions = definitions
        .iter()
        .map(|def| sort_definition(def, &ignored_fragments))
        .collect();
    Document { definitions }
}

/// Collect names of fragments spread at a mutation's preserved top level.
///
/// A fragment spread that is a direct child of a preserved selection list marks
/// its target fragment as ignored. The preserved region starts at a mutation's
/// top-level selection set and propagates through inline fragments nested
/// directly inside, recursively. It does not propagate through field children.
fn collect_ignored_fragments(document: &Document) -> HashSet<String> {
    let mut ignored = HashSet::new();
    for def in &document.definitions {
        if let Definition::Operation(op) = def {
            if op.operation == OperationType::Mutation {
                collect_ignored_in_set(&op.selection_set, &mut ignored);
            }
        }
    }
    ignored
}

/// Record fragment spreads directly in a preserved selection set and recurse
/// into inline-fragment children, which stay preserved.
fn collect_ignored_in_set(set: &SelectionSet, ignored: &mut HashSet<String>) {
    for selection in &set.selections {
        match selection {
            Selection::FragmentSpread(spread) => {
                ignored.insert(spread.name.clone());
            }
            Selection::InlineFragment(inline) => {
                collect_ignored_in_set(&inline.selection_set, ignored);
            }
            Selection::Field(_) => {}
        }
    }
}

/// Order definitions by kind then name. Fragment kind sorts before operation
/// kind, matching the GraphQL kind strings `"FragmentDefinition"` and
/// `"OperationDefinition"`.
fn sort_definitions(definitions: &mut [Definition]) {
    definitions.sort_by(|a, b| {
        let ka = definition_kind_rank(a);
        let kb = definition_kind_rank(b);
        ka.cmp(&kb)
            .then_with(|| optional_name_cmp(definition_name(a), definition_name(b)))
    });
}

fn definition_kind_rank(def: &Definition) -> u8 {
    // "FragmentDefinition" < "OperationDefinition" lexicographically.
    match def {
        Definition::Fragment(_) => 0,
        Definition::Operation(_) => 1,
    }
}

fn definition_name(def: &Definition) -> Option<&str> {
    match def {
        Definition::Fragment(f) => Some(&f.name),
        Definition::Operation(o) => o.name.as_deref(),
    }
}

/// Compare optional names the way lodash compares a missing `name.value`. A
/// present name sorts before a missing one, and two present names compare by
/// UTF-16 code units.
fn optional_name_cmp(a: Option<&str>, b: Option<&str>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => utf16_cmp(a, b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn sort_definition(def: &Definition, ignored_fragments: &HashSet<String>) -> Definition {
    match def {
        Definition::Operation(op) => Definition::Operation(sort_operation(op)),
        Definition::Fragment(frag) => {
            Definition::Fragment(sort_fragment_definition(frag, ignored_fragments))
        }
    }
}

fn sort_operation(op: &OperationDefinition) -> OperationDefinition {
    let ignored = op.operation == OperationType::Mutation;
    OperationDefinition {
        operation: op.operation,
        name: op.name.clone(),
        variable_definitions: sort_variable_definitions(&op.variable_definitions),
        // Operation directives keep order. Their arguments still sort.
        directives: sort_directive_arguments(&op.directives),
        selection_set: sort_selection_set(&op.selection_set, ignored),
    }
}

fn sort_fragment_definition(
    frag: &FragmentDefinition,
    ignored_fragments: &HashSet<String>,
) -> FragmentDefinition {
    // A fragment spread at a mutation's preserved top level suppresses sorting
    // of this fragment's directive list and variable-definition list. The body
    // still sorts, and directive arguments still sort, because those live in
    // separate nodes that the suppression does not reach.
    if ignored_fragments.contains(&frag.name) {
        return FragmentDefinition {
            name: frag.name.clone(),
            variable_definitions: sort_ignored_variable_definitions(&frag.variable_definitions),
            type_condition: frag.type_condition.clone(),
            directives: sort_directive_arguments(&frag.directives),
            selection_set: sort_selection_set(&frag.selection_set, false),
        };
    }
    FragmentDefinition {
        name: frag.name.clone(),
        variable_definitions: sort_variable_definitions(&frag.variable_definitions),
        type_condition: frag.type_condition.clone(),
        directives: sort_directives(&frag.directives),
        selection_set: sort_selection_set(&frag.selection_set, false),
    }
}

fn sort_variable_definitions(defs: &[VariableDefinition]) -> Vec<VariableDefinition> {
    let mut defs = sort_ignored_variable_definitions(defs);
    defs.sort_by(|a, b| utf16_cmp(&a.variable, &b.variable));
    defs
}

/// Transform each variable definition but keep the list in source order.
///
/// Directive arguments still sort. The directive list and the list of variable
/// definitions both keep source order.
fn sort_ignored_variable_definitions(defs: &[VariableDefinition]) -> Vec<VariableDefinition> {
    defs.iter()
        .map(|d| VariableDefinition {
            variable: d.variable.clone(),
            ty: d.ty.clone(),
            default_value: d.default_value.clone(),
            // Variable-definition directives keep order. Their arguments sort.
            directives: sort_directive_arguments(&d.directives),
        })
        .collect()
}

/// Sort each directive's arguments and reorder the list by directive name.
fn sort_directives(directives: &[Directive]) -> Vec<Directive> {
    let mut directives = sort_directive_arguments(directives);
    directives.sort_by(|a, b| utf16_cmp(&a.name, &b.name));
    directives
}

/// Sort each directive's arguments but keep the directive list in source order.
///
/// Every directive in the tree gets its arguments sorted, even when the
/// directive list around it is not reordered.
fn sort_directive_arguments(directives: &[Directive]) -> Vec<Directive> {
    directives
        .iter()
        .map(|d| Directive {
            name: d.name.clone(),
            arguments: sort_arguments(&d.arguments),
        })
        .collect()
}

fn sort_arguments(args: &[Argument]) -> Vec<Argument> {
    let mut args: Vec<Argument> = args.to_vec();
    args.sort_by(|a, b| utf16_cmp(&a.name, &b.name));
    args
}

/// Sort or preserve a selection set. When `ignored` is true the selection order
/// is left untouched and the ignore flag propagates into inline-fragment
/// children.
fn sort_selection_set(set: &SelectionSet, ignored: bool) -> SelectionSet {
    if ignored {
        let selections = set.selections.iter().map(sort_ignored_selection).collect();
        return SelectionSet { selections };
    }

    // Order by keys computed from the original nodes, then recurse into the
    // reordered nodes. Order is fixed before descending into children, so keys
    // see source-order nested children.
    let mut ordered: Vec<&Selection> = set.selections.iter().collect();
    ordered.sort_by(|a, b| utf16_cmp(&selection_key(a), &selection_key(b)));
    let selections = ordered.into_iter().map(sort_selection).collect();
    SelectionSet { selections }
}

/// Transform a selection inside a preserved (mutation) selection set.
fn sort_ignored_selection(selection: &Selection) -> Selection {
    match selection {
        // A field keeps its position but its own nested set sorts normally.
        Selection::Field(field) => Selection::Field(sort_field(field)),
        // A spread keeps its position. Its directives are still sorted.
        Selection::FragmentSpread(spread) => Selection::FragmentSpread(FragmentSpread {
            name: spread.name.clone(),
            directives: sort_directives(&spread.directives),
        }),
        // An inline fragment keeps its position, keeps its directive list in
        // source order, and propagates the ignore flag into its body. Directive
        // arguments still sort.
        Selection::InlineFragment(inline) => Selection::InlineFragment(InlineFragment {
            type_condition: inline.type_condition.clone(),
            directives: sort_directive_arguments(&inline.directives),
            selection_set: sort_selection_set(&inline.selection_set, true),
        }),
    }
}

/// Transform a selection inside a normally sorted selection set.
fn sort_selection(selection: &Selection) -> Selection {
    match selection {
        Selection::Field(field) => Selection::Field(sort_field(field)),
        Selection::FragmentSpread(spread) => Selection::FragmentSpread(FragmentSpread {
            name: spread.name.clone(),
            directives: sort_directives(&spread.directives),
        }),
        Selection::InlineFragment(inline) => Selection::InlineFragment(InlineFragment {
            type_condition: inline.type_condition.clone(),
            directives: sort_directives(&inline.directives),
            selection_set: sort_selection_set(&inline.selection_set, false),
        }),
    }
}

fn sort_field(field: &Field) -> Field {
    Field {
        alias: field.alias.clone(),
        name: field.name.clone(),
        // Field arguments are never sorted.
        arguments: field.arguments.clone(),
        // Field directive list keeps order. Their arguments still sort.
        directives: sort_directive_arguments(&field.directives),
        selection_set: field
            .selection_set
            .as_ref()
            .map(|set| sort_selection_set(set, false)),
    }
}

/// Build the string sort key for a selection.
///
/// Fields are prefixed `0`, spreads `1`, inline fragments `2`, so the three
/// groups order field, spread, inline. The suffix breaks ties: the name for
/// fields and spreads, the type condition plus the printed sorted inner
/// selection set for inline fragments.
fn selection_key(selection: &Selection) -> String {
    match selection {
        Selection::Field(field) => format!("{PREFIX_FIELD}{}", field.name),
        Selection::FragmentSpread(spread) => format!("{PREFIX_FRAGMENT_SPREAD}{}", spread.name),
        Selection::InlineFragment(inline) => {
            let type_condition = inline.type_condition.as_deref().unwrap_or("");
            let inner = build_inline_fragment_key(&inline.selection_set);
            format!("{PREFIX_INLINE_FRAGMENT}{type_condition}{inner}")
        }
    }
}

/// Build the inner-selection-set component of an inline-fragment sort key.
///
/// The immediate inner selections are reordered by their sort keys, then each
/// is printed as written and the printed forms are joined with single spaces
/// and whitespace-normalized. Reordering is one level deep: nested children of
/// each selection are printed in their source order, not recursively sorted.
fn build_inline_fragment_key(set: &SelectionSet) -> String {
    let mut ordered: Vec<&Selection> = set.selections.iter().collect();
    ordered.sort_by(|a, b| utf16_cmp(&selection_key(a), &selection_key(b)));
    let joined = ordered
        .iter()
        .map(|s| print_selection(s))
        .collect::<Vec<_>>()
        .join(" ");
    crate::normalize::normalize_whitespace(&joined)
}

/// Compare two strings by UTF-16 code units, matching JavaScript's default
/// string comparison. ASCII names are unaffected, but this keeps parity for
/// keys that contain non-ASCII characters from string-literal values.
fn utf16_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}
