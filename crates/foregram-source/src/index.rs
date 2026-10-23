// ForegramPM — lookup of the elements of a document by identifier.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use foregram_graph::{Diagnostic, Message};

use crate::model::{Document, Element, Item, Value};

/// Every element of a document by its identifier, nesting included.
///
/// This is the lookup the format provides: a notation resolves the words
/// of its content lines with it, an agent or an editor finds the element
/// it targets. The index borrows the document, so it cannot go stale: the
/// document stays as it was until the index is dropped.
///
/// Which words name other elements is the notation's business; the index
/// answers for any text it is given, a word or a string alike, and knows
/// nothing of the element the text comes from — an element may name
/// itself, and the notation judges.
#[derive(Clone, Debug)]
pub struct Index<'a> {
    elements: BTreeMap<&'a str, &'a Element>,
}

impl<'a> Index<'a> {
    /// Indexes every element of the document, at any depth.
    ///
    /// A document read by [`Document::parse`] has no duplicate identifier.
    /// Should one have been built or altered by hand, the first element in
    /// document order keeps the identifier.
    pub fn build(document: &'a Document) -> Index<'a> {
        let mut elements = BTreeMap::new();
        for element in &document.elements {
            walk(element, &mut elements);
        }
        Index { elements }
    }

    /// The element with this identifier, if any. Case matters.
    pub fn get(&self, id: &str) -> Option<&'a Element> {
        self.elements.get(id).copied()
    }

    /// The element a value names, or the refusal `T015` at the value.
    ///
    /// A reference may precede the definition it points to: the index is
    /// built once the whole file is read.
    pub fn resolve(&self, value: &Value) -> Result<&'a Element, Diagnostic> {
        self.get(&value.text)
            .ok_or_else(|| Diagnostic::new(value.pos, Message::new("T015").text("id", &value.text)))
    }

    /// The identifiers and their elements, in identifier order.
    pub fn iter(&self) -> impl Iterator<Item = (&'a str, &'a Element)> + '_ {
        self.elements.iter().map(|(id, element)| (*id, *element))
    }

    /// The number of elements indexed.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether the document has no element.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// Adds an element and its descendants, in document order.
fn walk<'a>(element: &'a Element, into: &mut BTreeMap<&'a str, &'a Element>) {
    into.entry(element.id.as_str()).or_insert(element);
    for item in &element.items {
        if let Item::Element(child) = item {
            walk(child, into);
        }
    }
}
