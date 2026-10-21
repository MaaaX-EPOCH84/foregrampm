// ForegramPM — tests of the measures of a layout, on cases with known values.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop dead: the product's rules do not apply here.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::{LEAF_A, LEAF_B, data, graph, layout};
use foregram_graph::{CURVE_STEPS, Catalog, Graph, Layout, Metrics, json};

/// Measures a graph and a layout written in JSON, after validating them
/// against each other.
fn measure(nodes: &str, edges: &str, placed: &str, routed: &str) -> Metrics {
    let graph = Graph::parse(graph(nodes, edges).as_bytes()).unwrap();
    let layout = Layout::parse(layout(placed, routed).as_bytes()).unwrap();
    assert_eq!(layout.check(&graph), vec![]);
    Metrics::measure(&graph, &layout)
}

fn leaf(id: &str, w: i64, h: i64) -> String {
    format!("{{\"id\": \"{id}\", \"width\": {w}, \"height\": {h}}}")
}

fn at(id: &str, x: i64, y: i64) -> String {
    format!("{{\"id\": \"{id}\", \"x\": {x}, \"y\": {y}}}")
}

fn edge(id: &str, source: &str, target: &str) -> String {
    format!("{{\"id\": \"{id}\", \"source\": \"{source}\", \"target\": \"{target}\"}}")
}

fn route(id: &str, points: &str) -> String {
    format!("{{\"id\": \"{id}\", \"route\": {points}}}")
}

/// Four 100 × 100 leaves at the corners of a 1 000 square: A top
/// left, B bottom right, C top right, D bottom left.
fn corners() -> (String, String) {
    let nodes = [LEAF_A, LEAF_B, &leaf("C", 100, 100), &leaf("D", 100, 100)].join(", ");
    let placed = [
        at("A", 0, 0),
        at("B", 1000, 1000),
        at("C", 1000, 0),
        at("D", 0, 1000),
    ]
    .join(", ");
    (nodes, placed)
}

#[test]
fn an_empty_graph_measures_zero() {
    assert_eq!(measure("", "", "", ""), Metrics::default());
}

#[test]
fn a_straight_edge_has_its_euclidean_length_and_no_bend() {
    let (nodes, placed) = corners();
    // From (100, 100) to (400, 500): 3-4-5, hence 500.
    let m = measure(
        &nodes,
        &edge("e", "A", "B"),
        &placed,
        &route("e", "[[100, 100], [400, 500], [1000, 1000]]"),
    );
    // Then from (400, 500) to (1000, 1000): √610 000 = 781.02.
    let tail = 781;
    assert_eq!(m.edge_length, 500 + tail);
    assert_eq!(m.longest_edge, 500 + tail);
    assert_eq!(m.bends, 1);
    assert_eq!(m.crossings, 0);
}

#[test]
fn bends_count_direction_changes_between_straight_segments_only() {
    let (nodes, placed) = corners();
    let edges = [edge("e", "A", "B"), edge("f", "C", "D")].join(", ");
    let bends = |e: &str, f: &str| {
        measure(
            &nodes,
            &edges,
            &placed,
            &[route("e", e), route("f", f)].join(", "),
        )
        .bends
    };
    // An L, then a U; an aligned point does not count; a U-turn does.
    assert_eq!(
        bends(
            "[[100, 50], [500, 50], [1000, 1000]]",
            "[[1000, 50], [50, 1000]]"
        ),
        1
    );
    assert_eq!(
        bends(
            "[[100, 50], [500, 50], [500, 500], [1000, 1000]]",
            "[[1000, 50], [50, 1000]]"
        ),
        2
    );
    assert_eq!(
        bends(
            "[[100, 50], [500, 50], [800, 50], [1000, 1000]]",
            "[[1000, 50], [50, 1000]]"
        ),
        1
    );
    assert_eq!(
        bends(
            "[[100, 50], [500, 50], [300, 50], [1000, 1000]]",
            "[[1000, 50], [50, 1000]]"
        ),
        2
    );
    // A repeated point makes no vertex.
    assert_eq!(
        bends(
            "[[100, 50], [500, 50], [500, 50], [1000, 1000]]",
            "[[1000, 50], [50, 1000]]"
        ),
        1
    );
    // A curve, and its junction with a straight segment, make no bend.
    assert_eq!(
        bends(
            "[[100, 50], [[500, 50], [500, 500], [1000, 1000]]]",
            "[[1000, 50], [50, 1000]]"
        ),
        0
    );
    assert_eq!(
        bends(
            "[[100, 50], [500, 50], [[500, 500], [900, 500], [1000, 1000]]]",
            "[[1000, 50], [50, 1000]]"
        ),
        0
    );
}

