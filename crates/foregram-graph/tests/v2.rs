// ForegramPM — tests of version 2 of the format: ports, nesting, curves.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::*;
use foregram_graph::{
    Diagnostic, Edge, Graph, Layout, Node, NodePlace, Point, Port, PortPlace, Pos, Segment, Side,
    Size,
};

fn keys(diags: &[Diagnostic]) -> Vec<&'static str> {
    diags.iter().map(|d| d.message.key).collect()
}

fn case(name: &str) -> Vec<Diagnostic> {
    cases()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, d)| d)
        .unwrap_or_else(|| panic!("cas inconnu : {name}"))
}

// ------------------------------------------------------------ the example

#[test]
fn the_components_example_is_valid_and_canonical() {
    let bytes = data("components.graph.json");
    let graph = Graph::parse(&bytes).unwrap();
    assert_eq!(graph.to_json().as_bytes(), bytes.as_slice());
    let bytes = data("components.layout.json");
    let layout = Layout::parse(&bytes).unwrap();
    assert_eq!(layout.to_json().as_bytes(), bytes.as_slice());
    assert!(layout.check(&graph).is_empty());

    // Four nodes including two children, three ports, a curved route.
    let all = graph.all_nodes();
    assert_eq!(all.len(), 4);
    let parents: Vec<Option<&str>> = all
        .iter()
        .map(|(_, parent)| parent.map(|p| p.id.as_str()))
        .collect();
    assert_eq!(parents, [None, None, Some("System"), Some("System")]);
    assert!(graph.node("System").unwrap().is_container());
    assert_eq!(graph.node("System").unwrap().size, None);
    let (owner, port) = graph.port("out").unwrap();
    assert_eq!(owner.id, "Core");
    assert_eq!(port.position, Some(Point { x: 12000, y: 3000 }));
    assert_eq!(
        port.size,
        Some(Size {
            width: 600,
            height: 600
        })
    );
    assert!(graph.port("nope").is_none());
    let e3 = &layout.edges[2];
    assert_eq!(e3.route.len(), 2);
    assert!(matches!(e3.route[1], Segment::Cubic { .. }));
    assert_eq!(e3.route[1].end(), Point { x: 41000, y: 10000 });
    assert_eq!(e3.route[1].points().len(), 3);
}

#[test]
fn a_version_1_file_still_reads_and_comes_back_as_version_2() {
    let v1 = String::from_utf8(data("gestpret.graph.json"))
        .unwrap()
        .replace("\"version\": 2", "\"version\": 1");
    let graph = Graph::parse(v1.as_bytes()).unwrap();
    assert!(graph.to_json().contains("\"version\": 2,"));
    assert_eq!(
        graph.to_json().as_bytes(),
        data("gestpret.graph.json").as_slice()
    );
}

#[test]
fn nested_nodes_are_written_one_per_line_with_deeper_indentation() {
    let graph = nested();
    let text = graph.to_json();
    let expected = "  \"nodes\": [\n    \
        {\"id\": \"C\", \"padding\": 10, \"children\": [\n      \
        {\"id\": \"A\", \"width\": 100, \"height\": 100, \"ports\": [{\"id\": \"p\", \"side\": \"east\", \"x\": 100, \"y\": 50}]}\n    \
        ]},\n    \
        {\"id\": \"B\", \"width\": 100, \"height\": 100, \"ports\": [{\"id\": \"q\", \"side\": \"west\", \"width\": 20, \"height\": 20}]}\n  \
        ],\n";
    assert!(text.contains(expected), "{text}");
    assert_eq!(Graph::parse(text.as_bytes()).unwrap().to_json(), text);
    let layout =
        Layout::parse(layout(NESTED_LAYOUT_NODES, NESTED_LAYOUT_EDGES).as_bytes()).unwrap();
    assert!(layout.check(&graph).is_empty());
    let text = layout.to_json();
    assert!(text.contains("\"route\": [[60, 110], [[60, 150], [80, 180], [100, 200]]]}"));
    assert_eq!(Layout::parse(text.as_bytes()).unwrap().to_json(), text);
}

// --------------------------------------------------------------- graph

#[test]
fn port_identifiers_are_unique_in_the_whole_graph() {
    assert_eq!(keys(&case("port-twice")), ["G006.again", "G006.again"]);
}

