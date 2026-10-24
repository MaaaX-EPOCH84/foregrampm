// ForegramPM — canonical form of a source file: the document written back.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Write;

use crate::model::{Comment, Directive, Document, Element, Item, Value};

/// What a line of the file is, for the blank line between two of them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// A directive, an attribute, a content line, or comments on their own.
    Line,
    /// An element, with its block.
    Block,
}

impl Document {
    /// Writes the document in its canonical form, described in
    /// `docs/source-format.md`: one item per line, two spaces per level,
    /// one space between tokens, `LF`, every comment written back where it
    /// stands, the author's order untouched, a label written only when it
    /// differs from the identifier, strings escaped with the shortest
    /// escape. Reading the text back gives the same document, positions
    /// apart; writing it again gives the same text.
    ///
    /// The writer trusts the document: one built by hand that breaks the
    /// rules of the model — a kind with a blank, an identifier that is not
    /// one — writes a file the reader refuses. A value that cannot stand as
    /// a word is written as a string.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        // The comments before the first item of the file are its preamble,
        // wherever a document built by hand put them: the reader cannot tell
        // the two apart, so the writer does not either.
        let first_before = self
            .directives
            .first()
            .map(|d| &d.before[..])
            .or_else(|| self.elements.first().map(|e| &e.before[..]))
            .unwrap_or(&[]);
        let rest = !self.directives.is_empty() || !self.elements.is_empty() || !self.end.is_empty();
        comments(&mut out, &self.preamble, 0);
        comments(&mut out, first_before, 0);
        if (!self.preamble.is_empty() || !first_before.is_empty()) && rest {
            out.push('\n');
        }
        let mut previous: Option<Entry> = None;
        for (i, directive) in self.directives.iter().enumerate() {
            separate(&mut out, &mut previous, Entry::Line);
            if i > 0 {
                comments(&mut out, &directive.before, 0);
            }
            attribute(&mut out, directive, 0);
        }
        for (i, element_) in self.elements.iter().enumerate() {
            separate(&mut out, &mut previous, Entry::Block);
            if i > 0 || !self.directives.is_empty() {
                comments(&mut out, &element_.before, 0);
            }
            element(&mut out, element_, 0);
        }
        if !self.end.is_empty() {
            separate(&mut out, &mut previous, Entry::Line);
            comments(&mut out, &self.end, 0);
        }
        out
    }
}

/// The blank line between two entries: written when at least one of them
/// is a block.
fn separate(out: &mut String, previous: &mut Option<Entry>, entry: Entry) {
    if let Some(before) = *previous
        && (before == Entry::Block || entry == Entry::Block)
    {
        out.push('\n');
    }
    *previous = Some(entry);
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// Comments on their own lines.
fn comments(out: &mut String, comments: &[Comment], depth: usize) {
    for comment in comments {
        indent(out, depth);
        out.push_str("//");
        out.push_str(&comment.text);
        out.push('\n');
    }
}

/// The end of a line: its comment if any, then the line break.
fn end_line(out: &mut String, after: Option<&Comment>) {
    if let Some(comment) = after {
        out.push_str(" //");
        out.push_str(&comment.text);
    }
    out.push('\n');
}

/// A directive or an attribute: `@key`, its values, its comment.
fn attribute(out: &mut String, directive: &Directive, depth: usize) {
    indent(out, depth);
    out.push('@');
    out.push_str(&directive.key);
    for value_ in &directive.values {
        out.push(' ');
        value(out, value_);
    }
    end_line(out, directive.after.as_ref());
}

/// An element, its comments before it already written.
fn element(out: &mut String, element: &Element, depth: usize) {
    indent(out, depth);
    out.push_str(&element.kind);
    out.push(' ');
    out.push_str(&element.id);
    if let Some(label) = &element.label
        && label.text != element.id
    {
        out.push(' ');
        string(out, &label.text);
    }
    if element.items.is_empty() && element.end.is_empty() && element.after_open.is_none() {
        out.push_str(" {}");
        end_line(out, element.after_close.as_ref());
        return;
    }
    out.push_str(" {");
    end_line(out, element.after_open.as_ref());
    let mut previous: Option<Entry> = None;
    for item in &element.items {
        match item {
            Item::Element(child) => {
                separate(out, &mut previous, Entry::Block);
                comments(out, &child.before, depth + 1);
                self::element(out, child, depth + 1);
            }
            Item::Attribute(directive) => {
                separate(out, &mut previous, Entry::Line);
                comments(out, &directive.before, depth + 1);
                attribute(out, directive, depth + 1);
            }
            Item::Content(content) => {
                separate(out, &mut previous, Entry::Line);
                comments(out, &content.before, depth + 1);
                indent(out, depth + 1);
                for (i, value_) in content.values.iter().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    value(out, value_);
                }
                end_line(out, content.after.as_ref());
            }
        }
    }
    if !element.end.is_empty() {
        separate(out, &mut previous, Entry::Line);
        comments(out, &element.end, depth + 1);
    }
    indent(out, depth);
    out.push('}');
    end_line(out, element.after_close.as_ref());
}

/// A value as written: a word, or a string when it was one or cannot be a
/// word.
fn value(out: &mut String, value: &Value) {
    if value.quoted || !is_word(&value.text) {
        string(out, &value.text);
    } else {
        out.push_str(&value.text);
    }
}

/// Whether a text reads back as one word: not empty, no blank, no
/// punctuation, no quote, no control character, and not opening a comment.
fn is_word(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with("//")
        && !text
            .chars()
            .any(|c| matches!(c, ' ' | '\t' | '{' | '}' | ';' | '"') || c.is_control())
}

/// A string between quotes, with the shortest escape for each character:
/// `\"`, `\\`, `\n`, `\t`, `\u{…}` for the other control characters, and
/// every other character as itself.
fn string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                // Writing to a `String` cannot fail.
                let _ = write!(out, "\\u{{{:X}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_is_whatever_the_lexer_would_read_as_one() {
        for word in ["a", "0,n", "#id", "12.5", "http://x/y", "é", "a/b"] {
            assert!(is_word(word), "{word}");
        }
        for text in [
            "", "a b", "a\tb", "{", "}", ";", "\"", "//c", "a\nb", "\u{7}",
        ] {
            assert!(!is_word(text), "{text:?}");
        }
    }

    #[test]
    fn a_string_takes_the_shortest_escape() {
        let mut out = String::new();
        string(&mut out, "q\" b\\ n\n t\t \u{7} \u{1F} é\u{1F600}");
        assert_eq!(out, "\"q\\\" b\\\\ n\\n t\\t \\u{7} \\u{1F} é\u{1F600}\"");
    }
}
