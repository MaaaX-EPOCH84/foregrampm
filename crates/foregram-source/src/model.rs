// ForegramPM — document model of a source file: what the reader returns.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use foregram_graph::Pos;

/// A well-formed source file: its header, its elements, and its comments,
/// each attached to the item it stands with.
///
/// The notation reads the directives and the elements; the comments are
/// there for the canonical form, which writes them back where they were.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    /// The comments that open the file, before its first directive or
    /// element — which therefore has none before it of its own.
    pub preamble: Vec<Comment>,
    /// The directives of the header, before the first element, in file order.
    pub directives: Vec<Directive>,
    /// The top-level elements, in file order.
    pub elements: Vec<Element>,
    /// The comments on their own line after the last directive or element
    /// of the file.
    pub end: Vec<Comment>,
}

/// A directive of the header or an attribute of an element: `@`, a key,
/// then values, all opaque to the format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Directive {
    /// The key, without its `@`.
    pub key: String,
    /// The values after the key, possibly none.
    pub values: Vec<Value>,
    /// Position of the `@`.
    pub pos: Pos,
    /// The comments on their own line just before.
    pub before: Vec<Comment>,
    /// The comment at the end of the same line.
    pub after: Option<Comment>,
}

/// An element: a kind, an identifier, an optional label, and the items of
/// its block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    /// The kind word, meaningful to the notation only.
    pub kind: String,
    /// The identifier, unique in the whole file.
    pub id: String,
    /// The label, a decoded string, when the author wrote one.
    pub label: Option<Value>,
    /// The items of the block, in file order.
    pub items: Vec<Item>,
    /// Position of the kind, the first character of the element.
    pub pos: Pos,
    /// Position of the identifier.
    pub id_pos: Pos,
    /// Position of the `}` that closes the block.
    pub close: Pos,
    /// The comments on their own line just before the head.
    pub before: Vec<Comment>,
    /// The comment at the end of the head line, after the `{`.
    pub after_open: Option<Comment>,
    /// The comment at the end of the line of the `}`.
    pub after_close: Option<Comment>,
    /// The comments on their own line after the last item of the block.
    pub end: Vec<Comment>,
}

/// What a block holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    /// A child element, which makes its parent a container.
    Element(Element),
    /// An attribute, `@key values`, read by the format or the layout.
    Attribute(Directive),
    /// A content line, read by the notation.
    Content(Content),
}

/// A content line: one or more values that the notation reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Content {
    /// The values, in order; never empty.
    pub values: Vec<Value>,
    /// Position of the first value.
    pub pos: Pos,
    /// The comments on their own line just before.
    pub before: Vec<Comment>,
    /// The comment at the end of the same line.
    pub after: Option<Comment>,
}

/// A word or a string, kept as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Value {
    /// The text: the word as written, or the string decoded, quotes removed.
    pub text: String,
    /// Whether the value was written as a string, between quotes.
    pub quoted: bool,
    /// Position of its first character, the opening quote for a string.
    pub pos: Pos,
}

/// A comment, `//` to the end of its line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    /// The text after `//`, line end and trailing blanks excluded.
    pub text: String,
    /// Position of the first `/`.
    pub pos: Pos,
}
