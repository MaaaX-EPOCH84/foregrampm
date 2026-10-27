// ForegramPM — tests of the message catalogue: complete, no dead entry, no misspelled key.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop dead: the product's rules do not apply here.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use std::collections::BTreeSet;

use foregram_graph::{
    Catalog, Diagnostic, Edge, EdgePlace, Engine, Graph, Layout, Metrics, Node, NodePlace, Point,
    Port, PortPlace, Pos, Segment, Size,
};

fn graph_refusals(text: &[u8]) -> Vec<Diagnostic> {
    Graph::parse(text).unwrap_err()
}

fn layout_refusals(text: &str) -> Vec<Diagnostic> {
    Layout::parse(text.as_bytes()).unwrap_err()
}

fn graph(nodes: &str, edges: &str) -> String {
    format!(
        "{{\"format\": \"foregram-graph\", \"version\": 1, \"name\": \"g\", \
         \"nodes\": [{nodes}], \"edges\": [{edges}]}}"
    )
}

fn layout(nodes: &str, edges: &str) -> String {
    format!(
        "{{\"format\": \"foregram-layout\", \"version\": 1, \"graph\": \"g\", \
         \"engine\": {{\"name\": \"m\", \"version\": \"1\"}}, \
         \"nodes\": [{nodes}], \"edges\": [{edges}]}}"
    )
}

const AB: &str = "{\"id\": \"A\", \"width\": 100, \"height\": 100}, \
                  {\"id\": \"B\", \"width\": 100, \"height\": 100}";