#[test]
fn a_curve_is_flattened_in_sixteen_steps() {
    assert_eq!(CURVE_STEPS, 16);
    let (nodes, placed) = corners();
    // A curve whose control points lie on the chord has the
    // length of the chord, to within a hundredth at each step.
    let m = measure(
        &nodes,
        &edge("e", "A", "B"),
        &placed,
        &route("e", "[[100, 100], [[400, 400], [700, 700], [1000, 1000]]]"),
    );
    // √(2 × 900²) = 1 272.79.
    let chord = 1273;
    assert!(
        (m.edge_length - chord).abs() <= CURVE_STEPS,
        "{} vs {chord}",
        m.edge_length
    );
    // A bulging curve is longer than its chord and shorter than its
    // control polygon.
    let m = measure(
        &nodes,
        &edge("e", "A", "B"),
        &placed,
        &route(
            "e",
            "[[100, 100], [[100, 1000], [1000, 1000], [1000, 1000]]]",
        ),
    );
    assert!(m.edge_length > chord, "{}", m.edge_length);
    assert!(m.edge_length < 900 + 900, "{}", m.edge_length);
    assert_eq!(m.bends, 0);
}

/// Two links from A to B: their common ends never count, and what
/// happens between them is what the case builds.
fn pair_of(e: &str, f: &str) -> Metrics {
    let (nodes, placed) = corners();
    let edges = [edge("e", "A", "B"), edge("f", "A", "B")].join(", ");
    measure(
        &nodes,
        &edges,
        &placed,
        &[route("e", e), route("f", f)].join(", "),
    )
}

#[test]
fn two_edges_in_an_x_cross_once() {
    // `e` is the diagonal; `f` crosses it once, then twice.
    let diagonal = "[[100, 100], [1000, 1000]]";
    let m = pair_of(
        diagonal,
        "[[100, 50], [600, 50], [600, 1050], [1000, 1050]]",
    );
    assert_eq!((m.crossings, m.shared_segments), (1, 0));
    let m = pair_of(
        diagonal,
        "[[100, 50], [600, 50], [600, 700], [1050, 700], [1050, 1000]]",
    );
    assert_eq!(m.crossings, 2);
}

#[test]
fn a_common_end_is_not_a_crossing() {
    let diagonal = "[[100, 100], [1000, 1000]]";
    // Same start point, same end point.
    let m = pair_of(diagonal, "[[100, 100], [1050, 100], [1050, 1000]]");
    assert_eq!((m.crossings, m.shared_segments), (0, 0));
    // A link that ends on the other and leaves again on the same side.
    let m = pair_of(
        diagonal,
        "[[100, 50], [500, 500], [500, 50], [1050, 50], [1050, 1000]]",
    );
    assert_eq!(m.crossings, 0);
}

