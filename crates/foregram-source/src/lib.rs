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
//! [`Document::to_text`] writes a document back in its canonical form, the
//! one fixed form of a file: reading it again gives the same document, and
//! writing it again gives the same bytes. Comments are attached to the item
//! they stand with, so that nothing of the file is lost.
//!
//! Like the rest of the product library, this crate takes bytes and returns
//! values: no file, clock or environment.
//!
//! # Public surface
//!
//! This is the contract frozen for the callers of the library, from version
//! 0.1.0 on. It is held by `tests/api.rs`, which lists every exported name:
//! adding one means adding it there.
//!
//! - **Model**: [`Document`], [`Directive`], [`Element`], [`Item`],
//!   [`Content`], [`Value`], [`Comment`]. Their fields are public, so that a
//!   caller — an editor, a test — can build or rewrite a document by hand;
//!   adding a field is therefore a change of the contract.
//! - **Reading and writing**: `Document::parse`, which returns the document
//!   or every refusal of the file, never a partial document;
//!   `Document::to_text`, the canonical form; the limit [`NESTING_MAX`].
//! - **Identifiers**: [`Index`], built over a document, which finds an
//!   element by identifier or resolves a value into one.
//! - **Refusals**: the `Diagnostic`, `Pos` and catalogues of
//!   `foregram-graph`, family `T`.

#![deny(missing_docs)]

mod canon;
mod index;
mod lex;
mod model;
mod parse;

pub use index::Index;
pub use model::{Comment, Content, Directive, Document, Element, Item, Value};
pub use parse::NESTING_MAX;