#[test]
fn an_edge_port_exists_belongs_to_its_node_and_excludes_a_side() {
    let diags = case("edge-port-unknown-or-foreign");
    assert_eq!(keys(&diags), ["G007.unknown", "G007.other", "G008"]);
    assert_eq!(
        diags[1].text(),
        "edge \"f\": `source_port` refers to port \"p\", which belongs to node \"A\", not to node \"B\""
    );
}

#[test]
fn ordered_ports_all_need_a_side() {
    assert_eq!(keys(&case("ordered-without-side")), ["G009"]);
}

#[test]
fn a_port_position_needs_a_side_and_sits_on_that_side() {
    assert_eq!(
        keys(&case("port-position")),
        ["G010.side", "G010.border", "S008", "G010.container"]
    );
}

#[test]
fn leaves_have_a_size_and_containers_do_not() {
    assert_eq!(
        keys(&case("sizes-and-padding")),
        ["G011.leaf", "G012", "G011.container"]
    );
}

#[test]
fn width_and_height_come_together_and_so_do_x_and_y() {
    assert_eq!(keys(&case("half-size")), ["S003", "S003", "S003", "S003"]);
    assert!(case("half-size")[3].text().contains("`y` missing (port)"));
}

#[test]
fn a_port_identifier_follows_the_identifier_rules() {
    let diags = case("bad-port-identifier");
    assert_eq!(keys(&diags), ["S007.empty"]);
    assert_eq!(diags[0].text(), "port #1: `id`: identifier empty");
}

#[test]
fn an_edge_may_join_a_node_to_its_own_ancestor() {
    // `f` goes from `A` to `C`, which contains it: valid in the graph and in the layout.
    let graph = nested();
    assert_eq!(graph.edges[1].target, "C");
    assert!(graph.validate().is_empty());
}

#[test]
fn a_graph_built_in_memory_reports_duplicate_ports_without_a_line() {
    let port = |id: &str| Port {
        id: id.into(),
        side: Some(Side::East),
        position: None,
        size: None,
        pos: Pos::NONE,
    };
    let graph = Graph {
        name: "m".into(),
        direction: None,
        nodes: vec![Node {
            id: "A".into(),
            size: Some(Size {
                width: 1,
                height: 1,
            }),
            layer: None,
            order: None,
            padding: None,
            ports_ordered: true,
            ports: vec![port("p"), port("p")],
            children: Vec::new(),
            pos: Pos::NONE,
        }],
        edges: vec![Edge {
            id: "e".into(),
            source: "A".into(),
            target: "A".into(),
            source_port: Some("p".into()),
            target_port: None,
            source_side: None,
            target_side: Some(Side::West),
            labels: Vec::new(),
            pos: Pos::NONE,
        }],
        pos: Pos::NONE,
    };
    let diags = graph.validate();
    assert_eq!(keys(&diags), ["G006"]);
    assert_eq!(
        diags[0].to_string(),
        "error[G006]: port identifier \"p\" already used"
    );
}

// ------------------------------------------------------------ layout

#[test]
fn a_layout_checks_its_own_ports_and_route_shapes() {
    assert_eq!(
        keys(&case("layout-ports-and-curves")),
        ["L014.twice-again", "L017"]
    );
    let diags = case("layout-half-size-and-label-point");
    assert_eq!(keys(&diags), ["S003", "S003", "S001.point"]);
    assert!(diags[1].text().contains("`y` missing (placed port)"));
}

#[test]
fn a_layout_built_in_memory_reports_duplicate_ports_without_a_line() {
    let port = PortPlace {
        id: "p".into(),
        x: 0,
        y: 0,
        pos: Pos::NONE,
    };
    let layout = Layout {
        graph: "m".into(),
        engine: foregram_graph::Engine {
            name: "m".into(),
            version: "1".into(),
            options: None,
        },
        nodes: vec![NodePlace {
            id: "A".into(),
            x: 0,
            y: 0,
            size: None,
            ports: vec![port.clone(), port],
            pos: Pos::NONE,
        }],
        edges: Vec::new(),
        pos: Pos::NONE,
    };
    assert_eq!(keys(&layout.validate()), ["L014.twice"]);
}

