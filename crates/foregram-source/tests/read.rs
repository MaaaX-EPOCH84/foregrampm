// ForegramPM — tests of the source reader: model, positions, refusals, robustness.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop short: the product rules do not apply here.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use foregram_graph::{Catalog, Diagnostic};
use foregram_source::{Content, Document, Element, Item, NESTING_MAX, Value};

fn read(text: &str) -> Document {
    Document::parse(text.as_bytes()).unwrap_or_else(|diags| {
        let lines: Vec<String> = diags.iter().map(ToString::to_string).collect();
        panic!("refused:\n{}", lines.join("\n"))
    })
}

fn refusals(bytes: &[u8]) -> Vec<Diagnostic> {
    match Document::parse(bytes) {
        Ok(document) => panic!("accepted: {document:?}"),
        Err(diags) => diags,
    }
}

/// The refusals as (key, line, column), the key telling a variant apart.
fn places(text: &str) -> Vec<(&'static str, u32, u32)> {
    refusals(text.as_bytes())
        .iter()
        .map(|d| (d.message.key, d.pos.line, d.pos.col))
        .collect()
}

fn texts(values: &[Value]) -> Vec<&str> {
    values.iter().map(|v| v.text.as_str()).collect()
}

fn contents(element: &Element) -> Vec<Vec<&str>> {
    element
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Content(Content { values, .. }) => Some(texts(values)),
            _ => None,
        })
        .collect()
}

/// The document without its positions, to compare two writings of it.
fn shape(document: &Document) -> String {
    fn element(out: &mut String, e: &Element, depth: usize) {
        let label = e.label.as_ref().map(|l| l.text.as_str()).unwrap_or("-");
        out.push_str(&format!("{:depth$}{} {} {label}\n", "", e.kind, e.id));
        for item in &e.items {
            match item {
                Item::Element(child) => element(out, child, depth + 2),
                Item::Attribute(a) => out.push_str(&format!(
                    "{:depth$}  @{} {:?}\n",
                    "",
                    a.key,
                    texts(&a.values)
                )),
                Item::Content(c) => {
                    out.push_str(&format!("{:depth$}  {:?}\n", "", texts(&c.values)))
                }
            }
        }
    }
    let mut out = String::new();
    for d in &document.directives {
        out.push_str(&format!("@{} {:?}\n", d.key, texts(&d.values)));
    }
    for e in &document.elements {
        element(&mut out, e, 0);
    }
    out
}

