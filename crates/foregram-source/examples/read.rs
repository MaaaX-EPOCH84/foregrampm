// ForegramPM — example: read a source file, resolve its references, rewrite it.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! The whole library in one pass: bytes in, a document or refusals out, then
//! an index over it and the canonical form back. The file is embedded at
//! compile time; a real caller reads it itself, the library never does.
//!
//! Which words are references is the notation's business. For the sake of
//! the example, a content line of two values in an `association` is read as
//! a Merise link: an element and its cardinality.

use foregram_graph::{Catalog, file_report};
use foregram_source::{Document, Index, Item};

const NAME: &str = "gestpret.fg";
const SOURCE: &[u8] = include_bytes!("../../../corpus/mcd/gestpret.fg");

fn main() {
    let catalog = Catalog::english();

    let document = match Document::parse(SOURCE) {
        Ok(document) => document,
        Err(refusals) => {
            for line in file_report(NAME, &refusals, catalog) {
                eprintln!("{line}");
            }
            std::process::exit(1);
        }
    };

    let index = Index::build(&document);
    let mut refusals = Vec::new();
    for element in document.elements.iter().filter(|e| e.kind == "association") {
        for item in &element.items {
            let Item::Content(line) = item else { continue };
            let [target, cardinality] = line.values.as_slice() else {
                continue;
            };
            match index.resolve(target) {
                Ok(entity) => println!(
                    "{} -> {} {} ({})",
                    element.id, entity.id, cardinality.text, entity.kind
                ),
                Err(refusal) => refusals.push(refusal),
            }
        }
    }
    if !refusals.is_empty() {
        for line in file_report(NAME, &refusals, catalog) {
            eprintln!("{line}");
        }
        std::process::exit(1);
    }

    // The corpus files are written in canonical form: rewriting gives the
    // same bytes.
    let canonical = document.to_text();
    let same = canonical.as_bytes() == SOURCE;
    println!(
        "{NAME}: {} elements, canonical form {}",
        index.len(),
        if same { "unchanged" } else { "differs" }
    );
}
