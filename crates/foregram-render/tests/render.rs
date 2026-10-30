// ForegramPM — tests of the neutral rendering: SVG frozen to the byte, PNG read back.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop dead: the product's rules do not apply here.
#![allow(clippy::unwrap_used, clippy::panic)]

use foregram_graph::{Graph, Layout};
use foregram_render::{Class, PIXELS_MAX, SCALE_MAX, Scene, Shape, Style, png, svg};

/// An example file of the graph library.
fn graph_data(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../foregram-graph/tests/data/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path} : {e}"))
}

/// A golden file of this folder.
fn golden(name: &str) -> String {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} : {e}"))
}

fn example(name: &str) -> (Graph, Layout) {
    let graph = Graph::parse(&graph_data(&format!("{name}.graph.json"))).unwrap();
    let layout = Layout::parse(&graph_data(&format!("{name}.layout.json"))).unwrap();
    assert!(layout.check(&graph).is_empty());
    (graph, layout)
}

/// Decodes a PNG: dimensions and grey pixels.
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = ::png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buffer).unwrap();
    assert_eq!(info.color_type, ::png::ColorType::Grayscale);
    assert_eq!(info.bit_depth, ::png::BitDepth::Eight);
    buffer.truncate(info.buffer_size());
    (info.width, info.height, buffer)
}

#[test]
fn the_examples_render_to_the_golden_svg_byte_for_byte() {
    for name in ["components", "gestpret"] {
        let (graph, layout) = example(name);
        let text = svg(&graph, &layout, &Style::default()).unwrap();
        assert_eq!(text, golden(&format!("{name}.svg")), "{name}");
        assert_eq!(text, svg(&graph, &layout, &Style::default()).unwrap());
    }
}

#[test]
fn the_svg_carries_the_identifiers_and_no_number_has_a_float_shape() {
    let (graph, layout) = example("components");
    let text = svg(&graph, &layout, &Style::default()).unwrap();
    for id in [
        "Client", "System", "Core", "Store", "e1", "e2", "e3", "req", "in", "out",
    ] {
        assert!(text.contains(&format!("<title>{id}</title>")), "{id}");
    }
    assert!(text.contains("<title>components · hand 1</title>"));
    let body = text.split_once("?>\n").unwrap().1;
    assert!(!body.contains(".0\""), "un entier s'écrit sans décimale");
}

#[test]
fn the_scene_draws_in_a_fixed_order() {
    let (graph, layout) = example("components");
    let scene = Scene::build(&graph, &layout, &Style::default()).unwrap();
    let classes: Vec<Class> = scene.items.iter().map(|i| i.class).collect();
    assert_eq!(classes[0], Class::Sheet);
    assert_eq!(
        &classes[1..5],
        [Class::Leaf, Class::Container, Class::Leaf, Class::Leaf]
    );
    let first_port = classes.iter().position(|c| *c == Class::Port).unwrap();
    assert!(
        classes[first_port..].iter().all(|c| *c == Class::Port),
        "les ports sont dessinés en dernier"
    );
    let last_edge = classes.iter().rposition(|c| *c == Class::Arrow).unwrap();
    let first_label = classes.iter().position(|c| *c == Class::Label).unwrap();
    assert!(last_edge < first_label, "les étiquettes après les liens");
    assert_eq!(
        classes.iter().filter(|c| **c == Class::Arrow).count(),
        graph.edges.len()
    );
    // The window: the bounding box plus the margin.
    assert_eq!((scene.view.x, scene.view.y), (-1000, -1000));
    assert_eq!((scene.view.width, scene.view.height), (50000, 12000));
    // A port without a box is a square of the style, centred on its anchor.
    let req = scene
        .items
        .iter()
        .find(|i| i.title.as_deref() == Some("req"))
        .unwrap();
    assert_eq!(
        req.shape,
        Shape::Rect(foregram_graph::Rect {
            x: 9800,
            y: 4800,
            width: 400,
            height: 400
        })
    );
}

#[test]
fn the_png_has_the_size_of_the_view_and_the_leaf_grey_inside_a_leaf() {
    let (graph, layout) = example("components");
    let style = Style::default();
    let bytes = png(&graph, &layout, &style, 1).unwrap();
    let (width, height, pixels) = decode(&bytes);
    assert_eq!((width, height), (500, 120));
    let at = |x: u32, y: u32| pixels[usize::try_from(y * width + x).unwrap()];
    // The window starts at (-10, -10): pixel (60, 60) is at the heart of
    // `Client`, pixel (5, 5) in the margin, pixel (330, 95) in the
    // container `System` but outside its leaves.
    assert_eq!(at(60, 60), style.leaf_fill);
    assert_eq!(at(5, 5), style.sheet);
    assert_eq!(at(330, 95), style.sheet);
    // The edge of `Client`, at x = 0 of the drawing, hence x = 10 of the image, is
    // a 1 px stroke centred on the outline: two half-covered pixels.
    assert!(at(9, 60) < style.sheet && at(10, 60) < style.leaf_fill);
    // The port `req`, a 4 px square centred on (100, 50) of the drawing, is
    // hollow: its core has the sheet background, over the link and the
    // edge of `Client`, and its edge is dark.
    assert_eq!(at(110, 60), style.sheet);
    assert!(at(108, 60) < style.sheet);
    assert_eq!(bytes, png(&graph, &layout, &style, 1).unwrap());

    let (width, height, _) = decode(&png(&graph, &layout, &style, 3).unwrap());
    assert_eq!((width, height), (1500, 360));
}

