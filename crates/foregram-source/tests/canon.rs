// ForegramPM — tests of the canonical form: golden files, idempotence, no loss.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop short: the product rules do not apply here.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use foregram_graph::Pos;
use foregram_source::{Comment, Content, Directive, Document, Element, Item, Value};

fn read(text: &str) -> Document {
    Document::parse(text.as_bytes()).unwrap_or_else(|diags| {
        let lines: Vec<String> = diags.iter().map(ToString::to_string).collect();
        panic!("refused:\n{}", lines.join("\n"))
    })
}

fn canon(text: &str) -> String {
    read(text).to_text()
}

fn root(name: &str) -> String {
    let path = format!("{}/../../{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The document with every position erased, to compare what was read
/// with what was written.
fn erase(document: &mut Document) {
    fn comment(c: &mut Comment) {
        c.pos = Pos::NONE;
    }
    fn comments(cs: &mut [Comment]) {
        cs.iter_mut().for_each(comment);
    }
    fn values(vs: &mut [Value]) {
        for v in vs {
            v.pos = Pos::NONE;
        }
    }
    fn directive(d: &mut Directive) {
        d.pos = Pos::NONE;
        values(&mut d.values);
        comments(&mut d.before);
        d.after.as_mut().map(comment);
    }
    fn element(e: &mut Element) {
        e.pos = Pos::NONE;
        e.id_pos = Pos::NONE;
        e.close = Pos::NONE;
        if let Some(label) = &mut e.label {
            label.pos = Pos::NONE;
        }
        comments(&mut e.before);
        e.after_open.as_mut().map(comment);
        e.after_close.as_mut().map(comment);
        comments(&mut e.end);
        for item in &mut e.items {
            match item {
                Item::Element(child) => element(child),
                Item::Attribute(a) => directive(a),
                Item::Content(c) => {
                    c.pos = Pos::NONE;
                    values(&mut c.values);
                    comments(&mut c.before);
                    c.after.as_mut().map(comment);
                }
            }
        }
    }
    comments(&mut document.preamble);
    document.directives.iter_mut().for_each(directive);
    document.elements.iter_mut().for_each(element);
    comments(&mut document.end);
}

/// Writing then reading gives the document back, and writing again gives
/// the same text.
fn round_trip(document: &Document) -> String {
    let text = document.to_text();
    let mut again = Document::parse(text.as_bytes())
        .unwrap_or_else(|diags| panic!("{text}\nrefused: {diags:?}"));
    erase(&mut again);
    let mut expected = document.clone();
    erase(&mut expected);
    assert_eq!(again, expected, "{text}");
    assert_eq!(Document::parse(text.as_bytes()).unwrap().to_text(), text);
    text
}

// --------------------------------------------------------------- golden

#[test]
fn the_corpus_files_are_canonical() {
    for name in ["gestpret", "commande", "employe", "interpreteur_wl"] {
        let text = root(&format!("corpus/mcd/{name}.fg"));
        assert_eq!(canon(&text), text, "{name}");
        round_trip(&read(&text));
    }
}

#[test]
fn the_documentation_shows_the_canonical_form_as_it_is() {
    let doc = root("docs/source-format.md");
    let example = doc.split("\n## Example\n").nth(1).unwrap();
    let blocks: Vec<&str> = example.split("```\n").skip(1).step_by(2).collect();
    assert_eq!(blocks.len(), 3);
    // The first example is canonical; the second is folded, and the third
    // is what the canonical form makes of it.
    assert_eq!(canon(blocks[0]), blocks[0]);
    assert_eq!(canon(blocks[1]), blocks[2]);
    assert_eq!(canon(blocks[2]), blocks[2]);
    // The rules are illustrated by one file, shown as written and as
    // rewritten.
    let rules = doc.split("\n## Canonical form\n").nth(1).unwrap();
    let rules = rules.split("\n## ").next().unwrap();
    // Its blocks: the command, the file as written, as rewritten, the JSON.
    let shown: Vec<&str> = rules.split("```\n").skip(1).step_by(2).collect();
    assert_eq!(shown.len(), 4);
    assert_eq!(canon(shown[1]), shown[2]);
    assert_eq!(canon(shown[2]), shown[2]);
}

// --------------------------------------------------------------- rules

#[test]
fn items_go_one_per_line_two_spaces_per_level() {
    assert_eq!(
        canon("\t entity  A {#a;b   c ;@k 1}"),
        "entity A {\n  #a\n  b c\n  @k 1\n}\n"
    );
    assert_eq!(
        canon("g G { g H { e A {x} } }"),
        "g G {\n  g H {\n    e A {\n      x\n    }\n  }\n}\n"
    );
    // An empty block stays on the head line; `;` and blank lines vanish.
    assert_eq!(canon("; e A { ;\n\n ; } ;\n\n\n"), "e A {}\n");
    // No final newline, CRLF, tabs: same bytes.
    assert_eq!(canon("e A {\r\n\tx\r\n}"), "e A {\n  x\n}\n");
}

#[test]
fn blank_lines_follow_a_fixed_rule() {
    // One after the preamble, one between the header and the first element,
    // one between two elements, none between lines.
    assert_eq!(
        canon("// p\n@a 1\n\n\n@b 2\ne A {}\n\n\n\ne B {}\n// end\n"),
        "// p\n\n@a 1\n@b 2\n\ne A {}\n\ne B {}\n\n// end\n"
    );
    // Inside a block: a blank line on each side of a child element, none
    // between content lines and attributes.
    assert_eq!(
        canon("g G { x\n\n\ny; e A {}\n\n e B {}\n@k 1; z\n// end\n}"),
        "g G {\n  x\n  y\n\n  e A {}\n\n  e B {}\n\n  @k 1\n  z\n  // end\n}\n"
    );
    // A preamble or end comments alone: no blank line.
    assert_eq!(canon("// only\n\n// two"), "// only\n// two\n");
    assert_eq!(canon("@a\n// end"), "@a\n// end\n");
    assert_eq!(canon("// p\n\n@a"), "// p\n\n@a\n");
}

#[test]
fn comments_come_back_where_they_stood() {
    let text = "// p1\n// p2\n\n@a 1 // da\n// before b\n@b\n\n// before E\nentity E { // open\n  // before x\n  x // x\n  @k 1 // k\n\n  // before F\n  group F {} // F\n\n  y\n  // end of E\n} // close\n\n// end 1\n// end 2\n";
    assert_eq!(canon(text), text);
    // Folded on few lines, the same comments find the same places.
    let folded = "// p1\n// p2\n@a 1 // da\n// before b\n@b\n// before E\nentity E { // open\n// before x\nx; // x\n@k 1; // k\n// before F\ngroup F {}; // F\ny;\n// end of E\n}; // close\n// end 1\n// end 2";
    assert_eq!(canon(folded), text);
    // A comment keeps its text as written, trailing blanks apart.
    assert_eq!(
        canon("e A {} //   tab\there  \t\n"),
        "e A {} //   tab\there\n"
    );
    assert_eq!(canon("//\n//\n"), "//\n//\n");
    // A comment after the head of an empty block keeps the block open.
    assert_eq!(canon("e A { // c\n}"), "e A { // c\n}\n");
    assert_eq!(canon("e A {\n// c\n}"), "e A {\n  // c\n}\n");
}

#[test]
fn a_label_equal_to_the_identifier_is_not_written() {
    assert_eq!(canon("e A \"A\" {}"), "e A {}\n");
    assert_eq!(canon("e A \"a\" {}"), "e A \"a\" {}\n");
    assert_eq!(canon("e A \"Été\" {}"), "e A \"Été\" {}\n");
}

#[test]
fn strings_keep_their_quotes_and_take_the_shortest_escape() {
    // A quoted value stays quoted, a word stays a word.
    assert_eq!(
        canon("@a \"x\" y\ne A { \"b\" c }"),
        "@a \"x\" y\n\ne A {\n  \"b\" c\n}\n"
    );
    assert_eq!(
        canon("e A \"q\\\" b\\\\ n\\n t\\t \\u{7} \\u{e9}\\u{1F600} \\u{22}\" {}"),
        "e A \"q\\\" b\\\\ n\\n t\\t \\u{7} é\u{1F600} \\\"\" {}\n"
    );
    // A raw tab inside a string comes out as its escape.
    assert_eq!(canon("e A \"a\tb\" {}"), "e A \"a\\tb\" {}\n");
}

#[test]
fn a_value_that_cannot_be_a_word_is_written_as_a_string() {
    // Only a document built by hand can hold such a value.
    let mut document = read("e A { x }\n");
    let Item::Content(content) = &mut document.elements[0].items[0] else {
        panic!()
    };
    content.values[0].text = "a b".to_string();
    content.values.push(Value {
        text: String::new(),
        quoted: false,
        pos: Pos::NONE,
    });
    assert_eq!(document.to_text(), "e A {\n  \"a b\" \"\"\n}\n");
}

#[test]
fn comments_before_the_first_item_are_the_preamble_even_when_built_by_hand() {
    // The reader never puts a comment before the first item: it is the
    // preamble. A document built by hand that does gets the same text as
    // the document the reader would make of it.
    let mut document = read("// pre\n\nentity A {}\n");
    document.elements[0].before.push(Comment {
        text: " about A".to_string(),
        pos: Pos::NONE,
    });
    let text = document.to_text();
    assert_eq!(text, "// pre\n// about A\n\nentity A {}\n");
    assert_eq!(canon(&text), text);
    let mut document = read("@a\n");
    document.directives[0].before.push(Comment {
        text: String::new(),
        pos: Pos::NONE,
    });
    assert_eq!(document.to_text(), "//\n\n@a\n");
}

#[test]
fn the_empty_document_is_the_empty_text() {
    assert_eq!(Document::default().to_text(), "");
    assert_eq!(canon(""), "");
    assert_eq!(canon("\n\n;;\n"), "");
}

// --------------------------------------------------------------- no loss

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

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
        from[self.below(from.len() as u64) as usize]
    }
}

