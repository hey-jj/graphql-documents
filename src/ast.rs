//! GraphQL executable document AST.
//!
//! The node types here cover the executable subset of the GraphQL grammar:
//! operations, fragments, selection sets, arguments, directives, variable
//! definitions, type references, and values. Type system definitions are out
//! of scope.

/// A parsed GraphQL document. Holds the top-level definitions in source order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    /// Top-level definitions: operations and fragment definitions.
    pub definitions: Vec<Definition>,
}

/// A top-level definition in a document.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Definition {
    /// An operation: query, mutation, or subscription.
    Operation(OperationDefinition),
    /// A fragment definition.
    Fragment(FragmentDefinition),
}

/// The three operation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    /// A read operation.
    Query,
    /// A write operation. Its top-level selections keep source order.
    Mutation,
    /// A long-lived event stream.
    Subscription,
}

impl OperationType {
    /// The keyword used when printing this operation type.
    pub fn keyword(self) -> &'static str {
        match self {
            OperationType::Query => "query",
            OperationType::Mutation => "mutation",
            OperationType::Subscription => "subscription",
        }
    }
}

/// An operation definition.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationDefinition {
    /// query, mutation, or subscription.
    pub operation: OperationType,
    /// The operation name, if any. Anonymous operations have `None`.
    pub name: Option<String>,
    /// Variable definitions declared on the operation.
    pub variable_definitions: Vec<VariableDefinition>,
    /// Directives applied to the operation.
    pub directives: Vec<Directive>,
    /// The root selection set.
    pub selection_set: SelectionSet,
}

/// A fragment definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FragmentDefinition {
    /// The fragment name.
    pub name: String,
    /// Variable definitions declared on the fragment.
    pub variable_definitions: Vec<VariableDefinition>,
    /// The type the fragment applies to.
    pub type_condition: String,
    /// Directives applied to the fragment.
    pub directives: Vec<Directive>,
    /// The fragment body.
    pub selection_set: SelectionSet,
}

/// A variable definition: `$name: Type = default @directives`.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableDefinition {
    /// The variable name without the leading `$`.
    pub variable: String,
    /// The declared type.
    pub ty: Type,
    /// The default value, if any.
    pub default_value: Option<Value>,
    /// Directives applied to the variable definition.
    pub directives: Vec<Directive>,
}

/// A type reference: named, list, or non-null.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Type {
    /// A named type like `Int`.
    Named(String),
    /// A list type like `[Int]`.
    List(Box<Type>),
    /// A non-null type like `Int!`.
    NonNull(Box<Type>),
}

/// A set of selections enclosed in braces.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectionSet {
    /// The selections in this set.
    pub selections: Vec<Selection>,
}

/// One member of a selection set.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Selection {
    /// A field selection.
    Field(Field),
    /// A fragment spread `...Name`.
    FragmentSpread(FragmentSpread),
    /// An inline fragment `... on Type { ... }`.
    InlineFragment(InlineFragment),
}

/// A field selection: `alias: name(args) @directives { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The field alias, if any.
    pub alias: Option<String>,
    /// The field name.
    pub name: String,
    /// Field arguments. These are never sorted.
    pub arguments: Vec<Argument>,
    /// Directives applied to the field.
    pub directives: Vec<Directive>,
    /// The nested selection set, if any.
    pub selection_set: Option<SelectionSet>,
}

/// A fragment spread: `...Name @directives`.
#[derive(Debug, Clone, PartialEq)]
pub struct FragmentSpread {
    /// The referenced fragment name.
    pub name: String,
    /// Directives applied to the spread.
    pub directives: Vec<Directive>,
}

/// An inline fragment: `... on Type @directives { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineFragment {
    /// The type condition, if `on Type` is present.
    pub type_condition: Option<String>,
    /// Directives applied to the inline fragment.
    pub directives: Vec<Directive>,
    /// The fragment body.
    pub selection_set: SelectionSet,
}

/// An argument: `name: value`.
#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    /// The argument name.
    pub name: String,
    /// The argument value.
    pub value: Value,
}

/// A directive: `@name(args)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Directive {
    /// The directive name without the leading `@`.
    pub name: String,
    /// Directive arguments. These are sorted by name.
    pub arguments: Vec<Argument>,
}

/// A GraphQL value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// A variable reference `$name`.
    Variable(String),
    /// An integer literal, kept as written.
    Int(String),
    /// A float literal, kept as written.
    Float(String),
    /// A string literal. The flag marks a block string.
    String {
        /// The decoded string content.
        value: String,
        /// True if this was written as a `"""..."""` block string.
        block: bool,
    },
    /// A boolean literal.
    Boolean(bool),
    /// The null literal.
    Null,
    /// An enum value.
    Enum(String),
    /// A list value `[ ... ]`.
    List(Vec<Value>),
    /// An object value `{ name: value, ... }`.
    Object(Vec<ObjectField>),
}

/// One entry in an object value.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectField {
    /// The field name.
    pub name: String,
    /// The field value.
    pub value: Value,
}
