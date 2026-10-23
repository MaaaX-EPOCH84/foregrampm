// ForegramPM — tests of the lookup by identifier: index, references, refusal.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop short: the product rules do not apply here.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use foregram_graph::{Catalog, Pos};
use foregram_source::{Content, Document, Element, Index, Item, Value};

fn read(text: &str) -> Document {
    Document::parse(text.as_bytes()).unwrap_or_else(|diags| {
        let lines: Vec<String> = diags.iter().map(ToString::to_string).collect();
        panic!("refused:\n{}", lines.join("\n"))
    })
}

fn corpus(name: &str) -> String {
    let path = format!("{}/../../corpus/mcd/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Every identifier of the document, in document order, at any depth.
fn ids(document: &Document) -> Vec<&str> {
    fn walk<'a>(element: &'a Element, out: &mut Vec<&'a str>) {
        out.push(&element.id);
        for item in &element.items {
            if let Item::Element(child) = item {
                walk(child, out);
            }
        }
    }
    let mut out = Vec::new();
    for element in &document.elements {
        walk(element, &mut out);
    }
    out
}

/// The content lines of the element, as values.
fn lines(element: &Element) -> Vec<&[Value]> {
    element
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Content(Content { values, .. }) => Some(values.as_slice()),
            _ => None,
        })
        .collect()
}

/// The first value of every content line of the element.
fn first_words(element: &Element) -> Vec<&Value> {
    lines(element).iter().map(|values| &values[0]).collect()
}

#[test]
fn every_element_of_the_corpus_is_found_by_its_identifier() {
    for name in ["gestpret", "commande", "employe", "interpreteur_wl"] {
        let document = read(&corpus(&format!("{name}.fg")));
        let index = Index::build(&document);
        let ids = ids(&document);
        assert_eq!(index.len(), ids.len(), "{name}");
        for id in &ids {
            let found = index.get(id).unwrap_or_else(|| panic!("{name}: {id}"));
            assert_eq!(found.id, *id);
        }
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        let listed: Vec<&str> = index.iter().map(|(id, _)| id).collect();
        assert_eq!(listed, sorted, "{name}");
    }
}

#[test]
fn the_entities_named_by_the_associations_of_the_corpus_resolve() {
    // In Merise the first word of `MEMBRE 0,n` names an entity; the
    // notation knows which lines say so — here, the ones whose second
    // value is a cardinality.
    let mut resolved = 0;
    for name in ["gestpret", "commande", "employe", "interpreteur_wl"] {
        let document = read(&corpus(&format!("{name}.fg")));
        let index = Index::build(&document);
        for element in &document.elements {
            if element.kind != "association" {
                continue;
            }
            for values in lines(element) {
                let cardinality = values
                    .get(1)
                    .is_some_and(|v| matches!(v.text.as_str(), "0,1" | "0,n" | "1,1" | "1,n"));
                if cardinality {
                    let entity = index.resolve(&values[0]).unwrap();
                    assert_eq!(entity.kind, "entity", "{name}: {}", values[0].text);
                    resolved += 1;
                }
            }
        }
    }
    // Cardinality lines of the four models: 2 + 18 + 8 + 8, counted in
    // the files.
    assert_eq!(resolved, 36);
}

#[test]
fn a_reference_may_precede_its_definition() {
    let document = read("association A {\n  B 0,n\n}\nentity B {}\n");
    let index = Index::build(&document);
    let word = first_words(&document.elements[0])[0];
    let found = index.resolve(word).unwrap();
    assert_eq!(
        (found.kind.as_str(), found.id_pos),
        ("entity", Pos { line: 4, col: 8 })
    );
}

#[test]
fn nested_elements_are_indexed() {
    let document = read("group G {\n  entity A {\n    group H { entity B {} }\n  }\n}\n");
    let index = Index::build(&document);
    assert_eq!(index.len(), 4);
    assert_eq!(index.get("B").unwrap().pos, Pos { line: 3, col: 15 });
    assert_eq!(index.get("H").unwrap().kind, "group");
}

#[test]
fn an_unknown_identifier_is_refused_at_the_value() {
    let document = read("entity A {\n  x\n  B 0,n\n}\n");
    let index = Index::build(&document);
    let words = first_words(&document.elements[0]);
    let refused = index.resolve(words[1]).unwrap_err();
    assert_eq!(
        (refused.code, refused.pos),
        ("T015", Pos { line: 3, col: 3 })
    );
    assert_eq!(
        refused.text(),
        "reference `B`: no element has this identifier"
    );
    assert_eq!(
        refused.text_in(Catalog::french()),
        "référence `B`\u{a0}: aucun élément ne porte cet identifiant"
    );
    for code in Catalog::LANGUAGES {
        let catalog = Catalog::for_language(code).unwrap();
        assert_eq!(refused.message.check(catalog), Ok(()), "{code}");
    }
}

#[test]
fn case_matters() {
    let document = read("entity CLIENT {}\nentity client {}\nassociation A { Client }\n");
    let index = Index::build(&document);
    assert_eq!(index.len(), 3);
    assert_eq!(index.get("CLIENT").unwrap().pos, Pos { line: 1, col: 1 });
    assert_eq!(index.get("client").unwrap().pos, Pos { line: 2, col: 1 });
    let word = first_words(&document.elements[2])[0];
    assert_eq!(index.resolve(word).unwrap_err().code, "T015");
}

#[test]
fn an_element_may_name_itself_and_a_string_is_looked_up_by_its_text() {
    let document = read("entity A {\n  A 0,n\n  \"A\"\n  \"a\"\n}\n");
    let index = Index::build(&document);
    let words = first_words(&document.elements[0]);
    assert!(std::ptr::eq(
        index.resolve(words[0]).unwrap(),
        &document.elements[0]
    ));
    assert!(words[1].quoted);
    assert_eq!(index.resolve(words[1]).unwrap().id, "A");
    assert_eq!(index.resolve(words[2]).unwrap_err().code, "T015");
}

#[test]
fn an_empty_document_has_an_empty_index() {
    let document = read("// nothing\n");
    let index = Index::build(&document);
    assert!(index.is_empty());
    assert_eq!(index.get("A"), None);
    assert_eq!(index.iter().count(), 0);
}

#[test]
fn a_document_altered_by_hand_keeps_the_first_of_two_identifiers() {
    let mut document = read("entity A {}\nentity B {}\n");
    document.elements[1].id = "A".to_string();
    let index = Index::build(&document);
    assert_eq!(index.len(), 1);
    assert_eq!(index.get("A").unwrap().pos, Pos { line: 1, col: 1 });
}