/// Builds documents that respect the model, with every feature of the
/// format, comments everywhere a comment may stand.
struct Maker {
    rng: Lcg,
    next_id: u32,
}

impl Maker {
    const COMMENTS: [&str; 6] = ["", " c", "  x y", "\tz", " é // not nested", " {;}\""];
    const WORDS: [&str; 7] = ["a", "0,n", "#id", "12.5", "http://x/y", "é", "a/b"];
    const STRINGS: [&str; 7] = ["", "a b", "q\"", "back\\", "n\nt\t", "\u{7}", "{;}//é"];

    fn comment(&mut self) -> Comment {
        Comment {
            text: self.rng.pick(&Self::COMMENTS).to_string(),
            pos: Pos::NONE,
        }
    }

    fn comments(&mut self) -> Vec<Comment> {
        (0..self.rng.below(3)).map(|_| self.comment()).collect()
    }

    fn maybe_comment(&mut self) -> Option<Comment> {
        (self.rng.below(3) == 0).then(|| self.comment())
    }

    fn value(&mut self) -> Value {
        if self.rng.below(3) == 0 {
            Value {
                text: self.rng.pick(&Self::STRINGS).to_string(),
                quoted: true,
                pos: Pos::NONE,
            }
        } else {
            Value {
                text: self.rng.pick(&Self::WORDS).to_string(),
                quoted: false,
                pos: Pos::NONE,
            }
        }
    }