#[test]
fn a_route_through_a_vertex_of_the_other_crosses_or_touches() {
    // `e` makes a horizontal detour at y = 500; `f` bends on it and passes
    // to the other side: 1; comes back on the same side: 0.
    let detour = "[[100, 100], [100, 500], [1000, 500], [1000, 1000]]";
    let m = pair_of(
        detour,
        "[[100, 100], [500, 100], [500, 500], [800, 1000], [1000, 1000]]",
    );
    assert_eq!(m.crossings, 1);
    let m = pair_of(
        detour,
        "[[100, 100], [500, 100], [500, 500], [800, 100], [1100, 100], [1100, 1000], [1000, 1000]]",
    );
    assert_eq!(m.crossings, 0);
    // A vertex of `e` on a segment of `f`: `f` crosses it straight through.
    let peak = "[[100, 100], [500, 300], [1000, 1000]]";
    let m = pair_of(peak, "[[100, 50], [500, 50], [500, 1050], [1000, 1050]]");
    assert_eq!(m.crossings, 1);
    // Vertex on vertex: `f` folds on the tip of `e` without entering
    // the angle, then entering it.
    let m = pair_of(
        peak,
        "[[100, 50], [500, 50], [500, 300], [800, 50], [1100, 50], [1100, 1000], [1000, 1000]]",
    );
    assert_eq!(m.crossings, 0);
    let m = pair_of(
        peak,
        "[[100, 50], [500, 50], [500, 300], [700, 900], [1000, 1050]]",
    );
    assert_eq!(m.crossings, 1);
}

#[test]
fn collinear_stretches_are_shared_not_crossed() {
    let detour = "[[100, 100], [100, 500], [1000, 500], [1000, 1000]]";
    let m = pair_of(
        detour,
        "[[100, 50], [300, 50], [300, 500], [700, 500], [700, 1050], [1000, 1050]]",
    );
    assert_eq!(
        (m.crossings, m.shared_segments, m.shared_length),
        (0, 1, 400)
    );
    // Two pieces in the same direction, end to end, do not overlap.
    let m = pair_of(
        "[[100, 100], [100, 500], [500, 500], [1000, 1000]]",
        "[[100, 50], [500, 50], [500, 500], [900, 500], [1050, 500], [1050, 1000]]",
    );
    assert_eq!((m.crossings, m.shared_segments), (0, 0));
    // A vertical stretch, at x = 500, from 100 to 600: the boxes of the two
    // segments are flat and merely touch, they are compared
    // all the same. `f` stays on the same side of `e` before and after: 0.
    let m = pair_of(
        "[[100, 100], [500, 100], [500, 1000], [1000, 1000]]",
        "[[100, 50], [500, 50], [500, 600], [1050, 600], [1050, 1000]]",
    );
    assert_eq!(
        (m.crossings, m.shared_segments, m.shared_length),
        (0, 1, 500)
    );
}

#[test]
fn overlapping_nodes_are_counted_with_their_area() {
    let nodes = [LEAF_A, LEAF_B].join(", ");
    let m = measure(&nodes, "", &[at("A", 0, 0), at("B", 50, 50)].join(", "), "");
    assert_eq!((m.node_overlaps, m.node_overlap_area), (1, 2500));
    // Touching is not overlapping.
    let m = measure(&nodes, "", &[at("A", 0, 0), at("B", 100, 0)].join(", "), "");
    assert_eq!((m.node_overlaps, m.node_overlap_area), (0, 0));
    // Nor is a child inside its parent; two containers on top of
    // each other are.
    let nodes = "{\"id\": \"C\", \"children\": [{\"id\": \"A\", \"width\": 100, \"height\": 100}]}, \
                 {\"id\": \"D\", \"children\": [{\"id\": \"B\", \"width\": 100, \"height\": 100}]}";
    let placed = "{\"id\": \"C\", \"x\": 0, \"y\": 0, \"width\": 200, \"height\": 200}, \
                  {\"id\": \"A\", \"x\": 10, \"y\": 10}, \
                  {\"id\": \"D\", \"x\": 150, \"y\": 0, \"width\": 200, \"height\": 200}, \
                  {\"id\": \"B\", \"x\": 10, \"y\": 10}";
    // C on D: 50 × 200; B, child of D, on C: 40 × 100.
    let m = measure(nodes, "", placed, "");
    assert_eq!(
        (m.node_overlaps, m.node_overlap_area),
        (2, 50 * 200 + 40 * 100)
    );
}

