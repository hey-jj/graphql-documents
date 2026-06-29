//! Whitespace normalization.
//!
//! Collapse each run of whitespace into a single ASCII space, then trim. This
//! matches the JavaScript `str.replace(/\s+/g, ' ').trim()`. The whitespace set
//! is the ECMAScript `\s` class, which is wider than the ASCII whitespace the
//! GraphQL printer emits. Matching the full set keeps the result identical for
//! any input, including non-ASCII spaces inside string values.

/// Test membership in the ECMAScript `RegExp` `\s` class.
///
/// The set is `WhiteSpace` plus `LineTerminator`: tab, line feed, vertical tab,
/// form feed, carriage return, space, no-break space, and the listed Unicode
/// space separators, line/paragraph separators, and the byte-order mark.
fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}' // tab
        | '\u{000A}' // line feed
        | '\u{000B}' // vertical tab
        | '\u{000C}' // form feed
        | '\u{000D}' // carriage return
        | '\u{0020}' // space
        | '\u{00A0}' // no-break space
        | '\u{1680}'
        | '\u{2000}'
            ..='\u{200A}'
        | '\u{2028}' // line separator
        | '\u{2029}' // paragraph separator
        | '\u{202F}'
        | '\u{205F}'
        | '\u{3000}'
        | '\u{FEFF}' // zero-width no-break space / BOM
    )
}

/// Collapse whitespace runs to single spaces and trim the ends.
pub fn normalize_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_run = false;
    for ch in input.chars() {
        if is_js_whitespace(ch) {
            in_run = true;
        } else {
            if in_run && !out.is_empty() {
                out.push(' ');
            }
            in_run = false;
            out.push(ch);
        }
    }
    // A trailing whitespace run leaves nothing to push, which is the trim.
    out
}
