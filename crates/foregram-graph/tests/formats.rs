// ForegramPM — tests of the exchange formats: graph and layout.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop dead: the product's rules do not apply here.
#![allow(clippy::unwrap_used, clippy::panic)]

use foregram_graph::{
    Anchor, COORD_MAX, Diagnostic, Direction, Edge, Graph, IDENT_MAX_CHARS, Layout, Node, Pos,
    Side, Size,
};

/// Reads an example file by its name, never by its rank.
fn data(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path} : {e}"))
}

fn gestpret() -> Graph {
    Graph::parse(&data("gestpret.graph.json")).unwrap()
}

/// Code, line and column of each refusal, in the order produced.
fn places(diags: &[Diagnostic]) -> Vec<(&'static str, u32, u32)> {
    diags
        .iter()
        .map(|d| (d.code, d.pos.line, d.pos.col))
        .collect()
}

fn codes(diags: &[Diagnostic]) -> Vec<&'static str> {
    diags.iter().map(|d| d.code).collect()
}

fn graph_refusals(text: &str) -> Vec<Diagnostic> {
    Graph::parse(text.as_bytes()).unwrap_err()
}

fn layout_refusals(text: &str) -> Vec<Diagnostic> {
    Layout::parse(text.as_bytes()).unwrap_err()
}

/// A graph with two nodes "A" and "B", whose body is replaced.
fn graph_with(nodes: &str, edges: &str) -> String {
    format!(
        "{{\"format\": \"foregram-graph\", \"version\": 1, \"name\": \"g\",\n\
         \"nodes\": [{nodes}],\n\
         \"edges\": [{edges}]}}"
    )
}

const AB: &str = "{\"id\": \"A\", \"width\": 100, \"height\": 100},\n\
                  {\"id\": \"B\", \"width\": 100, \"height\": 100}";

fn layout_with(nodes: &str, edges: &str) -> String {
    format!(
        "{{\"format\": \"foregram-layout\", \"version\": 1, \"graph\": \"g\",\n\
         \"engine\": {{\"name\": \"m\", \"version\": \"1\"}},\n\
         \"nodes\": [{nodes}],\n\
         \"edges\": [{edges}]}}"
    )
}

// ---------------------------------------------------------------- graph

#[test]
fn the_example_graph_is_read_whole() {
    let graph = gestpret();
    assert_eq!(graph.name, "gestpret");
    assert_eq!(graph.direction, Some(Direction::Right));
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
    let pret = graph.node("PRET").unwrap();
    assert_eq!(
        pret.size,
        Some(Size {
            width: 18000,
            height: 8800
        })
    );
    assert_eq!(pret.pos, Pos { line: 9, col: 5 });
    let edge = &graph.edges[0];
    assert_eq!(
        (edge.source.as_str(), edge.target.as_str()),
        ("MEMBRE", "PRET")
    );
    assert_eq!(edge.labels.len(), 1);
    assert_eq!(edge.labels[0].at, Anchor::Source);
    assert!(graph.node("ABSENT").is_none());
}

#[test]
fn the_example_graph_is_already_canonical() {
    let bytes = data("gestpret.graph.json");
    assert_eq!(gestpret().to_json().as_bytes(), bytes.as_slice());
}