#[test]
fn an_edge_through_a_foreign_node_is_counted_once_per_node() {
    let nodes = [LEAF_A, LEAF_B, &leaf("C", 100, 100)].join(", ");
    let placed = [at("A", 0, 0), at("B", 300, 0), at("C", 150, 0)].join(", ");
    let m = measure(
        &nodes,
        &edge("e", "A", "B"),
        &placed,
        &route("e", "[[100, 50], [300, 50]]"),
    );
    assert_eq!(m.edge_node_overlaps, 1);
    // Along the edge, or in two segments within the same node: nothing more.
    let m = measure(
        &nodes,
        &edge("e", "A", "B"),
        &placed,
        &route("e", "[[100, 0], [300, 0]]"),
    );
    assert_eq!(m.edge_node_overlaps, 0);
    let m = measure(
        &nodes,
        &edge("e", "A", "B"),
        &placed,
        &route("e", "[[100, 50], [200, 20], [300, 50]]"),
    );
    assert_eq!(m.edge_node_overlaps, 1);
    // A container that encloses one end of the link is not foreign.
    let nodes = "{\"id\": \"C\", \"children\": [{\"id\": \"A\", \"width\": 100, \"height\": 100}]}, \
                 {\"id\": \"B\", \"width\": 100, \"height\": 100}";
    let placed = "{\"id\": \"C\", \"x\": 0, \"y\": 0, \"width\": 200, \"height\": 200}, \
                  {\"id\": \"A\", \"x\": 10, \"y\": 10}, {\"id\": \"B\", \"x\": 300, \"y\": 0}";
    let m = measure(
        nodes,
        &edge("e", "A", "B"),
        placed,
        &route("e", "[[110, 60], [300, 50]]"),
    );
    assert_eq!(m.edge_node_overlaps, 0);
}

#[test]
fn labels_overlapping_anything_but_their_own_edge_are_counted() {
    let nodes = [LEAF_A, LEAF_B, &leaf("C", 100, 100)].join(", ");
    let placed = [at("A", 0, 0), at("B", 300, 0), at("C", 150, 200)].join(", ");
    let label = "{\"width\": 60, \"height\": 30, \"at\": \"center\"}";
    let labelled = |id: &str, s: &str, t: &str| {
        format!(
            "{{\"id\": \"{id}\", \"source\": \"{s}\", \"target\": \"{t}\", \"labels\": [{label}]}}"
        )
    };
    let placed_label = |id: &str, points: &str, x: i64, y: i64| {
        format!("{{\"id\": \"{id}\", \"route\": {points}, \"labels\": [[{x}, {y}]]}}")
    };
    // On its own link: nothing. On a node: 1.
    let e = labelled("e", "A", "B");
    let m = measure(
        &nodes,
        &e,
        &placed,
        &placed_label("e", "[[100, 50], [300, 50]]", 170, 40),
    );
    assert_eq!(m.label_overlaps, 0);
    let m = measure(
        &nodes,
        &e,
        &placed,
        &placed_label("e", "[[100, 50], [300, 50]]", 170, 190),
    );
    assert_eq!(m.label_overlaps, 1);
    // On the other link, and on its label: 1 each.
    let edges = [e.clone(), labelled("f", "C", "B")].join(", ");
    let m = measure(
        &nodes,
        &edges,
        &placed,
        &[
            placed_label("e", "[[100, 50], [300, 50]]", 280, 120),
            placed_label("f", "[[250, 200], [350, 100]]", 500, 500),
        ]
        .join(", "),
    );
    assert_eq!(m.label_overlaps, 1);
    let m = measure(
        &nodes,
        &edges,
        &placed,
        &[
            placed_label("e", "[[100, 50], [300, 50]]", 500, 500),
            placed_label("f", "[[250, 200], [350, 100]]", 540, 520),
        ]
        .join(", "),
    );
    assert_eq!(m.label_overlaps, 1);
    // Inside the container of its link: nothing.
    let nested = "{\"id\": \"D\", \"children\": [{\"id\": \"A\", \"width\": 100, \"height\": 100}, \
                  {\"id\": \"B\", \"width\": 100, \"height\": 100}]}";
    let inside = "{\"id\": \"D\", \"x\": 0, \"y\": 0, \"width\": 500, \"height\": 300}, \
                  {\"id\": \"A\", \"x\": 10, \"y\": 10}, {\"id\": \"B\", \"x\": 300, \"y\": 10}";
    let m = measure(
        nested,
        &e,
        inside,
        &placed_label("e", "[[110, 60], [300, 60]]", 170, 200),
    );
    assert_eq!(m.label_overlaps, 0);
}

