//! Printer for GraphQL executable documents.
//!
//! Output matches the canonical GraphQL printer: two-space indent,
//! blank lines between top-level definitions, and a line-length rule that wraps
//! long argument, list, object, and variable-definition lists across multiple
//! lines. The public [`crate::print_executable_graphql_document`] then collapses
//! the multi-line form into a single line, so the wrapping survives only as the
//! loss of commas between wrapped items.

use crate::ast::*;

/// Wrap argument, list, and object lists onto multiple lines past this width.
const MAX_LINE_LENGTH: usize = 80;

/// Print a whole document in canonical multi-line form.
pub fn print(document: &Document) -> String {
    document
        .definitions
        .iter()
        .map(print_definition)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Print a single selection node. Used to build inline-fragment sort keys.
pub fn print_selection(selection: &Selection) -> String {
    match selection {
        Selection::Field(field) => print_field(field),
        Selection::FragmentSpread(spread) => print_fragment_spread(spread),
        Selection::InlineFragment(inline) => print_inline_fragment(inline),
    }
}

fn print_definition(definition: &Definition) -> String {
    match definition {
        Definition::Operation(op) => print_operation(op),
        Definition::Fragment(frag) => print_fragment_definition(frag),
    }
}

fn print_operation(op: &OperationDefinition) -> String {
    let var_defs = print_operation_variable_definitions(&op.variable_definitions);
    let directives = print_directives(&op.directives);
    let selection_set = print_selection_set(&op.selection_set);

    let name_with_vars = join_filtered(&[op.name.clone().unwrap_or_default(), var_defs], "");
    let prefix = join_filtered(
        &[
            op.operation.keyword().to_string(),
            name_with_vars,
            directives,
        ],
        " ",
    );

    // The bare keyword "query" means an anonymous query with no variables and
    // no directives. It prints just the selection set.
    if prefix == "query" {
        selection_set
    } else {
        format!("{prefix} {selection_set}")
    }
}

fn print_fragment_definition(frag: &FragmentDefinition) -> String {
    let var_def_parts: Vec<String> = frag
        .variable_definitions
        .iter()
        .map(print_variable_definition)
        .collect();
    let var_defs = wrap("(", &join_filtered(&var_def_parts, ", "), ")");
    let directives = print_directives(&frag.directives);
    let selection_set = print_selection_set(&frag.selection_set);
    format!(
        "fragment {}{} on {} {}{}",
        frag.name,
        var_defs,
        frag.type_condition,
        wrap("", &directives, " "),
        selection_set
    )
}

/// Print operation variable definitions. Multi-line if any printed definition
/// already spans lines, matching `hasMultilineItems`.
fn print_operation_variable_definitions(defs: &[VariableDefinition]) -> String {
    if defs.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = defs.iter().map(print_variable_definition).collect();
    if parts.iter().any(|p| p.contains('\n')) {
        format!("(\n{}\n)", parts.join("\n"))
    } else {
        format!("({})", parts.join(", "))
    }
}

fn print_variable_definition(def: &VariableDefinition) -> String {
    let mut out = format!("${}: {}", def.variable, print_type(&def.var_type));
    if let Some(default) = &def.default_value {
        out.push_str(" = ");
        out.push_str(&print_value(default));
    }
    out.push_str(&wrap(" ", &print_directives(&def.directives), ""));
    out
}

fn print_type(ty: &Type) -> String {
    match ty {
        Type::Named(name) => name.clone(),
        Type::List(inner) => format!("[{}]", print_type(inner)),
        Type::NonNull(inner) => format!("{}!", print_type(inner)),
    }
}

fn print_selection_set(set: &SelectionSet) -> String {
    let parts: Vec<String> = set.selections.iter().map(print_selection).collect();
    block(&parts)
}

fn print_field(field: &Field) -> String {
    let mut prefix = String::new();
    if let Some(alias) = &field.alias {
        prefix.push_str(alias);
        prefix.push_str(": ");
    }
    prefix.push_str(&field.name);

    let args: Vec<String> = field.arguments.iter().map(print_argument).collect();
    let head = wrapped_line_and_args(&prefix, &args);

    let directives = print_directives(&field.directives);
    let selection_set = field
        .selection_set
        .as_ref()
        .map(print_selection_set)
        .unwrap_or_default();

    join_filtered(
        &[
            head,
            wrap(" ", &directives, ""),
            wrap(" ", &selection_set, ""),
        ],
        "",
    )
}

fn print_fragment_spread(spread: &FragmentSpread) -> String {
    let prefix = format!("...{}", spread.name);
    let head = wrapped_line_and_args(&prefix, &[]);
    let directives = print_directives(&spread.directives);
    format!("{head}{}", wrap(" ", &directives, ""))
}

fn print_inline_fragment(inline: &InlineFragment) -> String {
    let type_part = inline
        .type_condition
        .as_ref()
        .map(|tc| format!("on {tc}"))
        .unwrap_or_default();
    let directives = print_directives(&inline.directives);
    let selection_set = print_selection_set(&inline.selection_set);
    join_filtered(
        &["...".to_string(), type_part, directives, selection_set],
        " ",
    )
}

fn print_argument(arg: &Argument) -> String {
    format!("{}: {}", arg.name, print_value(&arg.value))
}

/// Print a directive list joined by spaces. An empty list prints as empty.
fn print_directives(directives: &[Directive]) -> String {
    let parts: Vec<String> = directives.iter().map(print_directive).collect();
    join_filtered(&parts, " ")
}

fn print_directive(directive: &Directive) -> String {
    let args: Vec<String> = directive.arguments.iter().map(print_argument).collect();
    // Directive arguments never wrap.
    format!("@{}{}", directive.name, wrap("(", &args.join(", "), ")"))
}

fn print_value(value: &Value) -> String {
    match value {
        Value::Variable(name) => format!("${name}"),
        Value::Int(text) => text.clone(),
        Value::Float(text) => text.clone(),
        Value::String { value, block } => {
            if *block {
                print_block_string(value)
            } else {
                print_string(value)
            }
        }
        Value::Boolean(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Enum(name) => name.clone(),
        Value::List(items) => {
            let values: Vec<String> = items.iter().map(print_value).collect();
            let line = format!("[{}]", values.join(", "));
            if utf16_len(&line) > MAX_LINE_LENGTH {
                format!("[\n{}\n]", indent(&values.join("\n")))
            } else {
                line
            }
        }
        Value::Object(fields) => {
            let entries: Vec<String> = fields
                .iter()
                .map(|f| format!("{}: {}", f.name, print_value(&f.value)))
                .collect();
            let line = format!("{{ {} }}", entries.join(", "));
            if utf16_len(&line) > MAX_LINE_LENGTH {
                block(&entries)
            } else {
                line
            }
        }
    }
}

/// Escape and quote a string the way the canonical GraphQL printer does.
///
/// Named escapes are used for backspace, form feed, newline, carriage return,
/// tab, quote, and backslash. Other characters below U+0020 plus U+007F become
/// `\uXXXX`. The forward slash is not escaped.
fn print_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' || c == '\u{7f}' => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Print a block string with `"""` delimiters, matching the canonical GraphQL printer.
///
/// The value is escaped, then leading and trailing newlines are added based on
/// whether it must print across multiple lines. A short single-line value prints
/// inline as `"""text"""`.
fn print_block_string(value: &str) -> String {
    let escaped = value.replace("\"\"\"", "\\\"\"\"");
    let lines: Vec<&str> = split_string_lines(&escaped);
    let is_single_line = lines.len() == 1;

    let force_leading_newline = lines.len() > 1
        && lines[1..]
            .iter()
            .all(|line| line.is_empty() || starts_with_space_or_tab(line));

    let has_trailing_triple_quotes = escaped.ends_with("\\\"\"\"");
    let has_trailing_quote = value.ends_with('"') && !has_trailing_triple_quotes;
    let has_trailing_slash = value.ends_with('\\');
    let force_trailing_newline = has_trailing_quote || has_trailing_slash;

    let print_as_multiple_lines = !is_single_line
        || utf16_len(value) > 70
        || force_trailing_newline
        || force_leading_newline
        || has_trailing_triple_quotes;

    let mut result = String::new();
    let skip_leading_newline = is_single_line && first_is_space_or_tab(value);
    if (print_as_multiple_lines && !skip_leading_newline) || force_leading_newline {
        result.push('\n');
    }
    result.push_str(&escaped);
    if print_as_multiple_lines || force_trailing_newline {
        result.push('\n');
    }
    format!("\"\"\"{result}\"\"\"")
}

/// Split on `\r\n`, `\n`, or `\r`, matching the GraphQL block-string split.
fn split_string_lines(s: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let bytes = s.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                lines.push(&s[start..i]);
                if bytes.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
                i += 1;
                start = i;
            }
            b'\n' => {
                lines.push(&s[start..i]);
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    lines.push(&s[start..]);
    lines
}

fn starts_with_space_or_tab(line: &str) -> bool {
    matches!(line.as_bytes().first(), Some(b' ' | b'\t'))
}

fn first_is_space_or_tab(value: &str) -> bool {
    matches!(value.as_bytes().first(), Some(b' ' | b'\t'))
}

/// Build a field or spread head, wrapping arguments onto multiple lines when the
/// single-line form exceeds the max width.
fn wrapped_line_and_args(prefix: &str, args: &[String]) -> String {
    let one_line = format!("{prefix}{}", wrap("(", &args.join(", "), ")"));
    if utf16_len(&one_line) > MAX_LINE_LENGTH {
        format!("{prefix}{}", wrap("(\n", &indent(&args.join("\n")), "\n)"))
    } else {
        one_line
    }
}

/// Count UTF-16 code units, matching the canonical GraphQL printer's `String.length`.
fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// Join parts with a separator, dropping empty parts. Mirrors the canonical GraphQL
/// `join`, which filters out empty and undefined entries.
fn join_filtered<S: AsRef<str>>(parts: &[S], separator: &str) -> String {
    parts
        .iter()
        .map(AsRef::as_ref)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(separator)
}

/// Wrap `body` with `start` and `end` when it is non-empty, else return empty.
fn wrap(start: &str, body: &str, end: &str) -> String {
    if body.is_empty() {
        String::new()
    } else {
        format!("{start}{body}{end}")
    }
}

/// Render a brace block with each entry on its own indented line. An empty block
/// returns an empty string, matching the canonical `block` rule.
fn block(parts: &[String]) -> String {
    let body = join_filtered(parts, "\n");
    wrap("{\n", &indent(&body), "\n}")
}

/// Indent every line of `s` by two spaces. An empty string stays empty.
fn indent(s: &str) -> String {
    if s.is_empty() {
        String::new()
    } else {
        format!("  {}", s.replace('\n', "\n  "))
    }
}
