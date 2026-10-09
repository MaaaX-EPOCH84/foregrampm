// ForegramPM — tests du catalogue de messages : complet, sans entrée morte, sans clé mal écrite.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// Un test qui échoue doit s'arrêter net : les règles du produit ne valent pas ici.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeSet;

use foregram_graph::{
    Catalog, Diagnostic, Edge, EdgePlace, Engine, Graph, Layout, Node, NodePlace, Point, Pos,
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

/// Les refus que tous les cas ci-dessous provoquent : de quoi atteindre
/// chaque entrée du catalogue qui sert à un refus.
fn every_refusal() -> Vec<Diagnostic> {
    let mut all = Vec::new();

    // Syntaxe JSON.
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

    // Structure : chaque objet du format sans ses clés, chaque type de valeur reçu à tort.
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
        graph("", "").replace("\"version\": 1", "\"version\": 2"),
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

    // Sens du graphe.
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

    // Un graphe construit en mémoire n'a pas de position : les doublons n'ont pas de ligne.
    let node = |id: &str| Node {
        id: id.into(),
        width: 1,
        height: 1,
        layer: None,
        order: None,
        pos: Pos::NONE,
    };
    let edge = |id: &str| Edge {
        id: id.into(),
        source: "A".into(),
        target: "A".into(),
        source_side: None,
        target_side: None,
        labels: Vec::new(),
        pos: Pos::NONE,
    };
    let memory = Graph {
        name: "m".into(),
        direction: None,
        nodes: vec![node("A"), node("A")],
        edges: vec![edge("e"), edge("e")],
        pos: Pos::NONE,
    };
    all.extend(memory.validate());

    // Structure et sens du placement.
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
                pos: Pos::NONE,
            },
            NodePlace {
                id: "A".into(),
                x: 0,
                y: 0,
                pos: Pos::NONE,
            },
        ],
        edges: vec![
            EdgePlace {
                id: "e".into(),
                route: vec![Point { x: 0, y: 0 }, Point { x: 1, y: 0 }],
                labels: Vec::new(),
                pos: Pos::NONE,
            };
            2
        ],
        pos: Pos::NONE,
    };
    all.extend(memory_layout.validate());

    // Confrontation au graphe.
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
    // Les deux extrémités d'un tracé sur le bord, mais du mauvais côté.
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
    let mut reached: BTreeSet<&str> = BTreeSet::new();
    for diag in every_refusal() {
        reached.extend(diag.message.keys());
    }

    // La présentation d'un refus, avec et sans position.
    let positioned = Graph::parse(b"?").unwrap_err();
    assert_eq!(
        positioned[0].render(catalog),
        "1:1: error[J002]: unexpected character `?`: a value is expected"
    );
    reached.insert("diag.at");
    let mut bare = positioned[0].clone();
    bare.pos = Pos::NONE;
    assert!(bare.render(catalog).starts_with("error[J002]: "));
    reached.insert("diag.bare");

    // Les entrées de la ligne de commande du banc, que ses propres tests exercent.
    let bench = format!(
        "{}/../foregram-bench/src/main.rs",
        env!("CARGO_MANIFEST_DIR")
    );
    let bench = std::fs::read_to_string(&bench).unwrap();
    let bench_keys = key_literals(&bench);
    for key in &bench_keys {
        assert!(catalog.get(key).is_some(), "clé du banc absente : {key}");
    }
    let bench_keys: Vec<&str> = bench_keys.iter().map(String::as_str).collect();
    reached.extend(bench_keys);

    let all: BTreeSet<&str> = catalog.keys().collect();
    let missing: Vec<&&str> = reached.difference(&all).collect();
    let dead: Vec<&&str> = all.difference(&reached).collect();
    assert!(
        missing.is_empty(),
        "clés employées mais absentes : {missing:?}"
    );
    assert!(dead.is_empty(), "entrées jamais atteintes : {dead:?}");
}

#[test]
fn every_key_written_in_the_library_exists_in_the_catalogue() {
    // Une clé mal écrite sur un chemin qu'aucun test n'emprunte se verrait seulement
    // à l'affichage : on la cherche donc dans le texte même du code.
    let dir = format!("{}/src", env!("CARGO_MANIFEST_DIR"));
    let mark = concat!("#[cfg", "(test)]");
    let mut found = BTreeSet::new();
    for name in [
        "diag.rs",
        "graph.rs",
        "json.rs",
        "layout.rs",
        "read.rs",
        "msg.rs",
    ] {
        let text = std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
        found.extend(key_literals(text.split(mark).next().unwrap()));
    }
    assert!(found.len() > 40, "{found:?}");
    for key in &found {
        assert!(
            Catalog::english().get(key).is_some(),
            "clé absente du catalogue : {key}"
        );
    }
}

/// Les chaînes littérales qui ont la forme d'une clé de catalogue :
/// un code (`G003`, `J005.zero`) ou un fragment pointé (`subject.node`).
fn key_literals(source: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for literal in string_literals(source) {
        let word = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        };
        let parts: Vec<&str> = literal.split('.').collect();
        let head = parts[0].as_bytes();
        let code = head.len() == 4
            && b"JSGL".contains(&head[0])
            && head[1..].iter().all(u8::is_ascii_digit)
            && parts[1..].iter().all(|p| word(p));
        let fragment = parts.len() >= 2 && parts.iter().all(|p| word(p));
        if code || fragment {
            keys.insert(literal);
        }
    }
    keys
}

/// Les chaînes littérales d'un source Rust, commentaires et caractère `'"'` écartés.
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