#[test]
fn the_extent_covers_nodes_routes_and_labels() {
    let nodes = [LEAF_A, LEAF_B].join(", ");
    let placed = [at("A", 0, 0), at("B", 200, 100)].join(", ");
    let m = measure(&nodes, "", &placed, "");
    assert_eq!((m.width, m.height, m.area), (300, 200, 60_000));
    assert_eq!(m.proportion, 1500);
    assert_eq!(m.density, 333);
    // A route that overflows enlarges the box; so does a label.
    let m = measure(
        &nodes,
        &edge("e", "A", "B"),
        &placed,
        &route("e", "[[50, 100], [50, 250], [200, 150]]"),
    );
    assert_eq!((m.width, m.height), (300, 250));
    let m = measure(
        &nodes,
        "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\", \
          \"labels\": [{\"width\": 40, \"height\": 20, \"at\": \"center\"}]}",
        &placed,
        "{\"id\": \"e\", \"route\": [[100, 50], [200, 150]], \"labels\": [[-100, 50]]}",
    );
    assert_eq!((m.width, m.height), (400, 200));
    // Thousandths round to nearest: 200/300 = 666.67.
    let m = measure(
        &nodes,
        "",
        &[at("A", 0, 0), at("B", 100, 200)].join(", "),
        "",
    );
    assert_eq!((m.proportion, m.density), (667, 333));
    // A single node: ratio 1 000, density 1 000.
    let m = measure(LEAF_A, "", &at("A", 0, 0), "");
    assert_eq!((m.proportion, m.density), (1000, 1000));
}

#[test]
fn broken_layer_and_order_constraints_are_counted_by_pair() {
    let layered = |id: &str, layer: i64, order: i64| {
        format!(
            "{{\"id\": \"{id}\", \"width\": 100, \"height\": 100, \"layer\": {layer}, \"order\": {order}}}"
        )
    };
    let nodes = [layered("A", 0, 0), layered("B", 1, 0), layered("C", 1, 1)].join(", ");
    // Direction `right` by default: B and C after A, C below B.
    let m = measure(
        &nodes,
        "",
        &[at("A", 0, 0), at("B", 200, 0), at("C", 200, 200)].join(", "),
        "",
    );
    assert_eq!((m.layer_violations, m.order_violations), (0, 0));
    // A after B and C, C above B: two layers and one order violated.
    let m = measure(
        &nodes,
        "",
        &[at("A", 400, 0), at("B", 200, 200), at("C", 200, 0)].join(", "),
        "",
    );
    assert_eq!((m.layer_violations, m.order_violations), (2, 1));
    // At the same height along the direction: violated too.
    let m = measure(
        &nodes,
        "",
        &[at("A", 0, 0), at("B", 0, 200), at("C", 200, 400)].join(", "),
        "",
    );
    assert_eq!((m.layer_violations, m.order_violations), (1, 0));
    // Direction `down`: layers follow y, order follows x.
    let down = graph(&nodes, "").replace(
        "\"name\": \"g\"",
        "\"name\": \"g\", \"direction\": \"down\"",
    );
    let graph = Graph::parse(down.as_bytes()).unwrap();
    let placed = layout(
        &[at("A", 0, 0), at("B", 200, 200), at("C", 0, 200)].join(", "),
        "",
    );
    let layout = Layout::parse(placed.as_bytes()).unwrap();
    let m = Metrics::measure(&graph, &layout);
    assert_eq!((m.layer_violations, m.order_violations), (0, 1));
}