/// The refusals that all the cases below provoke: enough to reach
/// every catalogue entry that serves a refusal.
fn every_refusal() -> Vec<Diagnostic> {
    let mut all = Vec::new();

    // The refusals specific to version 2: ports, nesting, curves.
    for (_, diags) in common::cases() {
        all.extend(diags);
    }

    // JSON syntax.
    let deep = format!("{}{}", "[".repeat(65), "]".repeat(65));
    let json: [&[u8]; 28] = [
        b"[\n \xff]",
        b"\xef\xbb\xbf{}",
        b"?",
        b"tru",
        b"[1 2]",
        b"{\"a\" 1}",
        b"{a: 1}",
        b"{\"a\": 1 2}",
        b"",
        b"[1",
        b"{\"a\": 1",
        b"{\"a\"",
        b"{",
        b"\"abc",
        b"1.5",
        b"-",
        b"01",
        b"-0",
        b"9223372036854775808",
        b"\"a\tb\"",
        br#""\x""#,
        br#""\u12g4""#,
        br#""\ud83d""#,
        br#""\ude00""#,
        b"{\"a\": 1, \"a\": 2}",
        deep.as_bytes(),
        b"1 2",
        b"[]",
    ];
    for text in json {
        all.extend(graph_refusals(text));
    }

    // Structure: each object of the format without its keys, each value type received by mistake.
    let structure = [
        "{}".to_string(),
        graph("{}", "{}").replace("\"g\"", "null"),
        graph("{\"id\": \"A\", \"width\": true, \"height\": 1}", ""),
        graph(
            "7",
            "{\"id\": \"e\", \"source\": \"A\", \"target\": \"A\", \"labels\": [{}]}",
        ),
        graph("", "").replace("\"nodes\": []", "\"nodes\": \"x\""),
        graph("", "").replace("foregram-graph", "autre"),
        graph("", "").replace("\"format\": \"foregram-graph\"", "\"format\": 1"),
        graph("", "").replace("\"version\": 1", "\"version\": 3"),
        graph("", "").replace("\"version\": 1", "\"version\": \"1\""),
        graph("", "").replace(
            "\"name\": \"g\"",
            "\"name\": \"g\", \"direction\": \"north\", \"extra\": 0",
        ),
        graph(
            AB,
            "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\", \"source_side\": \"left\", \
             \"labels\": [{\"width\": 1, \"height\": 1, \"at\": \"middle\"}]}",
        ),
    ];
    for text in &structure {
        all.extend(graph_refusals(text.as_bytes()));
    }

    // Direction of the graph.
    let long = "x".repeat(256);
    let meaning = [graph(
        &format!(
            "{{\"id\": \"\", \"width\": 0, \"height\": 1}}, \
                 {{\"id\": \"a\\tb\", \"width\": 1, \"height\": 1}}, \
                 {{\"id\": \"{long}\", \"width\": 1, \"height\": 1}}, \
                 {{\"id\": \"C\", \"width\": 1, \"height\": 1, \"order\": 0}}, \
                 {{\"id\": \"D\", \"width\": 1, \"height\": 1, \"layer\": 1, \"order\": 0}}, \
                 {{\"id\": \"D\", \"width\": 1, \"height\": 1, \"layer\": 1, \"order\": 0}}"
        ),
        "{\"id\": \"\", \"source\": \"C\", \"target\": \"Z\", \
              \"labels\": [{\"width\": 0, \"height\": 1, \"at\": \"center\"}]}, \
             {\"id\": \"f\", \"source\": \"C\", \"target\": \"C\", \
              \"labels\": [{\"width\": 0, \"height\": 1, \"at\": \"target\"}]}, \
             {\"id\": \"f\", \"source\": \"C\", \"target\": \"C\"}",
    )];
    for text in &meaning {
        all.extend(graph_refusals(text.as_bytes()));
    }

    // A graph built in memory has no position: duplicates have no line.
    let node = |id: &str| Node {
        id: id.into(),
        size: Some(Size {
            width: 1,
            height: 1,
        }),
        layer: None,
        order: None,
        padding: None,
        ports_ordered: false,
        ports: Vec::new(),
        children: Vec::new(),
        pos: Pos::NONE,
    };
    let edge = |id: &str| Edge {
        id: id.into(),
        source: "A".into(),
        target: "A".into(),
        source_port: None,
        target_port: None,
        source_side: None,
        target_side: None,
        labels: Vec::new(),
        pos: Pos::NONE,
    };
    let mut twice = node("A");
    let port = Port {
        id: "p".into(),
        side: None,
        position: None,
        size: None,
        pos: Pos::NONE,
    };
    twice.ports = vec![port.clone(), port];
    let memory = Graph {
        name: "m".into(),
        direction: None,
        nodes: vec![node("A"), twice],
        edges: vec![edge("e"), edge("e")],
        pos: Pos::NONE,
    };
    all.extend(memory.validate());

    // Structure and direction of the layout.
    let layouts = [
        "[]".to_string(),
        "{}".to_string(),
        layout("{}", "{}").replace("{\"name\": \"m\", \"version\": \"1\"}", "{}"),
        layout(
            "{\"id\": \"A\", \"x\": 0, \"y\": 0}, {\"id\": \"A\", \"x\": 1000000001, \"y\": 0}",
            "{\"id\": \"e\", \"route\": [[0, 0]]}, \
             {\"id\": \"e\", \"route\": [[0, 0], [1000000001, -1000000001]]}",
        )
        .replace("\"graph\": \"g\"", "\"graph\": \"\""),
        layout("", "{\"id\": \"f\", \"route\": [[1]]}"),
    ];
    for text in &layouts {
        all.extend(layout_refusals(text));
    }
    let memory_layout = Layout {
        graph: "m".into(),
        engine: Engine {
            name: "m".into(),
            version: "1".into(),
            options: None,
        },
        nodes: vec![
            NodePlace {
                id: "A".into(),
                x: 0,
                y: 0,
                size: None,
                ports: Vec::new(),
                pos: Pos::NONE,
            },
            NodePlace {
                id: "A".into(),
                x: 0,
                y: 0,
                size: None,
                ports: vec![
                    PortPlace {
                        id: "q".into(),
                        x: 0,
                        y: 0,
                        pos: Pos::NONE,
                    },
                    PortPlace {
                        id: "q".into(),
                        x: 0,
                        y: 0,
                        pos: Pos::NONE,
                    },
                ],
                pos: Pos::NONE,
            },
        ],
        edges: vec![
            EdgePlace {
                id: "e".into(),
                route: vec![
                    Segment::Line(Point { x: 0, y: 0 }),
                    Segment::Line(Point { x: 1, y: 0 }),
                ],
                labels: Vec::new(),
                pos: Pos::NONE,
            };
            2
        ],
        pos: Pos::NONE,
    };
    all.extend(memory_layout.validate());

    // Comparison against the graph.
    let edges = "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\", \
                 \"labels\": [{\"width\": 1, \"height\": 1, \"at\": \"center\"}]}, \
                 {\"id\": \"s\", \"source\": \"A\", \"target\": \"B\", \
                 \"source_side\": \"west\", \"target_side\": \"east\"}, \
                 {\"id\": \"absent\", \"source\": \"A\", \"target\": \"B\"}";
    let checked = Graph::parse(graph(AB, edges).as_bytes()).unwrap();
    let placed = Layout::parse(
        layout(
            "{\"id\": \"A\", \"x\": 0, \"y\": 0}, {\"id\": \"Z\", \"x\": 0, \"y\": 0}",
            "{\"id\": \"e\", \"route\": [[50, 50], [400, 400]]}, \
             {\"id\": \"s\", \"route\": [[100, 50], [100, 50]]}, \
             {\"id\": \"inconnu\", \"route\": [[0, 0], [1, 1]]}",
        )
        .replace("\"graph\": \"g\"", "\"graph\": \"autre\"")
        .as_bytes(),
    )
    .unwrap();
    all.extend(placed.check(&checked));
    // Both ends of a route on the edge, but on the wrong side.
    let sided = Layout::parse(
        layout(
            "{\"id\": \"A\", \"x\": 0, \"y\": 0}, {\"id\": \"B\", \"x\": 300, \"y\": 0}",
            "{\"id\": \"e\", \"route\": [[100, 50], [300, 50]], \"labels\": [[0, 0]]}, \
             {\"id\": \"s\", \"route\": [[100, 50], [300, 50]]}, \
             {\"id\": \"absent\", \"route\": [[100, 50], [299, 50]]}",
        )
        .as_bytes(),
    )
    .unwrap();
    all.extend(sided.check(&checked));
    all
}