fn empty() -> (Graph, Layout) {
    let graph = Graph::parse(
        br#"{"format": "foregram-graph", "version": 2, "name": "void", "nodes": [], "edges": []}"#,
    )
    .unwrap();
    let layout = Layout::parse(
        br#"{"format": "foregram-layout", "version": 2, "graph": "void",
            "engine": {"name": "none", "version": "0"}, "nodes": [], "edges": []}"#,
    )
    .unwrap();
    (graph, layout)
}

#[test]
fn an_empty_layout_renders_as_a_margin() {
    let (graph, layout) = empty();
    let text = svg(&graph, &layout, &Style::default()).unwrap();
    assert!(text.contains("viewBox=\"-10 -10 20 20\""));
    let (width, height, pixels) = decode(&png(&graph, &layout, &Style::default(), 1).unwrap());
    assert_eq!((width, height), (20, 20));
    assert!(pixels.iter().all(|p| *p == 255));
}

#[test]
fn the_scale_and_the_image_size_are_bounded() {
    let (graph, layout) = example("gestpret");
    let style = Style::default();
    for scale in [0, -1, SCALE_MAX + 1] {
        let diag = png(&graph, &layout, &style, scale).unwrap_err();
        assert_eq!(diag.code, "DRW001", "{scale}");
    }
    let (graph, layout) = empty();
    let (width, height, _) = decode(&png(&graph, &layout, &style, SCALE_MAX).unwrap());
    assert_eq!((width, height), (160, 160));

    let graph = Graph::parse(
        br#"{"format": "foregram-graph", "version": 2, "name": "huge",
            "nodes": [{"id": "a", "width": 900000000, "height": 900000000}], "edges": []}"#,
    )
    .unwrap();
    let layout = Layout::parse(
        br#"{"format": "foregram-layout", "version": 2, "graph": "huge",
            "engine": {"name": "none", "version": "0"},
            "nodes": [{"id": "a", "x": 0, "y": 0}], "edges": []}"#,
    )
    .unwrap();
    let diag = png(&graph, &layout, &style, 1).unwrap_err();
    assert_eq!(diag.code, "DRW002");
    assert!(diag.text().contains(&PIXELS_MAX.to_string()));
}

#[test]
fn the_view_grows_with_a_protruding_route_and_label() {
    let graph = Graph::parse(
        br#"{"format": "foregram-graph", "version": 2, "name": "loop",
            "nodes": [{"id": "a", "width": 10000, "height": 10000}],
            "edges": [{"id": "e", "source": "a", "target": "a",
                       "labels": [{"width": 2000, "height": 1000, "at": "center"}]}]}"#,
    )
    .unwrap();
    let layout = Layout::parse(
        br#"{"format": "foregram-layout", "version": 2, "graph": "loop",
            "engine": {"name": "none", "version": "0"},
            "nodes": [{"id": "a", "x": 0, "y": 0}],
            "edges": [{"id": "e", "route": [[10000, 5000], [30000, 5000], [30000, 12000], [5000, 12000], [5000, 10000]],
                       "labels": [[40000, -6000]]}]}"#,
    )
    .unwrap();
    assert!(layout.check(&graph).is_empty());
    let scene = Scene::build(&graph, &layout, &Style::default()).unwrap();
    // From (0, -60) to (420, 120) in pixels — the route goes down to 120 —, plus the margin.
    assert_eq!((scene.view.x, scene.view.y), (-1000, -7000));
    assert_eq!((scene.view.width, scene.view.height), (44000, 20000));
}

/// The example with ports, and the same layout with one place taken away:
/// a node (its identifier given), a port, an edge.
fn unplaced() -> (Graph, String, Layout, Layout, Layout) {
    let (graph, layout) = example("components");
    let mut no_node = layout.clone();
    let gone = no_node.nodes.remove(0).id;
    let mut no_port = layout.clone();
    let with_ports = no_port
        .nodes
        .iter_mut()
        .find(|n| !n.ports.is_empty())
        .unwrap();
    with_ports.ports.pop();
    let mut no_edge = layout.clone();
    no_edge.edges.pop();
    (graph, gone, no_node, no_port, no_edge)
}

#[test]
fn a_layout_taken_as_checked_but_leaving_something_unplaced_is_an_internal_error() {
    let (graph, gone, no_node, no_port, no_edge) = unplaced();
    let style = Style::default();
    // Each one would have been refused by `check`: the scene is asked anyway.
    assert!(!no_node.check(&graph).is_empty());
    let fault = Scene::build(&graph, &no_node, &style).unwrap_err();
    assert_eq!(fault.code, "INT001");
    assert!(fault.pos.is_none());
    assert_eq!(
        fault.text(),
        format!(
            "internal error, please report it: node \"{gone}\" has no place in a layout taken as checked"
        )
    );
    let fault = Scene::build(&graph, &no_port, &style).unwrap_err();
    assert_eq!(fault.code, "INT002");
    let fault = Scene::build(&graph, &no_edge, &style).unwrap_err();
    assert_eq!(fault.code, "INT003");
    // Both outputs carry the fault out instead of drawing with a hole.
    assert_eq!(svg(&graph, &no_edge, &style).unwrap_err().code, "INT003");
    assert_eq!(png(&graph, &no_node, &style, 1).unwrap_err().code, "INT001");
}
