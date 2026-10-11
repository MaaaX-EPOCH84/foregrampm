// ForegramPM — tests du rendu neutre : SVG figé à l'octet, PNG relu.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// Un test qui échoue doit s'arrêter net : les règles du produit ne valent pas ici.
#![allow(clippy::unwrap_used, clippy::panic)]

use foregram_graph::{Graph, Layout};
use foregram_render::{Class, PIXELS_MAX, SCALE_MAX, Scene, Shape, Style, png, svg};

/// Un fichier d'exemple de la bibliothèque du graphe.
fn graph_data(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../foregram-graph/tests/data/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path} : {e}"))
}

/// Un fichier d'or de ce dossier.
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

/// Décode un PNG : dimensions et pixels gris.
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
        let text = svg(&graph, &layout, &Style::default());
        assert_eq!(text, golden(&format!("{name}.svg")), "{name}");
        assert_eq!(text, svg(&graph, &layout, &Style::default()));
    }
}

#[test]
fn the_svg_carries_the_identifiers_and_no_number_has_a_float_shape() {
    let (graph, layout) = example("components");
    let text = svg(&graph, &layout, &Style::default());
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
    let scene = Scene::build(&graph, &layout, &Style::default());
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
    // La fenêtre : la boîte englobante plus la marge.
    assert_eq!((scene.view.x, scene.view.y), (-1000, -1000));
    assert_eq!((scene.view.width, scene.view.height), (50000, 12000));
    // Un port sans boîte est un carré du style, centré sur son ancre.
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
    // La fenêtre commence à (-10, -10) : le pixel (60, 60) est au cœur de
    // `Client`, le pixel (5, 5) dans la marge, le pixel (330, 95) dans le
    // conteneur `System` mais hors de ses feuilles.
    assert_eq!(at(60, 60), style.leaf_fill);
    assert_eq!(at(5, 5), style.sheet);
    assert_eq!(at(330, 95), style.sheet);
    // Le bord de `Client`, à x = 0 du dessin, donc x = 10 de l'image, est
    // un trait de 1 px centré sur le contour : deux pixels à moitié.
    assert!(at(9, 60) < style.sheet && at(10, 60) < style.leaf_fill);
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
    let text = svg(&graph, &layout, &Style::default());
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
        assert_eq!(diag.code, "V001", "{scale}");
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
    assert_eq!(diag.code, "V002");
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
    let scene = Scene::build(&graph, &layout, &Style::default());
    // De (0, -60) à (420, 120) en pixels — le tracé descend à 120 —, plus la marge.
    assert_eq!((scene.view.x, scene.view.y), (-1000, -7000));
    assert_eq!((scene.view.width, scene.view.height), (44000, 20000));
}