fn corpus(name: &str) -> String {
    let path = format!("{}/../../corpus/mcd/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The blocks of a seed `.mcd` file: kind, name, content lines as words.
fn seed_blocks(text: &str) -> Vec<(String, String, Vec<Vec<String>>)> {
    let mut blocks: Vec<(String, String, Vec<Vec<String>>)> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let words: Vec<String> = line.split_whitespace().map(String::from).collect();
        if line.starts_with(' ') {
            blocks.last_mut().unwrap().2.push(words);
        } else {
            blocks.push((words[0].clone(), words[1].clone(), Vec::new()));
        }
    }
    blocks
}

// --------------------------------------------------------------- corpus

#[test]
fn the_four_models_of_the_corpus_are_read_as_their_seed_twins() {
    // Element counts made by hand in the sources: entities + associations.
    for (name, count, comments) in [
        ("gestpret", 3, 7),
        ("commande", 19, 8),
        ("employe", 6, 8),
        ("interpreteur_wl", 15, 9),
    ] {
        let document = read(&corpus(&format!("{name}.fg")));
        assert_eq!(document.elements.len(), count, "{name}");
        assert_eq!(document.comments.len(), comments, "{name}");
        let header: Vec<(&str, Vec<&str>)> = document
            .directives
            .iter()
            .map(|d| (d.key.as_str(), texts(&d.values)))
            .collect();
        assert_eq!(
            header,
            [("notation", vec!["merise"]), ("direction", vec!["right"])],
            "{name}"
        );
        let seed = seed_blocks(&corpus(&format!("{name}.mcd")));
        let read: Vec<(String, String, Vec<Vec<String>>)> = document
            .elements
            .iter()
            .map(|e| {
                let kind = if e.kind == "entity" {
                    "entite"
                } else {
                    &e.kind
                };
                let lines = contents(e)
                    .into_iter()
                    .map(|l| l.into_iter().map(String::from).collect())
                    .collect();
                (kind.to_string(), e.id.clone(), lines)
            })
            .collect();
        assert_eq!(read, seed, "{name}");
    }
}

#[test]
fn the_examples_of_the_documentation_are_read() {
    let path = format!("{}/../../docs/source-format.md", env!("CARGO_MANIFEST_DIR"));
    let doc = std::fs::read_to_string(&path).unwrap();
    let example = doc.split("\n## Example\n").nth(1).unwrap();
    let blocks: Vec<&str> = example.split("```\n").skip(1).step_by(2).collect();
    assert_eq!(blocks.len(), 2);
    let gestpret = read(blocks[0]);
    assert_eq!(gestpret.elements.len(), 3);
    assert_eq!(
        gestpret.elements[1].label.as_ref().unwrap().text,
        "Matériel"
    );
    // The second example nests an entity in a group and writes items on one line.
    let grouped = read(blocks[1]);
    assert_eq!(
        shape(&grouped),
        "@notation [\"merise\"]\n\
         group PERSONNES Personnes\n  \
           entity MEMBRE -\n    [\"#id_membre\"]\n    [\"nom\"]\n    [\"prenom\"]\n\
         entity MATERIEL -\n  [\"#id_materiel\"]\n  [\"designation\"]\n\
         association PRET -\n  [\"date_pret\"]\n  [\"MEMBRE\", \"0,n\"]\n  [\"MATERIEL\", \"0,n\"]\n"
    );
}

// --------------------------------------------------------------- model

#[test]
fn an_element_carries_its_parts_and_their_positions() {
    let document = read(
        "@notation merise\n\
         \n\
         entity MATERIEL \"Matériel prêté\" {\n\
         \x20 #id_materiel   // the key\n\
         \x20 @layer 0\n\
         }\n",
    );
    let d = &document.directives[0];
    assert_eq!((d.key.as_str(), d.pos.line, d.pos.col), ("notation", 1, 1));
    assert_eq!((d.values[0].pos.line, d.values[0].pos.col), (1, 11));
    let e = &document.elements[0];
    assert_eq!((e.kind.as_str(), e.pos.line, e.pos.col), ("entity", 3, 1));
    assert_eq!(
        (e.id.as_str(), e.id_pos.line, e.id_pos.col),
        ("MATERIEL", 3, 8)
    );
    let label = e.label.as_ref().unwrap();
    assert_eq!(
        (label.text.as_str(), label.quoted, label.pos.col),
        ("Matériel prêté", true, 17)
    );
    assert_eq!((e.close.line, e.close.col), (6, 1));
    match &e.items[..] {
        [Item::Content(c), Item::Attribute(a)] => {
            assert_eq!(
                (c.pos.line, c.pos.col, texts(&c.values)),
                (4, 3, vec!["#id_materiel"])
            );
            assert_eq!((a.key.as_str(), a.pos.line, a.pos.col), ("layer", 5, 3));
            assert_eq!(texts(&a.values), ["0"]);
        }
        other => panic!("{other:?}"),
    }
    let comment = &document.comments[0];
    assert_eq!(
        (comment.text.as_str(), comment.pos.line, comment.pos.col),
        (" the key", 4, 18)
    );
}

#[test]
fn columns_count_code_points() {
    // `é` is two bytes and one column.
    let document = read("a b { \"éé\" c }\n");
    let Item::Content(c) = &document.elements[0].items[0] else {
        panic!()
    };
    assert_eq!(c.values[1].text, "c");
    assert_eq!((c.values[1].pos.line, c.values[1].pos.col), (1, 12));
    // And a refusal after it too.
    assert_eq!(places("a b { \"éé\" }}\n"), [("T006", 1, 13)]);
}

#[test]
fn items_on_one_line_are_the_same_element_as_on_several() {
    let one = read("entity CLIENT { #id_client; nom }\n");
    let many = read("entity CLIENT {\n  #id_client\n  nom\n}\n");
    assert_eq!(shape(&one), shape(&many));
    assert_eq!(one.elements[0].items.len(), 2);
    // `}` ends the item before it, and an element may follow on the same line.
    let two = read("entity A { x } entity B {}\n");
    assert_eq!(shape(&two), "entity A -\n  [\"x\"]\nentity B -\n");
    // An empty item, `;` alone, is nothing, also at the top level.
    assert_eq!(
        shape(&read("; entity A { ;; x ; } ;\n")),
        "entity A -\n  [\"x\"]\n"
    );
}

#[test]
fn any_kind_and_any_key_parse() {
    let document = read("@weird \"a b\" 12.5\nnode n1 { 0,n; \"x y\" z; @k }\n");
    assert_eq!(
        shape(&document),
        "@weird [\"a b\", \"12.5\"]\nnode n1 -\n  [\"0,n\"]\n  [\"x y\", \"z\"]\n  @k []\n"
    );
    assert!(document.directives[0].values[0].quoted);
    assert!(!document.directives[0].values[1].quoted);
}

#[test]
fn strings_are_decoded() {
    let document = read("e A \"q\\\" b\\\\ n\\n t\\t \\u{e9}\\u{1F600}\" {}\n");
    assert_eq!(
        document.elements[0].label.as_ref().unwrap().text,
        "q\" b\\ n\n t\t é\u{1F600}"
    );
    // A tab inside a string is kept as is.
    let tab = read("e A \"a\tb\" {}\n");
    assert_eq!(tab.elements[0].label.as_ref().unwrap().text, "a\tb");
}

#[test]
fn comments_are_kept_aside_and_a_word_may_hold_slashes() {
    let document = read("// head\nentity A { url http://x/y // tail\n} //\n");
    let comments: Vec<(&str, u32, u32)> = document
        .comments
        .iter()
        .map(|c| (c.text.as_str(), c.pos.line, c.pos.col))
        .collect();
    assert_eq!(comments, [(" head", 1, 1), (" tail", 2, 27), ("", 3, 3)]);
    assert_eq!(contents(&document.elements[0]), [["url", "http://x/y"]]);
    // A comment is not part of a string, nor a string of a comment.
    let quoted = read("e A \"// no\" { // \"no\"\n}\n");
    assert_eq!(quoted.elements[0].label.as_ref().unwrap().text, "// no");
    assert_eq!(quoted.comments[0].text, " \"no\"");
}

#[test]
fn nesting_is_kept_in_the_model() {
    let document = read("group G { group H { entity A {} } x }\n");
    let Item::Element(h) = &document.elements[0].items[0] else {
        panic!()
    };
    let Item::Element(a) = &h.items[0] else {
        panic!()
    };
    assert_eq!((h.id.as_str(), a.id.as_str()), ("H", "A"));
}

// --------------------------------------------------------------- text

#[test]
fn an_empty_file_or_comments_only_is_an_empty_document() {
    assert_eq!(read(""), Document::default());
    let only = read("// nothing\n\n   \n// more");
    assert!(only.directives.is_empty() && only.elements.is_empty());
    assert_eq!(only.comments.len(), 2);
}

#[test]
fn a_byte_order_mark_is_dropped_before_counting_columns() {
    let document = read("\u{FEFF}entity A {}\n");
    assert_eq!(
        (document.elements[0].pos.line, document.elements[0].pos.col),
        (1, 1)
    );
    assert_eq!(places("\u{FEFF}}"), [("T006", 1, 1)]);
}

#[test]
fn crlf_and_lf_give_the_same_document() {
    let lf = "@notation merise\nentity A \"a\" { // c\n  x\n}\n";
    let crlf = lf.replace('\n', "\r\n");
    assert_eq!(read(lf), read(&crlf));
    // Positions after a CRLF line are those of the LF file.
    assert_eq!(places("a {\r\n}}\r\n"), [("T008.id", 1, 3), ("T006", 2, 2)]);
}

// --------------------------------------------------------------- refusals

#[test]
fn bytes_that_are_not_utf8_stop_the_reading() {
    let diags = refusals(b"entity A {\n  ab\xffc }}}\n");
    let found: Vec<(&str, u32, u32)> = diags
        .iter()
        .map(|d| (d.message.key, d.pos.line, d.pos.col))
        .collect();
    assert_eq!(found, [("T001", 2, 5)]);
    assert_eq!(places_bytes(b"\xef\xbb\xbfe\xc3"), [("T001", 1, 2)]);
}

fn places_bytes(bytes: &[u8]) -> Vec<(&'static str, u32, u32)> {
    refusals(bytes)
        .iter()
        .map(|d| (d.message.key, d.pos.line, d.pos.col))
        .collect()
}

#[test]
fn control_characters_are_refused_once_per_line() {
    assert_eq!(
        places("e A {\n  a\u{7}b\u{7}\u{0}\n  c\rd\n  \"s\u{1b}\"\n  // \u{7f}\n}\n"),
        [
            ("T002", 2, 4),
            ("T002", 3, 4),
            ("T002", 4, 5),
            ("T002", 5, 6)
        ]
    );
    let diags = refusals("e A { \u{7} }".as_bytes());
    assert_eq!(
        diags[0].text(),
        "control character U+0007 refused: only tab and line ends are allowed"
    );
}

#[test]
fn strings_and_escapes_are_checked() {
    assert_eq!(
        places("e A \"open {\n}\n"),
        [("T008.top", 1, 1), ("T003", 1, 5), ("T006", 2, 1)]
    );
    assert_eq!(places("e A \"x\\"), [("T008.top", 1, 1), ("T003", 1, 5)]);
    assert_eq!(
        places("e A \"\\q \\u00e9 \\u{} \\u{1234567} \\u{e9\" {}\n"),
        [
            ("T004", 1, 6),
            ("T004", 1, 9),
            ("T004", 1, 16),
            ("T004", 1, 21),
            ("T004", 1, 33)
        ]
    );
    assert_eq!(
        places("e A \"\\u{D800} \\u{110000}\" {}"),
        [("T005", 1, 6), ("T005", 1, 15)]
    );
    let diags = refusals(b"e A \"\\q\" {}");
    assert_eq!(diags[0].text(), "unknown escape `\\q`");
    let diags = refusals(b"e A \"\\u{D800}\" {}");
    assert_eq!(
        diags[0].text(),
        "escape `\\u{D800}` is not a Unicode scalar value"
    );
}

#[test]
fn braces_must_pair() {
    assert_eq!(places("}\ne A {}\n}"), [("T006", 1, 1), ("T006", 3, 1)]);
    // One refusal per open block, at its brace.
    assert_eq!(
        places("e A {\n  f B { x\n"),
        [("T007", 1, 5), ("T007", 2, 7)]
    );
}

#[test]
fn an_element_head_has_a_strict_shape() {
    assert_eq!(places("{ x }"), [("T008.kind", 1, 1)]);
    assert_eq!(places("\"e\" A {}"), [("T008.kind", 1, 1)]);
    assert_eq!(places("@e A {}"), [("T008.kind", 1, 1)]);
    assert_eq!(places("e {}"), [("T008.id", 1, 3)]);
    assert_eq!(places("e \"A\" {}"), [("T008.id", 1, 3)]);
    assert_eq!(places("e A B {}"), [("T008.label", 1, 5)]);
    assert_eq!(places("e A \"a\" b {}"), [("T008.label", 1, 9)]);
    // A faulty head still opens its block, which is read for its refusals.
    assert_eq!(
        places("e A B {\n  f 1x {}\n}\n"),
        [("T008.label", 1, 5), ("T009", 2, 5)]
    );
    // The block must open on the head line.
    assert_eq!(
        places("e A\n{\n}\n"),
        [("T008.top", 1, 1), ("T008.kind", 2, 1)]
    );
}

#[test]
fn a_run_of_stray_lines_is_refused_once() {
    assert_eq!(
        places("nom\nprenom\n\n#x\ne A {}\nlate\n"),
        [("T008.top", 1, 1), ("T008.top", 6, 1)]
    );
    assert_eq!(places("e A {} tail\n"), [("T008.top", 1, 8)]);
}

#[test]
fn identifiers_are_ascii_and_unique() {
    assert_eq!(places("e 1a {}"), [("T009", 1, 3)]);
    assert_eq!(places("e é {}"), [("T009", 1, 3)]);
    assert_eq!(places("e a-b {}"), [("T009", 1, 3)]);
    read("e _a {} e b_9 {} e Z {} e z {}");
    assert_eq!(
        places("e A {\n  g B {}\n}\nf B {}\ng A { h B {} }\n"),
        [("T012", 4, 3), ("T012", 5, 3), ("T012", 5, 9)]
    );
    let diags = refusals(b"e A {}\ne A {}");
    assert_eq!(diags[0].text(), "identifier `A` already defined at 1:3");
    assert_eq!(
        diags[0].text_in(Catalog::french()),
        "identifiant `A` déjà défini en 1:3"
    );
}

#[test]
fn the_header_comes_first_and_keys_are_given_once() {
    assert_eq!(places("e A {}\n@notation merise\n"), [("T010", 2, 1)]);
    assert_eq!(places("e A {}\n@\n"), [("T011", 2, 1)]);
    assert_eq!(places("@ notation\n"), [("T011", 1, 1)]);
    assert_eq!(places("e A { @ 1 }"), [("T011", 1, 7)]);
    assert_eq!(places("@a 1\n@b\n@a 2\n"), [("T013", 3, 1)]);
    assert_eq!(places("e A { @a 1; @a 2 }"), [("T013", 1, 13)]);
    // Same key in two blocks, or in the header and a block: allowed.
    read("@a 1\ne A { @a 1 }\ne B { @a 1; f C { @a 2 } }");
    let diags = refusals(b"@a 1\n@a 2");
    assert_eq!(diags[0].text(), "key `@a` already given at 1:1");
}

#[test]
fn nesting_is_bounded() {
    let nest = |levels: usize| {
        let mut text = String::new();
        for level in 0..levels {
            text.push_str(&format!("g G{level} {{\n"));
        }
        text.push_str(&"}\n".repeat(levels));
        text
    };
    assert_eq!(NESTING_MAX, 32);
    read(&nest(32));
    assert_eq!(places(&nest(33)), [("T014", 33, 7)]);
    let diags = refusals(nest(40).as_bytes());
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].text(), "more than 32 nested blocks");
    // Far deeper than a stack would bear, with no head and no end: the
    // recursion stops at the bound.
    let diags = refusals("{".repeat(200_000).as_bytes());
    let t014: Vec<&Diagnostic> = diags.iter().filter(|d| d.code == "T014").collect();
    assert_eq!(t014.len(), 1);
    assert_eq!((t014[0].pos.line, t014[0].pos.col), (1, 33));
    assert_eq!(diags.iter().filter(|d| d.code == "T007").count(), 33);
}