#[test]
fn children_fit_in_their_parent_and_sizes_match_the_kind_of_node() {
    let diags = case("check-nesting-and-ports");
    assert_eq!(
        keys(&diags),
        [
            "L012",
            "L013.leaf",
            "L015.fixed",
            "L014.unknown",
            "L014.missing",
            "L009.start"
        ]
    );
    assert_eq!(
        diags[2].text(),
        "port \"p\": anchored at (100, 40), the graph fixes it at (100, 50)"
    );
}

#[test]
fn a_container_needs_its_size_and_ports_sit_on_their_side() {
    assert_eq!(
        keys(&case("check-port-sides")),
        ["L007", "L007", "L013.container", "L015.side"]
    );
}

#[test]
fn a_route_with_a_port_touches_the_port() {
    assert_eq!(keys(&case("check-route-off-port-start")), ["L016.start"]);
    assert_eq!(
        keys(&case("check-port-interior")),
        ["L015.border", "L016.end"]
    );
    // The port `q` has a 20 × 20 box centred on its anchor: any point
    // of the box fits, the first point outside the box does not.
    let graph = nested();
    let check = |end: &str| {
        let edges = format!(
            "{{\"id\": \"e\", \"route\": [[110, 60], {end}]}},\n\
             {{\"id\": \"f\", \"route\": [[60, 110], [100, 200]]}}"
        );
        let layout = Layout::parse(layout(NESTED_LAYOUT_NODES, &edges).as_bytes()).unwrap();
        keys(&layout.check(&graph))
    };
    let none: [&str; 0] = [];
    assert_eq!(check("[300, 50]"), none);
    assert_eq!(check("[290, 40]"), none);
    assert_eq!(check("[310, 60]"), none);
    assert_eq!(check("[311, 60]"), ["L016.end"]);
    assert_eq!(check("[289, 50]"), ["L016.end"]);
}

#[test]
fn padding_counts_when_a_child_is_placed_against_the_border() {
    // `C` has a padding of 10: a child at (10, 10) fits, at (9, 10) does not.
    let graph = nested();
    let check = |a: &str| {
        let nodes = format!(
            "{{\"id\": \"C\", \"x\": 0, \"y\": 0, \"width\": 200, \"height\": 200}},\n\
             {{\"id\": \"A\", \"x\": {a}, \"ports\": [{{\"id\": \"p\", \"x\": 100, \"y\": 50}}]}},\n\
             {{\"id\": \"B\", \"x\": 300, \"y\": 0, \"ports\": [{{\"id\": \"q\", \"x\": 0, \"y\": 50}}]}}"
        );
        let edges = "{\"id\": \"e\", \"route\": [[300, 50], [300, 50]]}, \
                     {\"id\": \"f\", \"route\": [[0, 200], [0, 200]]}";
        let layout = Layout::parse(layout(&nodes, edges).as_bytes()).unwrap();
        keys(&layout.check(&graph))
            .into_iter()
            .filter(|k| k.starts_with("L012"))
            .collect::<Vec<_>>()
    };
    let none: [&str; 0] = [];
    assert_eq!(check("10, \"y\": 10"), none);
    assert_eq!(check("90, \"y\": 90"), none);
    assert_eq!(check("9, \"y\": 10"), ["L012"]);
    assert_eq!(check("91, \"y\": 10"), ["L012"]);
}

#[test]
fn nested_positions_are_relative_to_the_parent() {
    // `A` at (10, 10) in `C` at (0, 0): its port `p` is at (110, 60) in absolute terms.
    // Moving `C` moves `A` and `p`: the route must follow.
    let graph = nested();
    let nodes = NESTED_LAYOUT_NODES.replace(
        "\"id\": \"C\", \"x\": 0, \"y\": 0",
        "\"id\": \"C\", \"x\": 1000, \"y\": 0",
    );
    let placed = Layout::parse(layout(&nodes, NESTED_LAYOUT_EDGES).as_bytes()).unwrap();
    let diags = placed.check(&graph);
    assert_eq!(keys(&diags), ["L016.start", "L009.start", "L009.end"]);
    let edges = NESTED_LAYOUT_EDGES
        .replace("[110, 60]", "[1110, 60]")
        .replace("[60, 110]", "[1060, 110]")
        .replace("[100, 200]", "[1100, 200]");
    let placed = Layout::parse(layout(&nodes, &edges).as_bytes()).unwrap();
    assert!(placed.check(&graph).is_empty());
}