#[test]
fn port_order_is_read_clockwise_on_each_side() {
    let node = |ports: &str| {
        format!(
            "{{\"id\": \"A\", \"width\": 100, \"height\": 100, \"ports_ordered\": true, \"ports\": [{ports}]}}"
        )
    };
    let placed =
        |ports: &str| format!("{{\"id\": \"A\", \"x\": 0, \"y\": 0, \"ports\": [{ports}]}}");
    let north = node("{\"id\": \"p\", \"side\": \"north\"}, {\"id\": \"q\", \"side\": \"north\"}");
    // On the north side, left to right.
    let m = measure(
        &north,
        "",
        &placed("{\"id\": \"p\", \"x\": 20, \"y\": 0}, {\"id\": \"q\", \"x\": 80, \"y\": 0}"),
        "",
    );
    assert_eq!(m.port_order_violations, 0);
    let m = measure(
        &north,
        "",
        &placed("{\"id\": \"p\", \"x\": 80, \"y\": 0}, {\"id\": \"q\", \"x\": 20, \"y\": 0}"),
        "",
    );
    assert_eq!(m.port_order_violations, 1);
    // On the west side, bottom to top; another side does not count.
    let west = node(
        "{\"id\": \"p\", \"side\": \"west\"}, {\"id\": \"q\", \"side\": \"west\"}, {\"id\": \"r\", \"side\": \"east\"}",
    );
    let m = measure(
        &west,
        "",
        &placed(
            "{\"id\": \"p\", \"x\": 0, \"y\": 80}, {\"id\": \"q\", \"x\": 0, \"y\": 20}, {\"id\": \"r\", \"x\": 100, \"y\": 90}",
        ),
        "",
    );
    assert_eq!(m.port_order_violations, 0);
    let m = measure(
        &west,
        "",
        &placed(
            "{\"id\": \"p\", \"x\": 0, \"y\": 20}, {\"id\": \"q\", \"x\": 0, \"y\": 80}, {\"id\": \"r\", \"x\": 100, \"y\": 90}",
        ),
        "",
    );
    assert_eq!(m.port_order_violations, 1);
    // On the south side, right to left.
    let south = node("{\"id\": \"p\", \"side\": \"south\"}, {\"id\": \"q\", \"side\": \"south\"}");
    let m = measure(
        &south,
        "",
        &placed("{\"id\": \"p\", \"x\": 80, \"y\": 100}, {\"id\": \"q\", \"x\": 20, \"y\": 100}"),
        "",
    );
    assert_eq!(m.port_order_violations, 0);
    let m = measure(
        &south,
        "",
        &placed("{\"id\": \"p\", \"x\": 20, \"y\": 100}, {\"id\": \"q\", \"x\": 80, \"y\": 100}"),
        "",
    );
    assert_eq!(m.port_order_violations, 1);
    // Without `ports_ordered`, nothing is imposed.
    let free = north.replace("\"ports_ordered\": true, ", "");
    let m = measure(
        &free,
        "",
        &placed("{\"id\": \"p\", \"x\": 80, \"y\": 0}, {\"id\": \"q\", \"x\": 20, \"y\": 0}"),
        "",
    );
    assert_eq!(m.port_order_violations, 0);
}

