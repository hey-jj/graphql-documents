//! Parser for GraphQL executable documents.
//!
//! This follows the GraphQL grammar for the executable subset. It tokenizes
//! the source then builds the AST. Comments, commas, and insignificant
//! whitespace are ignored, matching the GraphQL spec.

use crate::ast::*;

/// A parse failure with a message and byte offset into the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable description of the failure.
    pub message: String,
    /// Byte offset where the failure was detected.
    pub position: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at position {}", self.message, self.position)
    }
}

impl std::error::Error for ParseError {}

/// Parse a GraphQL executable document from source text.
///
/// # Errors
///
/// Returns a [`ParseError`] if the source is not a valid executable document.
pub fn parse(source: &str) -> Result<Document, ParseError> {
    let mut parser = Parser::new(source);
    let document = parser.parse_document()?;
    Ok(document)
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Parser {
            input: source.as_bytes(),
            pos: 0,
        }
    }

    fn err<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            message: message.into(),
            position: self.pos,
        })
    }

    // --- lexical helpers ---

    /// Skip whitespace, line terminators, commas, BOM, and comments.
    fn skip_ignored(&mut self) {
        loop {
            match self.peek_byte() {
                Some(b' ' | b'\t' | b'\n' | b'\r' | b',') => self.pos += 1,
                Some(b'#') => {
                    while let Some(c) = self.peek_byte() {
                        if c == b'\n' || c == b'\r' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                Some(0xEF) if self.input[self.pos..].starts_with(&[0xEF, 0xBB, 0xBF]) => {
                    self.pos += 3;
                }
                _ => break,
            }
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ignored();
        self.peek_byte()
    }

    /// Consume `expected` after skipping ignored tokens, or error.
    fn expect_byte(&mut self, expected: u8) -> Result<(), ParseError> {
        self.skip_ignored();
        if self.peek_byte() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            self.err(format!("expected '{}'", expected as char))
        }
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        self.skip_ignored();
        if self.peek_byte() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn is_name_start(c: u8) -> bool {
        c == b'_' || c.is_ascii_alphabetic()
    }

    fn is_name_continue(c: u8) -> bool {
        c == b'_' || c.is_ascii_alphanumeric()
    }

    /// Read a Name token: `/[_A-Za-z][_0-9A-Za-z]*/`.
    fn parse_name(&mut self) -> Result<String, ParseError> {
        self.skip_ignored();
        let start = self.pos;
        match self.peek_byte() {
            Some(c) if Self::is_name_start(c) => self.pos += 1,
            _ => return self.err("expected a name"),
        }
        while let Some(c) = self.peek_byte() {
            if Self::is_name_continue(c) {
                self.pos += 1;
            } else {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&self.input[start..self.pos]).into_owned())
    }

    /// Match an exact keyword without consuming a longer name.
    fn peek_keyword(&mut self, keyword: &str) -> bool {
        self.skip_ignored();
        let kw = keyword.as_bytes();
        if !self.input[self.pos..].starts_with(kw) {
            return false;
        }
        // The next byte must not continue the name, or this is a longer token.
        !matches!(self.input.get(self.pos + kw.len()), Some(&c) if Self::is_name_continue(c))
    }

    // --- document ---

    fn parse_document(&mut self) -> Result<Document, ParseError> {
        let mut definitions = Vec::new();
        loop {
            self.skip_ignored();
            if self.peek_byte().is_none() {
                break;
            }
            definitions.push(self.parse_definition()?);
        }
        if definitions.is_empty() {
            return self.err("a document must define at least one definition");
        }
        Ok(Document { definitions })
    }

    fn parse_definition(&mut self) -> Result<Definition, ParseError> {
        if self.peek() == Some(b'{') {
            // Anonymous query shorthand.
            let selection_set = self.parse_selection_set()?;
            return Ok(Definition::Operation(OperationDefinition {
                operation: OperationType::Query,
                name: None,
                variable_definitions: Vec::new(),
                directives: Vec::new(),
                selection_set,
            }));
        }
        if self.peek_keyword("fragment") {
            return Ok(Definition::Fragment(self.parse_fragment_definition()?));
        }
        if self.peek_keyword("query")
            || self.peek_keyword("mutation")
            || self.peek_keyword("subscription")
        {
            return Ok(Definition::Operation(self.parse_operation_definition()?));
        }
        self.err("expected a definition")
    }

    fn parse_operation_definition(&mut self) -> Result<OperationDefinition, ParseError> {
        let operation = if self.peek_keyword("query") {
            self.parse_name()?;
            OperationType::Query
        } else if self.peek_keyword("mutation") {
            self.parse_name()?;
            OperationType::Mutation
        } else {
            self.parse_name()?;
            OperationType::Subscription
        };
        let name = if matches!(self.peek(), Some(c) if Self::is_name_start(c)) {
            Some(self.parse_name()?)
        } else {
            None
        };
        let variable_definitions = self.parse_variable_definitions()?;
        let directives = self.parse_directives()?;
        let selection_set = self.parse_selection_set()?;
        Ok(OperationDefinition {
            operation,
            name,
            variable_definitions,
            directives,
            selection_set,
        })
    }

    fn parse_fragment_definition(&mut self) -> Result<FragmentDefinition, ParseError> {
        self.parse_name()?; // "fragment"
        let name = self.parse_name()?;
        if name == "on" {
            return self.err("a fragment cannot be named 'on'");
        }
        let variable_definitions = self.parse_variable_definitions()?;
        if !self.peek_keyword("on") {
            return self.err("expected 'on' in fragment definition");
        }
        self.parse_name()?; // "on"
        let type_condition = self.parse_name()?;
        let directives = self.parse_directives()?;
        let selection_set = self.parse_selection_set()?;
        Ok(FragmentDefinition {
            name,
            variable_definitions,
            type_condition,
            directives,
            selection_set,
        })
    }

    fn parse_variable_definitions(&mut self) -> Result<Vec<VariableDefinition>, ParseError> {
        let mut defs = Vec::new();
        if !self.consume_byte(b'(') {
            return Ok(defs);
        }
        while self.peek() != Some(b')') {
            self.expect_byte(b'$')?;
            let variable = self.parse_name()?;
            self.expect_byte(b':')?;
            let var_type = self.parse_type()?;
            let default_value = if self.consume_byte(b'=') {
                Some(self.parse_value(true)?)
            } else {
                None
            };
            let directives = self.parse_directives()?;
            defs.push(VariableDefinition {
                variable,
                var_type,
                default_value,
                directives,
            });
            if self.peek_byte().is_none() {
                return self.err("unterminated variable definitions");
            }
        }
        self.expect_byte(b')')?;
        Ok(defs)
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let mut base = if self.consume_byte(b'[') {
            let inner = self.parse_type()?;
            self.expect_byte(b']')?;
            Type::List(Box::new(inner))
        } else {
            Type::Named(self.parse_name()?)
        };
        if self.consume_byte(b'!') {
            base = Type::NonNull(Box::new(base));
        }
        Ok(base)
    }

    fn parse_directives(&mut self) -> Result<Vec<Directive>, ParseError> {
        let mut directives = Vec::new();
        while self.peek() == Some(b'@') {
            self.pos += 1;
            let name = self.parse_name()?;
            let arguments = self.parse_arguments()?;
            directives.push(Directive { name, arguments });
        }
        Ok(directives)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Argument>, ParseError> {
        let mut args = Vec::new();
        if !self.consume_byte(b'(') {
            return Ok(args);
        }
        while self.peek() != Some(b')') {
            let name = self.parse_name()?;
            self.expect_byte(b':')?;
            let value = self.parse_value(false)?;
            args.push(Argument { name, value });
            if self.peek_byte().is_none() {
                return self.err("unterminated arguments");
            }
        }
        self.expect_byte(b')')?;
        Ok(args)
    }

    fn parse_selection_set(&mut self) -> Result<SelectionSet, ParseError> {
        self.expect_byte(b'{')?;
        let mut selections = Vec::new();
        while self.peek() != Some(b'}') {
            if self.peek_byte().is_none() {
                return self.err("unterminated selection set");
            }
            selections.push(self.parse_selection()?);
        }
        self.expect_byte(b'}')?;
        Ok(SelectionSet { selections })
    }

    fn parse_selection(&mut self) -> Result<Selection, ParseError> {
        if self.peek() == Some(b'.') {
            // Spread: "...".
            self.expect_byte(b'.')?;
            self.expect_byte(b'.')?;
            self.expect_byte(b'.')?;
            // Inline fragment if "on", a directive, or a selection set follows.
            if self.peek_keyword("on") {
                self.parse_name()?; // "on"
                let type_condition = Some(self.parse_name()?);
                let directives = self.parse_directives()?;
                let selection_set = self.parse_selection_set()?;
                return Ok(Selection::InlineFragment(InlineFragment {
                    type_condition,
                    directives,
                    selection_set,
                }));
            }
            match self.peek() {
                Some(b'@') | Some(b'{') => {
                    let directives = self.parse_directives()?;
                    let selection_set = self.parse_selection_set()?;
                    Ok(Selection::InlineFragment(InlineFragment {
                        type_condition: None,
                        directives,
                        selection_set,
                    }))
                }
                _ => {
                    let name = self.parse_name()?;
                    let directives = self.parse_directives()?;
                    Ok(Selection::FragmentSpread(FragmentSpread {
                        name,
                        directives,
                    }))
                }
            }
        } else {
            self.parse_field()
        }
    }

    fn parse_field(&mut self) -> Result<Selection, ParseError> {
        let first = self.parse_name()?;
        let (alias, name) = if self.consume_byte(b':') {
            (Some(first), self.parse_name()?)
        } else {
            (None, first)
        };
        let arguments = self.parse_arguments()?;
        let directives = self.parse_directives()?;
        let selection_set = if self.peek() == Some(b'{') {
            Some(self.parse_selection_set()?)
        } else {
            None
        };
        Ok(Selection::Field(Field {
            alias,
            name,
            arguments,
            directives,
            selection_set,
        }))
    }

    // --- values ---

    fn parse_value(&mut self, is_const: bool) -> Result<Value, ParseError> {
        self.skip_ignored();
        match self.peek_byte() {
            Some(b'$') => {
                if is_const {
                    return self.err("a constant value cannot be a variable");
                }
                self.pos += 1;
                Ok(Value::Variable(self.parse_name()?))
            }
            Some(b'[') => {
                self.pos += 1;
                let mut items = Vec::new();
                while self.peek() != Some(b']') {
                    if self.peek_byte().is_none() {
                        return self.err("unterminated list value");
                    }
                    items.push(self.parse_value(is_const)?);
                }
                self.expect_byte(b']')?;
                Ok(Value::List(items))
            }
            Some(b'{') => {
                self.pos += 1;
                let mut fields = Vec::new();
                while self.peek() != Some(b'}') {
                    if self.peek_byte().is_none() {
                        return self.err("unterminated object value");
                    }
                    let name = self.parse_name()?;
                    self.expect_byte(b':')?;
                    let value = self.parse_value(is_const)?;
                    fields.push(ObjectField { name, value });
                }
                self.expect_byte(b'}')?;
                Ok(Value::Object(fields))
            }
            Some(b'"') => self.parse_string(),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) if Self::is_name_start(c) => {
                let name = self.parse_name()?;
                match name.as_str() {
                    "true" => Ok(Value::Boolean(true)),
                    "false" => Ok(Value::Boolean(false)),
                    "null" => Ok(Value::Null),
                    _ => Ok(Value::Enum(name)),
                }
            }
            _ => self.err("expected a value"),
        }
    }

    fn parse_number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        let mut is_float = false;
        if self.peek_byte() == Some(b'-') {
            self.pos += 1;
        }
        if self.peek_byte() == Some(b'0') {
            self.pos += 1;
        } else {
            if !matches!(self.peek_byte(), Some(c) if c.is_ascii_digit()) {
                return self.err("invalid number");
            }
            while matches!(self.peek_byte(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if self.peek_byte() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            if !matches!(self.peek_byte(), Some(c) if c.is_ascii_digit()) {
                return self.err("invalid float");
            }
            while matches!(self.peek_byte(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek_byte(), Some(c) if c.is_ascii_digit()) {
                return self.err("invalid exponent");
            }
            while matches!(self.peek_byte(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = String::from_utf8_lossy(&self.input[start..self.pos]).into_owned();
        if is_float {
            Ok(Value::Float(text))
        } else {
            Ok(Value::Int(text))
        }
    }

    fn parse_string(&mut self) -> Result<Value, ParseError> {
        if self.input[self.pos..].starts_with(b"\"\"\"") {
            return self.parse_block_string();
        }
        self.pos += 1; // opening quote
        let mut value = String::new();
        loop {
            match self.peek_byte() {
                None => return self.err("unterminated string"),
                Some(b'"') => {
                    self.pos += 1;
                    break;
                }
                Some(b'\n' | b'\r') => return self.err("unterminated string"),
                Some(b'\\') => {
                    self.pos += 1;
                    self.parse_escape(&mut value)?;
                }
                Some(_) => {
                    let ch = self.next_char()?;
                    value.push(ch);
                }
            }
        }
        Ok(Value::String {
            value,
            block: false,
        })
    }

    fn parse_escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        match self.peek_byte() {
            Some(b'"') => {
                out.push('"');
                self.pos += 1;
            }
            Some(b'\\') => {
                out.push('\\');
                self.pos += 1;
            }
            Some(b'/') => {
                out.push('/');
                self.pos += 1;
            }
            Some(b'b') => {
                out.push('\u{8}');
                self.pos += 1;
            }
            Some(b'f') => {
                out.push('\u{c}');
                self.pos += 1;
            }
            Some(b'n') => {
                out.push('\n');
                self.pos += 1;
            }
            Some(b'r') => {
                out.push('\r');
                self.pos += 1;
            }
            Some(b't') => {
                out.push('\t');
                self.pos += 1;
            }
            Some(b'u') => {
                self.pos += 1;
                self.parse_unicode_escape(out)?;
            }
            _ => return self.err("invalid escape sequence"),
        }
        Ok(())
    }

    fn parse_unicode_escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        // Braced form: \u{XXXX}.
        if self.peek_byte() == Some(b'{') {
            self.pos += 1;
            let start = self.pos;
            while matches!(self.peek_byte(), Some(c) if c.is_ascii_hexdigit()) {
                self.pos += 1;
            }
            let hex = std::str::from_utf8(&self.input[start..self.pos]).unwrap_or("");
            if self.peek_byte() != Some(b'}') || hex.is_empty() {
                return self.err("invalid unicode escape");
            }
            self.pos += 1;
            let code = u32::from_str_radix(hex, 16).map_err(|_| ParseError {
                message: "invalid unicode escape".into(),
                position: self.pos,
            })?;
            match char::from_u32(code) {
                Some(c) => out.push(c),
                None => return self.err("invalid unicode code point"),
            }
            return Ok(());
        }
        // Fixed four-hex-digit form, with surrogate pair support.
        let high = self.read_four_hex()?;
        if (0xD800..=0xDBFF).contains(&high) && self.input[self.pos..].starts_with(b"\\u") {
            self.pos += 2;
            let low = self.read_four_hex()?;
            if (0xDC00..=0xDFFF).contains(&low) {
                let code = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
                match char::from_u32(code) {
                    Some(c) => out.push(c),
                    None => return self.err("invalid surrogate pair"),
                }
                return Ok(());
            }
            return self.err("invalid low surrogate");
        }
        match char::from_u32(high) {
            Some(c) => out.push(c),
            None => return self.err("invalid unicode code point"),
        }
        Ok(())
    }

    fn read_four_hex(&mut self) -> Result<u32, ParseError> {
        if self.pos + 4 > self.input.len() {
            return self.err("invalid unicode escape");
        }
        let slice = &self.input[self.pos..self.pos + 4];
        let hex = std::str::from_utf8(slice).map_err(|_| ParseError {
            message: "invalid unicode escape".into(),
            position: self.pos,
        })?;
        let code = u32::from_str_radix(hex, 16).map_err(|_| ParseError {
            message: "invalid unicode escape".into(),
            position: self.pos,
        })?;
        self.pos += 4;
        Ok(code)
    }

    fn parse_block_string(&mut self) -> Result<Value, ParseError> {
        self.pos += 3; // opening """
        let mut raw = String::new();
        loop {
            if self.input[self.pos..].starts_with(b"\\\"\"\"") {
                raw.push_str("\"\"\"");
                self.pos += 4;
                continue;
            }
            if self.input[self.pos..].starts_with(b"\"\"\"") {
                self.pos += 3;
                break;
            }
            match self.peek_byte() {
                None => return self.err("unterminated block string"),
                Some(_) => {
                    let ch = self.next_char()?;
                    raw.push(ch);
                }
            }
        }
        let value = dedent_block_string(&raw);
        Ok(Value::String { value, block: true })
    }

    /// Decode the next UTF-8 character and advance.
    fn next_char(&mut self) -> Result<char, ParseError> {
        let rest = &self.input[self.pos..];
        let s = match std::str::from_utf8(rest) {
            Ok(s) => s,
            Err(e) if e.valid_up_to() > 0 => std::str::from_utf8(&rest[..e.valid_up_to()]).unwrap(),
            Err(_) => return self.err("invalid UTF-8 in source"),
        };
        match s.chars().next() {
            Some(c) => {
                self.pos += c.len_utf8();
                Ok(c)
            }
            None => self.err("unexpected end of input"),
        }
    }
}

/// Apply the GraphQL block-string dedent algorithm from the specification.
///
/// Common leading whitespace is stripped from every line after the first, and
/// leading and trailing blank lines are removed.
fn dedent_block_string(raw: &str) -> String {
    let lines: Vec<&str> = split_lines(raw);

    let mut common_indent: Option<usize> = None;
    for line in lines.iter().skip(1) {
        let indent = leading_whitespace(line);
        if indent < line.len() {
            match common_indent {
                Some(c) if indent < c => common_indent = Some(indent),
                None => common_indent = Some(indent),
                _ => {}
            }
        }
    }

    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            result.push((*line).to_string());
        } else {
            let indent = common_indent.unwrap_or(0).min(line.len());
            result.push(line[indent..].to_string());
        }
    }

    while result.first().is_some_and(|l| is_blank(l)) {
        result.remove(0);
    }
    while result.last().is_some_and(|l| is_blank(l)) {
        result.pop();
    }

    result.join("\n")
}

fn split_lines(s: &str) -> Vec<&str> {
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

fn leading_whitespace(line: &str) -> usize {
    line.bytes()
        .take_while(|&b| b == b' ' || b == b'\t')
        .count()
}

fn is_blank(line: &str) -> bool {
    line.bytes().all(|b| b == b' ' || b == b'\t')
}