#[test]
fn every_refusal_agrees_with_the_catalogue() {
    for diag in every_refusal() {
        assert_eq!(diag.message.check(Catalog::english()), Ok(()), "{diag}");
    }
}

#[test]
fn every_catalogue_entry_is_reached_and_none_is_dead() {
    let catalog = Catalog::english();
    let mut reached: BTreeSet<String> = BTreeSet::new();
    for diag in every_refusal() {
        reached.extend(entries(catalog, diag.message.keys()));
    }

    // How a refusal is presented, with and without a position.
    let positioned = Graph::parse(b"?").unwrap_err();
    assert_eq!(
        positioned[0].render(catalog),
        "1:1: error[JSN002]: unexpected character `?`: a value is expected"
    );
    reached.insert("diag.at".into());
    let mut bare = positioned[0].clone();
    bare.pos = Pos::NONE;
    assert!(bare.render(catalog).starts_with("error[JSN002]: "));
    reached.insert("diag.bare".into());

    // The refusals of a file as a command line writes them.
    let report = foregram_graph::file_report("g.json", &[positioned[0].clone(), bare], catalog);
    assert_eq!(
        report,
        [
            "g.json:1:1: error[JSN002]: unexpected character `?`: a value is expected",
            "g.json: error[JSN002]: unexpected character `?`: a value is expected",
            "g.json: refused (2 errors)",
        ]
    );
    let one = foregram_graph::file_report("g.json", &positioned, catalog);
    assert_eq!(one.last().unwrap(), "g.json: refused (1 error)");
    reached.extend(entries(
        catalog,
        [
            "diag.file-at",
            "diag.file-bare",
            "diag.refused",
            "count.error",
        ],
    ));

    // The same refusals as JSON: position or `null`, key, arguments, sentence.
    let file = "a \"b\".json";
    assert_eq!(
        positioned[0].to_json(file, catalog),
        "{\"kind\": \"diagnostic\", \"file\": \"a \\\"b\\\".json\", \"code\": \"JSN002\", \
         \"line\": 1, \"col\": 1, \"key\": \"JSN002.value-char\", \"args\": {\"char\": \"?\"}, \
         \"text\": \"unexpected character `?`: a value is expected\"}"
    );
    let bare_json = report_bare(&positioned[0]).to_json(file, catalog);
    assert!(
        bare_json.contains("\"line\": null, \"col\": null"),
        "{bare_json}"
    );
    assert!(foregram_graph::json::parse(bare_json.as_bytes()).is_ok());

    // The report of the measures, one line per measure.
    for line in Metrics::default().report() {
        assert_eq!(line.check(catalog), Ok(()));
        reached.extend(entries(catalog, line.keys()));
    }

    // The entries of the product's command line, of the bench — command
    // line, GraphML, ELK and MCD imports, reference engines, board —, of
    // the rendering and of the source reader, which their own tests
    // exercise.
    let mut bench_keys = BTreeSet::new();
    for name in [
        "foregram/src/main.rs",
        "foregram-bench/src/main.rs",
        "foregram-bench/src/graphml.rs",
        "foregram-bench/src/elkt.rs",
        "foregram-bench/src/elk.rs",
        "foregram-bench/src/mcd.rs",
        "foregram-bench/src/engine/mod.rs",
        "foregram-bench/src/engine/dot.rs",
        "foregram-bench/src/engine/elk.rs",
        "foregram-bench/src/engine/fit.rs",
        "foregram-bench/src/board.rs",
        "foregram-render/src/lib.rs",
        "foregram-render/src/scene.rs",
        "foregram-render/src/svg.rs",
        "foregram-render/src/raster.rs",
        "foregram-source/src/canon.rs",
        "foregram-source/src/index.rs",
        "foregram-source/src/lex.rs",
        "foregram-source/src/parse.rs",
    ] {
        let path = format!("{}/../{name}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap();
        let mark = concat!("#[cfg", "(test)]");
        bench_keys.extend(key_literals(text.split(mark).next().unwrap()));
    }
    for key in &bench_keys {
        assert!(exists(catalog, key), "clé du banc absente : {key}");
    }
    reached.extend(entries(catalog, bench_keys.iter().map(String::as_str)));

    let all: BTreeSet<String> = catalog.keys().map(String::from).collect();
    let missing: Vec<&String> = reached.difference(&all).collect();
    let dead: Vec<&String> = all.difference(&reached).collect();
    assert!(
        missing.is_empty(),
        "clés employées mais absentes : {missing:?}"
    );
    assert!(dead.is_empty(), "entrées jamais atteintes : {dead:?}");
}

#[test]
fn every_key_written_in_the_library_exists_in_the_catalogue() {
    // A misspelled key on a path that no test takes would only show
    // on display: so it is looked for in the text of the code itself.
    let dir = format!("{}/src", env!("CARGO_MANIFEST_DIR"));
    let mark = concat!("#[cfg", "(test)]");
    let mut found = BTreeSet::new();
    for name in [
        "diag.rs",
        "graph.rs",
        "json.rs",
        "layout.rs",
        "metrics.rs",
        "read.rs",
        "msg.rs",
    ] {
        let text = std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
        found.extend(key_literals(text.split(mark).next().unwrap()));
    }
    assert!(found.len() > 40, "{found:?}");
    for key in &found {
        assert!(
            exists(Catalog::english(), key),
            "clé absente du catalogue : {key}"
        );
    }
}

/// The families that only the bench uses: it speaks only English, and the
/// French catalogue does not carry them.
fn bench_only(key: &str) -> bool {
    let head = key.split('.').next().unwrap_or("");
    matches!(head, "bench" | "board")
        || code_of(head).is_some_and(|(family, _)| BENCH.contains(&family))
}

/// Every code family, three capital letters each.
const FAMILIES: &[&str] = &[
    "JSN", "SCH", "GPH", "LYT", "GML", "ELK", "MCD", "REF", "DRW", "SRC",
];
/// The families of the bench's imports and reference engines.
const BENCH: &[&str] = &["GML", "ELK", "MCD", "REF"];

/// A refusal code split into its family and its number: three capital
/// letters, then three digits.
fn code_of(head: &str) -> Option<(&str, &str)> {
    let b = head.as_bytes();
    (b.len() == 6
        && b[..3].iter().all(u8::is_ascii_uppercase)
        && b[3..].iter().all(u8::is_ascii_digit))
    .then(|| head.split_at(3))
}

#[test]
fn every_code_has_a_known_family_and_a_number_other_than_000() {
    let mut codes = 0;
    for key in Catalog::english().keys().chain(Catalog::french().keys()) {
        let head = key.split('.').next().unwrap_or("");
        if head.bytes().next().is_some_and(|b| b.is_ascii_uppercase()) {
            let Some((family, number)) = code_of(head) else {
                panic!("code malformé : {key}")
            };
            assert!(FAMILIES.contains(&family), "famille inconnue : {key}");
            assert_ne!(number, "000", "000 est réservé : {key}");
            codes += 1;
        }
    }
    assert!(codes > 100, "{codes}");
}

#[test]
fn the_french_catalogue_has_every_key_of_the_product_and_no_other() {
    let english = Catalog::english();
    let french = Catalog::french();
    assert_eq!(french.language(), "fr");
    let product: BTreeSet<&str> = english.keys().filter(|key| !bench_only(key)).collect();
    let translated: BTreeSet<&str> = french.keys().collect();
    let missing: Vec<&&str> = product.difference(&translated).collect();
    let orphan: Vec<&&str> = translated.difference(&product).collect();
    assert!(
        missing.is_empty(),
        "clés absentes du français : {missing:?}"
    );
    assert!(
        orphan.is_empty(),
        "clés du français inconnues ou du banc : {orphan:?}"
    );
    assert!(product.len() > 160, "{}", product.len());
    // The bench does have its own families: the exclusion is not empty.
    assert!(english.keys().filter(|key| bench_only(key)).count() > 70);
}

#[test]
fn the_french_templates_have_the_same_slots_as_the_english_ones() {
    let english = Catalog::english();
    for key in Catalog::french().keys() {
        let fr = slots(Catalog::french().get(key).unwrap());
        let en = slots(english.get(key).unwrap());
        assert_eq!(fr, en, "emplacements différents pour {key}");
    }
    // Each refusal therefore also matches the French.
    for diag in every_refusal() {
        assert_eq!(diag.message.check(Catalog::french()), Ok(()), "{diag}");
    }
}

#[test]
fn every_plural_key_has_both_variants_in_each_language() {
    for code in Catalog::LANGUAGES {
        let catalog = Catalog::for_language(code).unwrap();
        let mut bases = 0;
        for key in catalog.keys() {
            if let Some(base) = key.strip_suffix(".one") {
                bases += 1;
                assert!(
                    catalog.get(&format!("{base}.other")).is_some(),
                    "{code} : {base}.other"
                );
            }
            if let Some(base) = key.strip_suffix(".other") {
                // `GPH007.other` is a refusal variant, not a plural.
                if base.starts_with("count.") {
                    assert!(
                        catalog.get(&format!("{base}.one")).is_some(),
                        "{code} : {base}.one"
                    );
                }
            }
        }
        assert!(bases >= 11, "{code} : {bases}");
    }
}

#[test]
fn french_follows_its_typography() {
    // Neither an ordinary space nor nothing before `;`, `!`, `?`, nor inside the « »: a
    // non-breaking space. The colons of a locator (« {path}: », « {line}:{col}: »)
    // keep the English form; those of a sentence take the non-breaking one.
    let nbsp = '\u{a0}';
    for key in Catalog::french().keys() {
        let text = Catalog::french().get(key).unwrap();
        for (i, c) in text.char_indices() {
            let before = text[..i].chars().last();
            let after = text[i + c.len_utf8()..].chars().next();
            match c {
                ';' | '!' | '?' | '»' => {
                    assert_eq!(before, Some(nbsp), "{key} : avant « {c} » dans {text:?}")
                }
                '«' => assert_eq!(after, Some(nbsp), "{key} : après « dans {text:?}"),
                ':' if before == Some(' ') => panic!("{key} : espace ordinaire avant « : »"),
                _ => {}
            }
        }
        assert!(
            !text.contains('"'),
            "{key} : guillemets droits dans {text:?}"
        );
    }
}

/// A used key exists: as is, or as the base of a plural whose
/// two variants are in the catalogue.
fn exists(catalog: &Catalog, key: &str) -> bool {
    catalog.get(key).is_some()
        || (catalog.get(&format!("{key}.one")).is_some()
            && catalog.get(&format!("{key}.other")).is_some())
}

/// The catalogue entries reached by used keys: the key
/// itself, or the two variants of a plural base.
fn entries<'a>(catalog: &Catalog, keys: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut out = Vec::new();
    for key in keys {
        if catalog.get(key).is_some() {
            out.push(key.to_string());
        } else {
            out.push(format!("{key}.one"));
            out.push(format!("{key}.other"));
        }
    }
    out
}