#[test]
fn a_graph_in_any_key_order_comes_back_canonical() {
    let loose = "{\"edges\":[{\"target\":\"B\",\"labels\":[],\"source\":\"A\",\"id\":\"e\",
        \"target_side\":\"west\",\"source_side\":\"east\"}],
        \"nodes\":[{\"height\":2,\"order\":1,\"id\":\"A\",\"layer\":0,\"width\":1},
        {\"id\":\"B\",\"width\":3,\"height\":4}],
        \"direction\":\"down\",\"version\":1,\"name\":\"é \\\"x\\\"\",\"format\":\"foregram-graph\"}";
    let graph = Graph::parse(loose.as_bytes()).unwrap();
    let canonical = graph.to_json();
    assert_eq!(
        canonical,
        "{\n  \"format\": \"foregram-graph\",\n  \"version\": 2,\n  \"name\": \"é \\\"x\\\"\",\n  \
         \"direction\": \"down\",\n  \"nodes\": [\n    \
         {\"id\": \"A\", \"width\": 1, \"height\": 2, \"layer\": 0, \"order\": 1},\n    \
         {\"id\": \"B\", \"width\": 3, \"height\": 4}\n  ],\n  \"edges\": [\n    \
         {\"id\": \"e\", \"source\": \"A\", \"target\": \"B\", \"source_side\": \"east\", \
         \"target_side\": \"west\"}\n  ]\n}\n"
    );
    // The canonical form is a fixed point: reading it back then rewriting it changes nothing.
    let again = Graph::parse(canonical.as_bytes()).unwrap().to_json();
    assert_eq!(again, canonical);
}

#[test]
fn an_empty_graph_is_valid() {
    let graph = Graph::parse(graph_with("", "").as_bytes()).unwrap();
    assert!(graph.nodes.is_empty() && graph.edges.is_empty());
    assert_eq!(
        graph.to_json(),
        "{\n  \"format\": \"foregram-graph\",\n  \"version\": 2,\n  \"name\": \"g\",\n  \
         \"nodes\": [],\n  \"edges\": []\n}\n"
    );
}

#[test]
fn loops_and_parallel_edges_are_valid() {
    let edges = "{\"id\": \"boucle\", \"source\": \"A\", \"target\": \"A\"},
                 {\"id\": \"e1\", \"source\": \"A\", \"target\": \"B\"},
                 {\"id\": \"e2\", \"source\": \"A\", \"target\": \"B\"},
                 {\"id\": \"e3\", \"source\": \"B\", \"target\": \"A\"}";
    let graph = Graph::parse(graph_with(AB, edges).as_bytes()).unwrap();
    assert_eq!(graph.edges.len(), 4);
}

#[test]
fn node_and_edge_identifiers_live_apart() {
    let edges = "{\"id\": \"A\", \"source\": \"A\", \"target\": \"B\"}";
    assert!(Graph::parse(graph_with(AB, edges).as_bytes()).is_ok());
}

#[test]
fn an_edge_identifier_is_used_once() {
    let edges = "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\"},\n\
                 {\"id\": \"e\", \"source\": \"B\", \"target\": \"A\"}";
    let diags = graph_refusals(&graph_with(AB, edges));
    assert_eq!(places(&diags), [("G002", 5, 1)]);
    assert_eq!(
        diags[0].text(),
        "edge identifier \"e\" already used (line 4)"
    );
}

#[test]
fn a_syntax_error_is_the_only_refusal() {
    let diags = graph_refusals("{\"format\": \"foregram-graph\", \"width\": 1.5}");
    assert_eq!(places(&diags), [("J004", 1, 39)]);
}

#[test]
fn the_root_must_be_a_graph_object() {
    assert_eq!(places(&graph_refusals("[]")), [("S001", 1, 1)]);
    assert_eq!(
        codes(&graph_refusals("{}")),
        ["S003", "S003", "S003", "S003", "S003"]
    );
}

#[test]
fn format_and_version_are_checked() {
    let text = graph_with("", "").replace("foregram-graph", "foregram-layout");
    assert_eq!(codes(&graph_refusals(&text)), ["S004"]);
    // Version 2 is read, like 1; 3 does not exist yet.
    let text = graph_with("", "").replace("\"version\": 1", "\"version\": 2");
    assert!(Graph::parse(text.as_bytes()).is_ok());
    let text = graph_with("", "").replace("\"version\": 1", "\"version\": 3");
    assert_eq!(codes(&graph_refusals(&text)), ["S005"]);
    assert!(
        graph_refusals(&text)[0]
            .text()
            .contains("versions 1 to 2 are read")
    );
    let text = graph_with("", "").replace("\"version\": 1", "\"version\": \"1\"");
    assert_eq!(codes(&graph_refusals(&text)), ["S001"]);
}

#[test]
fn structure_refusals_are_all_reported_with_their_place() {
    let nodes = "{\"id\": \"A\", \"width\": \"large\", \"height\": 100, \"colour\": 1},\n\
                 {\"id\": \"B\", \"width\": 100},\n\
                 7";
    let edges = "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\", \"source_side\": \"left\",\n\
                 \"labels\": [{\"width\": 1, \"height\": 1, \"at\": \"middle\"}]}";
    let text = format!(
        "{{\"format\": \"foregram-graph\", \"version\": 1, \"name\": \"g\", \"direction\": \"north\",\n\
         \"nodes\": [{nodes}],\n\"edges\": [{edges}], \"extra\": null}}"
    );
    let diags = graph_refusals(&text);
    assert_eq!(
        places(&diags),
        [
            ("S006", 1, 70), // unknown direction
            ("S001", 2, 32), // width is not an integer
            ("S002", 2, 56), // unknown key « colour »
            ("S003", 3, 1),  // height missing
            ("S001", 4, 1),  // a node is not an object
            ("S006", 5, 68), // unknown side
            ("S006", 6, 44), // unknown anchor
            ("S002", 6, 58), // unknown key « extra »
        ]
    );
    assert!(diags[0].text().contains("right, down, left, up"));
}

#[test]
fn meaning_refusals_are_all_reported() {
    let diags = Graph::parse(&data("refuse-lien-orphelin.graph.json")).unwrap_err();
    assert_eq!(
        places(&diags),
        [("G001", 7, 5), ("S008", 7, 5), ("G003", 10, 5)]
    );
    assert_eq!(
        diags[0].text(),
        "node identifier \"A\" already used (line 6)"
    );
    assert_eq!(
        diags[2].text(),
        "edge \"e1\": `target` refers to node \"B\", which does not exist"
    );
}

#[test]
fn sizes_and_constraints_have_bounds() {
    let nodes = format!(
        "{{\"id\": \"A\", \"width\": 0, \"height\": {}}},\n\
         {{\"id\": \"B\", \"width\": {COORD_MAX}, \"height\": 1, \"layer\": -1}},\n\
         {{\"id\": \"C\", \"width\": 1, \"height\": 1, \"order\": 0}},\n\
         {{\"id\": \"D\", \"width\": 1, \"height\": 1, \"layer\": 2, \"order\": 3}},\n\
         {{\"id\": \"E\", \"width\": 1, \"height\": 1, \"layer\": 2, \"order\": 3}},\n\
         {{\"id\": \"F\", \"width\": 1, \"height\": 1, \"layer\": 2, \"order\": 4}}",
        COORD_MAX + 1
    );
    let edges = "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\",\n\
                 \"labels\": [{\"width\": 1, \"height\": 1, \"at\": \"center\"}, \
                 {\"width\": -5, \"height\": 1, \"at\": \"target\"}]}";
    let diags = graph_refusals(&graph_with(&nodes, edges));
    assert_eq!(
        places(&diags),
        [
            ("S008", 2, 11), // A: width 0
            ("S008", 2, 11), // A: height too large
            ("S008", 3, 1),  // B: negative layer
            ("G004", 4, 1),  // C: order without layer
            ("G005", 6, 1),  // E: same place as D
            ("S008", 8, 11), // label no. 2: negative width
        ]
    );
    assert!(diags[5].text().starts_with("edge \"e\", label #2"));
}

#[test]
fn identifiers_have_rules() {
    let long = "x".repeat(IDENT_MAX_CHARS + 1);
    let fits = "é".repeat(IDENT_MAX_CHARS);
    let nodes = format!(
        "{{\"id\": \"\", \"width\": 1, \"height\": 1}},\n\
         {{\"id\": \"a\\tb\", \"width\": 1, \"height\": 1}},\n\
         {{\"id\": \"{long}\", \"width\": 1, \"height\": 1}},\n\
         {{\"id\": \"{fits}\", \"width\": 1, \"height\": 1}}"
    );
    let diags = graph_refusals(&graph_with(&nodes, ""));
    assert_eq!(codes(&diags), ["S007", "S007", "S007"]);
    assert_eq!(diags[0].text(), "node #1: `id`: identifier empty");
    assert!(diags[1].text().starts_with("node #2: `id`"));
    assert!(diags[2].text().contains("256 characters, at most 255"));
}

#[test]
fn a_graph_built_in_memory_is_validated_the_same_way() {
    let node = |id: &str| Node {
        id: id.to_string(),
        size: Some(Size {
            width: 10,
            height: 10,
        }),
        layer: None,
        order: None,
        padding: None,
        ports_ordered: false,
        ports: Vec::new(),
        children: Vec::new(),
        pos: Pos::NONE,
    };
    let mut graph = Graph {
        name: "mémoire".to_string(),
        direction: None,
        nodes: vec![node("A"), node("B")],
        edges: vec![Edge {
            id: "e".to_string(),
            source: "A".to_string(),
            target: "B".to_string(),
            source_port: None,
            target_port: None,
            source_side: Some(Side::South),
            target_side: None,
            labels: Vec::new(),
            pos: Pos::NONE,
        }],
        pos: Pos::NONE,
    };
    assert!(graph.validate().is_empty());
    // What comes out of memory reads back identically.
    let text = graph.to_json();
    assert_eq!(Graph::parse(text.as_bytes()).unwrap().to_json(), text);

    graph.nodes.push(node("A"));
    graph.edges[0].target = "Z".to_string();
    let diags = graph.validate();
    assert_eq!(codes(&diags), ["G001", "G003"]);
    // Without source text, no position, and the message does not pretend to have one.
    assert!(diags[0].pos.is_none());
    assert_eq!(
        diags[0].to_string(),
        "error[G001]: node identifier \"A\" already used"
    );
}

// ------------------------------------------------------------- layout

#[test]
fn the_example_layout_fits_its_graph() {
    let layout = Layout::parse(&data("gestpret.layout.json")).unwrap();
    assert_eq!(layout.engine.name, "main");
    assert_eq!(layout.edges[0].route.len(), 4);
    assert!(layout.check(&gestpret()).is_empty());
}

#[test]
fn the_example_layout_is_already_canonical() {
    let bytes = data("gestpret.layout.json");
    let layout = Layout::parse(&bytes).unwrap();
    assert_eq!(layout.to_json().as_bytes(), bytes.as_slice());
}

#[test]
fn an_empty_layout_fits_an_empty_graph() {
    let graph = Graph::parse(graph_with("", "").as_bytes()).unwrap();
    let layout = Layout::parse(layout_with("", "").as_bytes()).unwrap();
    assert!(layout.check(&graph).is_empty());
    assert_eq!(
        layout.to_json(),
        "{\n  \"format\": \"foregram-layout\",\n  \"version\": 2,\n  \"graph\": \"g\",\n  \
         \"engine\": {\"name\": \"m\", \"version\": \"1\"},\n  \"nodes\": [],\n  \"edges\": []\n}\n"
    );
}

#[test]
fn a_layout_is_validated_on_its_own() {
    let nodes = format!(
        "{{\"id\": \"A\", \"x\": 0, \"y\": 0}},\n\
         {{\"id\": \"A\", \"x\": {}, \"y\": 0}}",
        COORD_MAX + 1
    );
    let edges = "{\"id\": \"e\", \"route\": [[0, 0]]},\n\
                 {\"id\": \"e\", \"route\": [[0, 0], [1, 1]]}";
    let diags = layout_refusals(&layout_with(&nodes, edges));
    assert_eq!(
        places(&diags),
        [
            ("L003", 4, 1),
            ("S008", 4, 1),
            ("L008", 5, 11),
            ("L006", 6, 1)
        ]
    );
}

#[test]
fn layout_structure_is_strict() {
    let edges = "{\"id\": \"e\", \"route\": [[0, 0], [1], [1, \"2\"], 3]},\n\
                 {\"id\": \"f\", \"route\": [[0, 0], [1, 1]], \"labels\": {}}";
    let text = layout_with("{\"id\": \"A\", \"x\": 0}", edges).replace(
        "\"version\": \"1\"}",
        "\"version\": \"1\", \"date\": \"x\"}",
    );
    let diags = layout_refusals(&text);
    assert_eq!(
        codes(&diags),
        ["S002", "S003", "S001", "S001", "S001", "S001"]
    );
    assert!(
        diags[2]
            .text()
            .contains("a route element is a point [x, y] or a curve")
    );
    // Labels, for their part, are points and nothing else.
    let edges = "{\"id\": \"e\", \"route\": [[0, 0], [1, 1]], \"labels\": [[1]]}";
    let diags = layout_refusals(&layout_with("", edges));
    assert_eq!(codes(&diags), ["S001"]);
    assert!(diags[0].text().contains("a point is written [x, y]"));
}

#[test]
fn a_layout_must_cover_its_graph_and_nothing_else() {
    let layout = Layout::parse(&data("refuse-hors-bord.layout.json")).unwrap();
    let diags = layout.check(&gestpret());
    assert_eq!(
        places(&diags),
        [
            ("L004", 1, 1),  // MATERIEL not placed
            ("L007", 1, 1),  // materiel-pret without route
            ("L002", 9, 5),  // INCONNU absent from the graph
            ("L009", 12, 5), // start outside the edge of MEMBRE
            ("L011", 12, 5), // missing label
        ]
    );
    assert_eq!(
        diags[3].text(),
        "edge \"membre-pret\": the route starts at (13000, 7800), off the border of node \"MEMBRE\""
    );
}

#[test]
fn a_layout_routes_only_the_edges_of_its_graph() {
    let graph = Graph::parse(graph_with(AB, "").as_bytes()).unwrap();
    let nodes = "{\"id\": \"A\", \"x\": 0, \"y\": 0}, {\"id\": \"B\", \"x\": 300, \"y\": 0}";
    let edges = "{\"id\": \"fantôme\", \"route\": [[100, 50], [300, 50]]}";
    let layout = Layout::parse(layout_with(nodes, edges).as_bytes()).unwrap();
    let diags = layout.check(&graph);
    assert_eq!(codes(&diags), ["L005"]);
    assert_eq!(diags[0].text(), "edge \"fantôme\" is not in the graph");
}

#[test]
fn a_layout_names_its_graph() {
    let graph = Graph::parse(graph_with("", "").replace("\"g\"", "\"autre\"").as_bytes()).unwrap();
    let layout = Layout::parse(layout_with("", "").as_bytes()).unwrap();
    assert_eq!(codes(&layout.check(&graph)), ["L001"]);
}

#[test]
fn route_ends_sit_on_the_node_border() {
    let graph = Graph::parse(
        graph_with(AB, "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\"}").as_bytes(),
    )
    .unwrap();
    // A occupies (0,0)-(100,100); B occupies (300,0)-(400,100).
    let nodes = "{\"id\": \"A\", \"x\": 0, \"y\": 0}, {\"id\": \"B\", \"x\": 300, \"y\": 0}";
    let check = |route: &str| {
        let edges = format!("{{\"id\": \"e\", \"route\": {route}}}");
        let layout = Layout::parse(layout_with(nodes, &edges).as_bytes()).unwrap();
        codes(&layout.check(&graph))
    };
    let none: [&str; 0] = [];
    assert_eq!(check("[[100, 50], [300, 50]]"), none); // right edge, left edge
    assert_eq!(check("[[100, 100], [300, 0]]"), none); // two corners
    assert_eq!(check("[[50, 0], [350, 100]]"), none); // top edge, bottom edge
    assert_eq!(check("[[50, 50], [300, 50]]"), ["L009"]); // start inside
    assert_eq!(check("[[101, 50], [300, 50]]"), ["L009"]); // start one notch outside
    assert_eq!(check("[[100, 50], [299, 50]]"), ["L009"]); // arrival one notch short
    assert_eq!(check("[[100, 101], [401, 50]]"), ["L009", "L009"]);
}

#[test]
fn imposed_sides_are_respected() {
    let edge = "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\", \
                \"source_side\": \"east\", \"target_side\": \"north\"}";
    let graph = Graph::parse(graph_with(AB, edge).as_bytes()).unwrap();
    let nodes = "{\"id\": \"A\", \"x\": 0, \"y\": 0}, {\"id\": \"B\", \"x\": 300, \"y\": 0}";
    let check = |route: &str| {
        let edges = format!("{{\"id\": \"e\", \"route\": {route}}}");
        let layout = Layout::parse(layout_with(nodes, &edges).as_bytes()).unwrap();
        layout.check(&graph)
    };
    assert!(check("[[100, 50], [350, 0]]").is_empty());
    assert!(check("[[100, 0], [300, 0]]").is_empty()); // a corner lies on two sides
    let diags = check("[[50, 100], [300, 50]]");
    assert_eq!(codes(&diags), ["L010", "L010"]);
    assert!(
        diags[0]
            .text()
            .contains("off the imposed side `east` of node \"A\"")
    );
    assert!(
        diags[1]
            .text()
            .contains("off the imposed side `north` of node \"B\"")
    );
}

#[test]
fn a_loop_is_checked_at_both_ends_of_the_same_node() {
    let graph = Graph::parse(
        graph_with(AB, "{\"id\": \"e\", \"source\": \"A\", \"target\": \"A\"}").as_bytes(),
    )
    .unwrap();
    let nodes = "{\"id\": \"A\", \"x\": 0, \"y\": 0}, {\"id\": \"B\", \"x\": 300, \"y\": 0}";
    let edges = "{\"id\": \"e\", \"route\": [[100, 20], [150, 20], [150, 80], [100, 80]]}";
    let layout = Layout::parse(layout_with(nodes, edges).as_bytes()).unwrap();
    assert!(layout.check(&graph).is_empty());
}

#[test]
fn negative_coordinates_are_valid() {
    let graph = Graph::parse(
        graph_with(AB, "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\"}").as_bytes(),
    )
    .unwrap();
    let nodes = "{\"id\": \"A\", \"x\": -500, \"y\": -500}, {\"id\": \"B\", \"x\": 300, \"y\": 0}";
    let edges = "{\"id\": \"e\", \"route\": [[-400, -450], [300, 50]]}";
    let layout = Layout::parse(layout_with(nodes, edges).as_bytes()).unwrap();
    assert!(layout.check(&graph).is_empty());
    let text = layout.to_json();
    assert_eq!(Layout::parse(text.as_bytes()).unwrap().to_json(), text);
}

// --------------------------------------------------------- documentation

fn doc() -> String {
    doc_named("exchange-format.md")
}

fn doc_named(name: &str) -> String {
    let path = format!("{}/../../docs/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} : {e}"))
}

/// The code families of the exchange format, its imports, its engines and
/// its rendering, documented in `exchange-format.md`.
const EXCHANGE: &str = "JSGLXEMRV";
/// The code family of the source format, documented in `source-format.md`.
const SOURCE: &str = "T";
/// Every family.
const ALL: &str = "JSGLXEMRVT";

/// The rows of the tables of a documentation file.
fn table_rows(doc: &str) -> String {
    doc.lines()
        .filter(|line| line.starts_with("| "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The codes cited in a text: a letter among `letters`, then three digits.
fn codes_in(text: &str, quoted: bool, letters: &str) -> std::collections::BTreeSet<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = std::collections::BTreeSet::new();
    for window in chars.windows(if quoted { 6 } else { 4 }) {
        let code = if quoted {
            // Between quotes: the code alone, or the start of a key with a variant (`"J005.zero"`).
            if window[0] != '"' || !(window[5] == '"' || window[5] == '.') {
                continue;
            }
            &window[1..5]
        } else {
            window
        };
        if letters.contains(code[0]) && code[1..].iter().all(char::is_ascii_digit) {
            found.insert(code.iter().collect());
        }
    }
    found
}

#[test]
fn the_documentation_shows_the_example_files_as_they_are() {
    let doc = doc();
    for name in [
        "gestpret.graph.json",
        "gestpret.layout.json",
        "components.graph.json",
        "components.layout.json",
    ] {
        let file = String::from_utf8(data(name)).unwrap();
        assert!(
            doc.contains(&format!("```json\n{file}```")),
            "la documentation ne reprend plus {name} tel quel"
        );
    }
}

/// What separates, in a source file, the code from its tests.
const MARK: &str = concat!("#[cfg", "(test)]");

#[test]
fn the_documentation_lists_every_code_and_no_other() {
    // The codes the product can utter are those of the catalogue.
    let keys: Vec<&str> = foregram_graph::Catalog::english().keys().collect();
    let produced = codes_in(&keys.join("\n"), false, EXCHANGE);
    // Every code a table of the document cites, whatever its family: a
    // code of the source format listed here would be one too many.
    let documented = codes_in(&table_rows(&doc()), false, ALL);
    assert_eq!(produced, documented);
    assert_eq!(produced.len(), 84);

    // Each refusal the code can utter is provoked by at least one test:
    // those of this file, and those that follow the code in json.rs, in the
    // GraphML, ELK and MCD imports of the bench, in its reference engines and
    // in the rendering.
    let mut tests = String::new();
    for source in [
        "src/json.rs",
        "../foregram-bench/src/graphml.rs",
        "../foregram-bench/src/elkt.rs",
        "../foregram-bench/src/elk.rs",
        "../foregram-bench/src/mcd.rs",
        "../foregram-bench/src/engine/mod.rs",
        "../foregram-bench/src/engine/dot.rs",
        "../foregram-bench/src/engine/elk.rs",
        "../foregram-render/src/raster.rs",
        "../foregram-render/tests/render.rs",
    ] {
        let path = format!("{}/{source}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} : {e}"));
        tests.push_str(text.split(MARK).last().unwrap());
    }
    for name in ["formats.rs", "v2.rs", "common/mod.rs"] {
        let here = format!("{}/tests/{name}", env!("CARGO_MANIFEST_DIR"));
        tests.push_str(&std::fs::read_to_string(&here).unwrap_or_else(|e| panic!("{here} : {e}")));
    }
    let tested = codes_in(&tests, true, EXCHANGE);
    let untested: Vec<&String> = produced.difference(&tested).collect();
    assert!(untested.is_empty(), "codes sans test : {untested:?}");
}

#[test]
fn the_source_format_documentation_lists_every_code_and_no_other() {
    let keys: Vec<&str> = foregram_graph::Catalog::english().keys().collect();
    let produced = codes_in(&keys.join("\n"), false, SOURCE);
    let documented = codes_in(&table_rows(&doc_named("source-format.md")), false, ALL);
    assert_eq!(produced, documented);
    assert_eq!(produced.len(), 15);

    // Each one is provoked by at least one test of the source crate.
    let mut tests = String::new();
    for name in ["read.rs", "index.rs", "canon.rs"] {
        let path = format!(
            "{}/../foregram-source/tests/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        tests.push_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} : {e}")));
    }
    let tested = codes_in(&tests, true, SOURCE);
    let untested: Vec<&String> = produced.difference(&tested).collect();
    assert!(untested.is_empty(), "codes sans test : {untested:?}");
}