#[test]
fn every_refusal_is_reported_sorted_by_position() {
    let text = "e A {\n  @k 1\n  @k 2\n}\n}\ne A {}\nf 9 { \"x\\q\" }\n@late\n";
    assert_eq!(
        places(text),
        [
            ("T013", 3, 3),
            ("T006", 5, 1),
            ("T012", 6, 3),
            ("T009", 7, 3),
            ("T004", 7, 9),
            ("T010", 8, 1),
        ]
    );
}

#[test]
fn every_refusal_reads_in_both_languages() {
    let text = "e A {\n  \"\\q\" \"\\u{D800}\" \u{7}\n  @\n  @k; @k\n}\n}\ne A {}\ne 9 {}\n{}\ne a b {}\ne \"x\" {}\n@late\nstray\n\"open\ne B {";
    let diags = refusals(text.as_bytes());
    let mut keys: Vec<&str> = diags.iter().map(|d| d.message.key).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), 15, "{keys:?}");
    for diag in diags.iter().chain(&refusals(b"\xff")) {
        for code in Catalog::LANGUAGES {
            let catalog = Catalog::for_language(code).unwrap();
            assert_eq!(diag.message.check(catalog), Ok(()), "{code}: {diag}");
        }
    }
    // Deep nesting, the sixteenth key.
    assert!(
        refusals("{".repeat(40).as_bytes())
            .iter()
            .any(|d| d.code == "T014")
    );
}