    fn values(&mut self, min: u64) -> Vec<Value> {
        (0..min + self.rng.below(3)).map(|_| self.value()).collect()
    }

    fn directive(&mut self, n: usize) -> Directive {
        Directive {
            key: format!("k{n}"),
            values: self.values(0),
            pos: Pos::NONE,
            before: self.comments(),
            after: self.maybe_comment(),
        }
    }

    fn element(&mut self, depth: usize) -> Element {
        let id = format!("I{}", self.next_id);
        self.next_id += 1;
        let label = (self.rng.below(2) == 0).then(|| Value {
            text: self.rng.pick(&Self::STRINGS).to_string(),
            quoted: true,
            pos: Pos::NONE,
        });
        let count = if depth >= 4 { 0 } else { self.rng.below(4) };
        let mut items = Vec::new();
        for n in 0..count {
            let item = match self.rng.below(3) {
                0 => Item::Element(self.element(depth + 1)),
                1 => Item::Attribute(self.directive(n as usize)),
                _ => Item::Content(Content {
                    values: self.values(1),
                    pos: Pos::NONE,
                    before: self.comments(),
                    after: self.maybe_comment(),
                }),
            };
            items.push(item);
        }
        Element {
            kind: self
                .rng
                .pick(&["entity", "association", "group", "x"])
                .to_string(),
            id,
            label,
            items,
            pos: Pos::NONE,
            id_pos: Pos::NONE,
            close: Pos::NONE,
            before: self.comments(),
            after_open: self.maybe_comment(),
            after_close: self.maybe_comment(),
            end: self.comments(),
        }
    }

    fn document(&mut self) -> Document {
        self.next_id = 0;
        let directives = (0..self.rng.below(3))
            .map(|n| self.directive(n as usize))
            .collect();
        let elements = (0..self.rng.below(4)).map(|_| self.element(1)).collect();
        let mut document = Document {
            preamble: self.comments(),
            directives,
            elements,
            end: self.comments(),
        };
        // Without an item, every comment is the preamble; and the first
        // item has none before it, they would be the preamble too.
        if document.directives.is_empty() && document.elements.is_empty() {
            document.end.clear();
        }
        if let Some(first) = document.directives.first_mut() {
            first.before.clear();
        } else if let Some(first) = document.elements.first_mut() {
            first.before.clear();
        }
        document
    }
}

#[test]
fn any_document_comes_back_the_same_from_its_text() {
    let mut maker = Maker {
        rng: Lcg(84),
        next_id: 0,
    };
    let mut longest = 0;
    for _ in 0..3_000 {
        let document = maker.document();
        let text = round_trip(&document);
        longest = longest.max(text.lines().count());
    }
    assert!(longest > 40, "{longest}");
}

#[test]
fn any_text_that_reads_rewrites_the_same_twice() {
    let pieces: [&str; 22] = [
        "{", "}", ";", "\n", "\r\n", "\"", "@", "//", " ", "\t", "entity ", "A", "B ", "é ",
        "\"x y\" ", "// c\n", "@k 1\n", "e A {", "}\n", "x;", "\\n", "\\u{e9}",
    ];
    let mut rng = Lcg(2026);
    let mut accepted = 0;
    for _ in 0..20_000 {
        let mut input = String::new();
        for _ in 0..rng.below(30) {
            input.push_str(rng.pick(&pieces));
        }
        if let Ok(document) = Document::parse(input.as_bytes()) {
            accepted += 1;
            round_trip(&document);
        }
    }
    assert!(accepted > 1_000, "{accepted}");
}
