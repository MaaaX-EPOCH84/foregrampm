// ForegramPM — tests des mesures d'un placement, sur des cas à valeurs connues.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// Un test qui échoue doit s'arrêter net : les règles du produit ne valent pas ici.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::{LEAF_A, LEAF_B, data, graph, layout};
use foregram_graph::{CURVE_STEPS, Catalog, Graph, Layout, Metrics, json};

/// Mesure un graphe et un placement écrits en JSON, après les avoir
/// validés l'un pour l'autre.
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

/// Quatre feuilles de 100 × 100 aux coins d'un carré de 1 000 : A en haut
/// à gauche, B en bas à droite, C en haut à droite, D en bas à gauche.
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
    // De (100, 100) à (400, 500) : 3-4-5, donc 500.
    let m = measure(
        &nodes,
        &edge("e", "A", "B"),
        &placed,
        &route("e", "[[100, 100], [400, 500], [1000, 1000]]"),
    );
    // Puis de (400, 500) à (1000, 1000) : √610 000 = 781,02.
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
    // Un L, puis un U ; un point aligné ne compte pas ; un demi-tour compte.
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
    // Un point répété ne fait pas de sommet.
    assert_eq!(
        bends(
            "[[100, 50], [500, 50], [500, 50], [1000, 1000]]",
            "[[1000, 50], [50, 1000]]"
        ),
        1
    );
    // Une courbe, et sa jonction avec un segment droit, ne font aucun coude.
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
    // Une courbe dont les points de contrôle sont sur la corde a la
    // longueur de la corde, au centième près à chaque pas.
    let m = measure(
        &nodes,
        &edge("e", "A", "B"),
        &placed,
        &route("e", "[[100, 100], [[400, 400], [700, 700], [1000, 1000]]]"),
    );
    // √(2 × 900²) = 1 272,79.
    let chord = 1273;
    assert!(
        (m.edge_length - chord).abs() <= CURVE_STEPS,
        "{} vs {chord}",
        m.edge_length
    );
    // Une courbe bombée est plus longue que sa corde et plus courte que son
    // polygone de contrôle.
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

/// Deux liens d'A à B : leurs bouts communs ne comptent jamais, et ce qui
/// se passe entre les deux est ce que le cas construit.
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
    // `e` est la diagonale ; `f` la traverse une fois, puis deux.
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
    // Même point de départ, même point d'arrivée.
    let m = pair_of(diagonal, "[[100, 100], [1050, 100], [1050, 1000]]");
    assert_eq!((m.crossings, m.shared_segments), (0, 0));
    // Un lien qui aboutit sur l'autre et repart du même côté.
    let m = pair_of(
        diagonal,
        "[[100, 50], [500, 500], [500, 50], [1050, 50], [1050, 1000]]",
    );
    assert_eq!(m.crossings, 0);
}