// --------------------------------------------------------------- robustness

/// A small generator, so that the inputs are the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

#[test]
fn random_input_never_panics_and_reads_the_same_twice() {
    let pieces: [&[u8]; 18] = [
        b"{",
        b"}",
        b";",
        b"\n",
        b"\r\n",
        b"\r",
        b"\"",
        b"\\",
        b"\\u{",
        b"@",
        b"//",
        b" ",
        b"\t",
        b"entity",
        b"A",
        "é".as_bytes(),
        b"\x00",
        b"\xff",
    ];
    let mut rng = Lcg(84);
    let mut accepted = 0;
    for _ in 0..20_000 {
        let mut input = Vec::new();
        for _ in 0..rng.next() % 40 {
            if rng.next().is_multiple_of(8) {
                input.push((rng.next() % 256) as u8);
            } else {
                input.extend_from_slice(pieces[(rng.next() % 18) as usize]);
            }
        }
        let first = Document::parse(&input);
        assert_eq!(first, Document::parse(&input));
        match first {
            Ok(_) => accepted += 1,
            Err(diags) => {
                assert!(!diags.is_empty());
                assert!(
                    diags
                        .windows(2)
                        .all(|w| (w[0].pos, w[0].code) <= (w[1].pos, w[1].code))
                );
                assert!(diags.iter().all(|d| d.pos.line >= 1 && d.pos.col >= 1));
            }
        }
    }
    // The generator does reach well-formed files too.
    assert!(accepted > 100, "{accepted}");
}

#[test]
fn a_large_file_is_read_whole() {
    let mut text = String::from("@notation merise\n");
    for n in 0..60_000 {
        text.push_str(&format!(
            "entity E{n} \"Entité {n}\" {{\n  #id_{n}\n  nom; prenom\n  @layer {}\n}}\n",
            n % 7
        ));
    }
    assert!(text.len() > 4_000_000);
    let document = read(&text);
    assert_eq!(document.elements.len(), 60_000);
    let last = document.elements.last().unwrap();
    assert_eq!((last.pos.line, last.pos.col), (2 + 59_999 * 5, 1));
    assert_eq!(last.items.len(), 4);
}
