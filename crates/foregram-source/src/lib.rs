// ForegramPM — reader of the source format of diagrams.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads the text file a diagram is written in, described in
//! `docs/source-format.md`, into a [`Document`]: directives, elements with
//! their items, and comments kept aside.
//!
//! The reader is written by hand, a lexer then a recursive descent. Every
//! node carries the line and column of its first character, the column in
//! code points. Every refusal of a file is collected and returned sorted by
//! position, with a stable code of the family `T` and a message composed
//! from the catalogues of `foregram-graph`. The reader never panics, and its
//! recursion is bounded by [`NESTING_MAX`].
//!
//! The format knows no notation: any kind word parses, and the content
//! lines are kept as values for the notation to read. What the format does
//! own is the identifier: unique in the file, found again through an
//! [`Index`], which resolves a reference or refuses an unknown one.
//!
//! Like the rest of the product library, this crate takes bytes and returns
//! values: no file, clock or environment.

#![deny(missing_docs)]

mod index;
mod lex;
mod model;
mod parse;

pub use index::Index;
pub use model::{Comment, Content, Directive, Document, Element, Item, Value};
pub use parse::NESTING_MAX;