#[test]
fn the_example_files_measure_as_counted_by_hand() {
    let graph = Graph::parse(&data("components.graph.json")).unwrap();
    let layout = Layout::parse(&data("components.layout.json")).unwrap();
    let m = Metrics::measure(&graph, &layout);
    // e1: 3 500 + 1 000 + 3 500 in two bends; e2: 6 000; e3: a
    // curve between its chord (3 606) and its control polygon (4 032).
    assert_eq!(m.bends, 2);
    assert!(
        m.edge_length > 14_000 + 3_606 && m.edge_length < 14_000 + 4_032,
        "{}",
        m.edge_length
    );
    assert_eq!(m.longest_edge, 8000);
    assert_eq!((m.crossings, m.shared_segments, m.node_overlaps), (0, 0, 0));
    assert_eq!((m.edge_node_overlaps, m.label_overlaps), (0, 0));
    assert_eq!((m.width, m.height), (48_000, 10_000));
    // Leaves: 60 + 72 + 40 million over 480 million.
    assert_eq!(m.density, 358);
    assert_eq!(m.proportion, 4800);
    assert_eq!(m.port_order_violations, 0);
    assert_eq!(m, Metrics::measure(&graph, &layout));
}

#[test]
fn the_report_has_one_line_per_measure_from_the_catalogue() {
    let graph = Graph::parse(&data("components.graph.json")).unwrap();
    let layout = Layout::parse(&data("components.layout.json")).unwrap();
    let catalog = Catalog::english();
    let lines: Vec<String> = Metrics::measure(&graph, &layout)
        .report()
        .iter()
        .map(|m| {
            m.check(catalog).unwrap();
            m.render(catalog)
        })
        .collect();
    assert_eq!(lines.len(), 9);
    assert_eq!(lines[0], "crossings: 0");
    assert_eq!(lines[2], "bends: 2");
    assert_eq!(
        lines[7],
        "extent: 48000 x 10000, area 480000000, proportion 4800/1000, density 358/1000"
    );
    assert_eq!(
        lines[8],
        "constraints broken: layer 0, order 0, port order 0"
    );
}

#[test]
fn the_json_has_one_integer_key_per_measure_in_a_fixed_order() {
    let graph = Graph::parse(&data("components.graph.json")).unwrap();
    let layout = Layout::parse(&data("components.layout.json")).unwrap();
    let m = Metrics::measure(&graph, &layout);
    let text = m.to_json();
    assert!(!text.contains('\n'));
    // The values checked by hand for `components`, under the right keys.
    assert!(
        text.starts_with(
            "{\"crossings\": 0, \"shared_segments\": 0, \"shared_length\": 0, \"bends\": 2, "
        ),
        "{text}"
    );
    assert!(text.contains(", \"longest_edge\": 8000, "), "{text}");
    assert!(
        text.contains(", \"width\": 48000, \"height\": 10000, \"area\": 480000000, \"proportion\": 4800, \"density\": 358, "),
        "{text}"
    );
    assert!(text.ends_with(", \"port_order_violations\": 0}"), "{text}");
    // Read back by the strict reader: one object, each measure once, as an integer.
    let value = json::parse(text.as_bytes()).unwrap();
    let json::Kind::Object(members) = value.kind else {
        panic!("{text}");
    };
    let read: Vec<(String, i64)> = members
        .into_iter()
        .map(|member| match member.value.kind {
            json::Kind::Int(n) => (member.key, n),
            other => panic!("{other:?}"),
        })
        .collect();
    let fields: Vec<(String, i64)> = m
        .fields()
        .iter()
        .map(|(name, value)| ((*name).to_string(), *value))
        .collect();
    assert_eq!(read, fields);
    assert_eq!(read.len(), 18);
    assert_eq!(
        Metrics::default().fields().iter().map(|f| f.1).sum::<i64>(),
        0
    );
}

#[test]
fn the_documentation_states_the_curve_steps() {
    let doc = std::fs::read_to_string(format!(
        "{}/../../docs/exchange-format.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(doc.contains(&format!("{CURVE_STEPS} straight segments")));
}