#[test]
fn a_route_through_a_vertex_of_the_other_crosses_or_touches() {
    // `e` fait un détour horizontal à y = 500 ; `f` coude dessus et passe
    // de l'autre côté : 1 ; revient du même côté : 0.
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
    // Un sommet de `e` sur un segment de `f` : `f` le traverse tout droit.
    let peak = "[[100, 100], [500, 300], [1000, 1000]]";
    let m = pair_of(peak, "[[100, 50], [500, 50], [500, 1050], [1000, 1050]]");
    assert_eq!(m.crossings, 1);
    // Sommet sur sommet : `f` plie sur la pointe de `e` sans entrer dans
    // l'angle, puis en y entrant.
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
    // Deux morceaux dans le même sens, bout à bout, ne se recouvrent pas.
    let m = pair_of(
        "[[100, 100], [100, 500], [500, 500], [1000, 1000]]",
        "[[100, 50], [500, 50], [500, 500], [900, 500], [1050, 500], [1050, 1000]]",
    );
    assert_eq!((m.crossings, m.shared_segments), (0, 0));
    // Un tronçon vertical, en x = 500, de 100 à 600 : les boîtes des deux
    // segments sont plates et ne font que se toucher, elles sont confrontées
    // quand même. `f` reste du même côté de `e` avant et après : 0.
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
    // Se toucher n'est pas se recouvrir.
    let m = measure(&nodes, "", &[at("A", 0, 0), at("B", 100, 0)].join(", "), "");
    assert_eq!((m.node_overlaps, m.node_overlap_area), (0, 0));
    // Un enfant dans son parent, non plus ; deux conteneurs l'un sur
    // l'autre, si.
    let nodes = "{\"id\": \"C\", \"children\": [{\"id\": \"A\", \"width\": 100, \"height\": 100}]}, \
                 {\"id\": \"D\", \"children\": [{\"id\": \"B\", \"width\": 100, \"height\": 100}]}";
    let placed = "{\"id\": \"C\", \"x\": 0, \"y\": 0, \"width\": 200, \"height\": 200}, \
                  {\"id\": \"A\", \"x\": 10, \"y\": 10}, \
                  {\"id\": \"D\", \"x\": 150, \"y\": 0, \"width\": 200, \"height\": 200}, \
                  {\"id\": \"B\", \"x\": 10, \"y\": 10}";
    // C sur D : 50 × 200 ; B, enfant de D, sur C : 40 × 100.
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
    // Le long du bord, ou en deux segments dans le même nœud : rien de plus.
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
    // Un conteneur qui enferme un bout du lien n'est pas étranger.
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
    // Sur son propre lien : rien. Sur un nœud : 1.
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
    // Sur l'autre lien, et sur son étiquette : 1 chacun.
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
    // Dans le conteneur de son lien : rien.
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
    // Un tracé qui déborde agrandit la boîte ; une étiquette aussi.
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
    // Les millièmes s'arrondissent au plus près : 200/300 = 666,67.
    let m = measure(
        &nodes,
        "",
        &[at("A", 0, 0), at("B", 100, 200)].join(", "),
        "",
    );
    assert_eq!((m.proportion, m.density), (667, 333));
    // Un seul nœud : proportion 1 000, densité 1 000.
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
    // Direction `right` par défaut : B et C après A, C sous B.
    let m = measure(
        &nodes,
        "",
        &[at("A", 0, 0), at("B", 200, 0), at("C", 200, 200)].join(", "),
        "",
    );
    assert_eq!((m.layer_violations, m.order_violations), (0, 0));
    // A après B et C, C au-dessus de B : deux couches et un ordre violés.
    let m = measure(
        &nodes,
        "",
        &[at("A", 400, 0), at("B", 200, 200), at("C", 200, 0)].join(", "),
        "",
    );
    assert_eq!((m.layer_violations, m.order_violations), (2, 1));
    // À la même hauteur le long de la direction : violé aussi.
    let m = measure(
        &nodes,
        "",
        &[at("A", 0, 0), at("B", 0, 200), at("C", 200, 400)].join(", "),
        "",
    );
    assert_eq!((m.layer_violations, m.order_violations), (1, 0));
    // Direction `down` : les couches suivent y, l'ordre suit x.
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
    // Au nord, de gauche à droite.
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
    // À l'ouest, de bas en haut ; un autre côté ne compte pas.
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
    // Au sud, de droite à gauche.
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
    // Sans `ports_ordered`, rien n'est imposé.
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
    // e1 : 3 500 + 1 000 + 3 500 en deux coudes ; e2 : 6 000 ; e3 : une
    // courbe entre sa corde (3 606) et son polygone de contrôle (4 032).
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
    // Feuilles : 60 + 72 + 40 millions sur 480 millions.
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
    // Les valeurs vérifiées à la main pour `components`, aux bonnes clés.
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
    // Relu par le lecteur strict : un objet, chaque mesure une fois, en entier.
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