/// The names of the `{name}` slots of a template, as the library reads them.
fn slots(template: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let len = after
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .unwrap_or(after.len());
        if len > 0 && after[len..].starts_with('}') {
            out.insert(after[..len].to_string());
        }
        rest = after;
    }
    out
}

/// The string literals shaped like a catalogue key:
/// a code (`GPH003`, `JSN005.zero`) or a dotted fragment (`subject.node`) whose
/// family, the first segment, is one of the catalogue's — which
/// rules out ELK's option identifiers, dotted too.
fn key_literals(source: &str) -> BTreeSet<String> {
    let families: BTreeSet<&str> = Catalog::english()
        .keys()
        .filter_map(|key| key.split('.').next())
        .filter(|family| family.chars().all(char::is_lowercase))
        .collect();
    let mut keys = BTreeSet::new();
    for literal in string_literals(source) {
        let word = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        };
        let parts: Vec<&str> = literal.split('.').collect();
        let code = code_of(parts[0]).is_some_and(|(family, _)| FAMILIES.contains(&family))
            && parts[1..].iter().all(|p| word(p));
        let fragment =
            parts.len() >= 2 && parts.iter().all(|p| word(p)) && families.contains(parts[0]);
        if code || fragment {
            keys.insert(literal);
        }
    }
    keys
}

/// The string literals of a Rust source, comments and the `'"'` character excluded.
fn string_literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        let line = line.replace("'\"'", "");
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            if c != '"' {
                continue;
            }
            let mut literal = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        chars.next();
                    }
                    '"' => break,
                    c => literal.push(c),
                }
            }
            out.push(literal);
        }
    }
    out
}

/// The same refusal, without a position.
fn report_bare(diag: &Diagnostic) -> Diagnostic {
    let mut bare = diag.clone();
    bare.pos = Pos::NONE;
    bare
}
