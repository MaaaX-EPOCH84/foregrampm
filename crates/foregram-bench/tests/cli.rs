// ForegramPM — tests de la ligne de commande du banc.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// Un test qui échoue doit s'arrêter net : les règles du produit ne valent pas ici.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::process::{Command, Output};

fn data(name: &str) -> String {
    format!(
        "{}/../foregram-graph/tests/data/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn run(args: &[&str]) -> (i32, String, String) {
    let Output {
        status,
        stdout,
        stderr,
    } = Command::new(env!("CARGO_BIN_EXE_foregram-bench"))
        .args(args)
        .output()
        .unwrap();
    (
        status.code().unwrap(),
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

#[test]
fn a_valid_graph_and_its_layout_exit_zero() {
    let graph = data("gestpret.graph.json");
    let layout = data("gestpret.layout.json");
    let (code, out, err) = run(&["validate", &graph, &layout]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        format!(
            "{graph}: valid graph (3 nodes, 2 edges)\n\
             {layout}: valid layout of graph \"gestpret\" (engine main 1)\n"
        )
    );
    assert_eq!(err, "");
}

#[test]
fn a_refused_graph_exits_one_and_says_where() {
    let graph = data("refuse-lien-orphelin.graph.json");
    let (code, out, err) = run(&["validate", &graph]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert_eq!(
        err,
        format!(
            "{graph}:7:5: error[G001]: node identifier \"A\" already used (line 6)\n\
             {graph}:7:5: error[S008]: node \"A\": `height`: 0 outside the range 1 to 1000000000\n\
             {graph}:10:5: error[G003]: edge \"e1\": `target` refers to node \"B\", which does not exist\n\
             {graph}: refused (3 errors)\n"
        )
    );
}

#[test]
fn a_refused_layout_exits_one_while_its_graph_passes() {
    let graph = data("gestpret.graph.json");
    let layout = data("refuse-hors-bord.layout.json");
    let (code, out, err) = run(&["validate", &graph, &layout]);
    assert_eq!(code, 1);
    assert_eq!(out, format!("{graph}: valid graph (3 nodes, 2 edges)\n"));
    assert!(
        err.contains(&format!("{layout}:12:5: error[L009]:")),
        "{err}"
    );
    assert!(
        err.ends_with(&format!("{layout}: refused (5 errors)\n")),
        "{err}"
    );
}

#[test]
fn every_file_is_read_even_after_a_refusal() {
    let bad = data("refuse-lien-orphelin.graph.json");
    let layout = data("gestpret.layout.json");
    let (code, out, err) = run(&["validate", &bad, &layout, &layout]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert_eq!(
        err.matches("not checked against the graph").count(),
        2,
        "{err}"
    );
}

#[test]
fn an_unreadable_file_exits_two() {
    let missing = data("absent.graph.json");
    let (code, out, err) = run(&["validate", &missing]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(
        err.starts_with(&format!("{missing}: cannot read: ")),
        "{err}"
    );

    // Illisible l'emporte sur refusé : le code dit ce qu'il y a de plus grave.
    let bad = data("refuse-lien-orphelin.graph.json");
    let (code, _, _) = run(&["validate", &bad, &missing]);
    assert_eq!(code, 2);
}

#[test]
fn a_wrong_call_exits_two_with_the_usage() {
    for args in [
        &[][..],
        &["validate"],
        &["mesurer", "x"],
        &["--help", "x"],
        &["graphml"],
        &["graphml", "a", "b"],
        &["check-corpus"],
    ] {
        let (code, out, err) = run(args);
        assert_eq!(code, 2, "{args:?}");
        assert_eq!(out, "");
        assert!(err.starts_with("usage: foregram-bench validate"), "{err}");
    }
}

#[test]
fn help_and_version_exit_zero() {
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("usage: foregram-bench validate"));
    let (code, out, _) = run(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        format!("foregram-bench {}\n", env!("CARGO_PKG_VERSION"))
    );
}

// ---------------------------------------------------------------- GraphML

/// Un répertoire neuf sous la cible de compilation, vidé s'il existait.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &std::path::Path, text: &str) -> String {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
    path.display().to_string()
}

/// Un graphe GraphML de `nodes` nœuds en chaîne, à la façon de Rome.
fn chain(nodes: usize) -> String {
    let mut text = String::from("<graphml>\n<graph edgedefault=\"undirected\" id=\"G\">\n");
    for n in 0..nodes {
        text.push_str(&format!("<node id=\"n{n}\"/>\n"));
    }
    for e in 1..nodes {
        text.push_str(&format!(
            "<edge id=\"e{e}\" source=\"n{}\" target=\"n{e}\"/>\n",
            e - 1
        ));
    }
    text.push_str("</graph>\n</graphml>\n");
    text
}

fn log_line(name: &str, nodes: usize, edges: usize) -> String {
    format!(
        "----------\n\tname: {name}\tformat: graphml\tnodes: {nodes}\tedges: {edges}\n\
         \tdirected: false\tacyclic: undefined\n"
    )
}

#[test]
fn a_graphml_file_comes_out_as_a_canonical_graph() {
    let dir = scratch("graphml-one");
    let path = write(&dir.join("grafo1.3.graphml"), &chain(3));
    let (code, out, err) = run(&["graphml", &path]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "{\n  \"format\": \"foregram-graph\",\n  \"version\": 2,\n  \"name\": \"grafo1.3\",\n  \
         \"nodes\": [\n    \
         {\"id\": \"n0\", \"width\": 4950, \"height\": 3500},\n    \
         {\"id\": \"n1\", \"width\": 4950, \"height\": 3500},\n    \
         {\"id\": \"n2\", \"width\": 4950, \"height\": 3500}\n  ],\n  \
         \"edges\": [\n    \
         {\"id\": \"e1\", \"source\": \"n0\", \"target\": \"n1\"},\n    \
         {\"id\": \"e2\", \"source\": \"n1\", \"target\": \"n2\"}\n  ]\n}\n"
    );
    assert_eq!(err, "");

    // Ce qui sort se relit tel quel.
    let json = write(&dir.join("grafo1.3.graph.json"), &out);
    let (code, _, err) = run(&["validate", &json]);
    assert_eq!(code, 0, "{err}");
}

#[test]
fn a_refused_graphml_file_says_where_in_the_graphml() {
    let dir = scratch("graphml-refused");
    let path = write(
        &dir.join("bad.graphml"),
        "<graphml>\n<graph>\n<node id=\"a\"/>\n<edge id=\"e\" source=\"a\" target=\"b\"/>\n</graph>\n</graphml>\n",
    );
    let (code, out, err) = run(&["graphml", &path]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert_eq!(
        err,
        format!(
            "{path}:4:1: error[G003]: edge \"e\": `target` refers to node \"b\", which does not exist\n\
             {path}: refused (1 error)\n"
        )
    );
    let missing = dir.join("absent.graphml").display().to_string();
    let (code, _, _) = run(&["graphml", &missing]);
    assert_eq!(code, 2);
}

#[test]
fn a_corpus_that_matches_its_graph_log_exits_zero() {
    let cache = scratch("corpus-ok");
    let corpora = [
        ("rome", [("grafo1.3", 3), ("grafo2.4", 4)]),
        ("north", [("g.3.1", 3), ("g.4.1", 4)]),
    ];
    for (corpus, graphs) in corpora {
        let mut log = String::new();
        for (name, nodes) in graphs {
            write(
                &cache.join(corpus).join(format!("{name}.graphml")),
                &chain(nodes),
            );
            log.push_str(&log_line(name, nodes, nodes - 1));
        }
        write(&cache.join(corpus).join("Graph.log"), &log);
    }
    let (code, out, err) = run(&["check-corpus", &cache.display().to_string()]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "rome: 2 graphs, 7 nodes, 5 edges, all valid and as in Graph.log\n\
         north: 2 graphs, 7 nodes, 5 edges, all valid and as in Graph.log\n"
    );
    assert_eq!(err, "");
}

#[test]
fn every_disagreement_with_graph_log_is_reported() {
    let cache = scratch("corpus-bad");
    let rome = cache.join("rome");
    let differs = write(&rome.join("differs.graphml"), &chain(3));
    let absent = write(&rome.join("absent.graphml"), &chain(2));
    let refused = write(&rome.join("refused.graphml"), "<graphml/>");
    write(&rome.join("ignored.txt"), "not a graph");
    let log = write(
        &rome.join("Graph.log"),
        &format!(
            "{}{}{}\tname: broken\tnodes: x\n",
            log_line("differs", 3, 3),
            log_line("refused", 1, 0),
            log_line("orphan", 1, 0)
        ),
    );
    write(&cache.join("north").join("Graph.log"), "");
    let (code, out, err) = run(&["check-corpus", &cache.display().to_string()]);
    assert_eq!(code, 1);
    assert_eq!(
        out,
        "north: 0 graphs, 0 nodes, 0 edges, all valid and as in Graph.log\n"
    );
    assert_eq!(
        err,
        format!(
            "{log}:10: line not understood\n\
             {absent}: absent from Graph.log\n\
             {differs}: 3 nodes and 2 edges, Graph.log says 3 nodes and 3 edges\n\
             {refused}:1:1: error[X004]: `graphml` must contain exactly one `graph`, it contains 0\n\
             {refused}: refused (1 error)\n\
             {log}: lists graph \"orphan\", which has no file\n\
             rome: 3 graphs, 5 problems\n"
        )
    );
}

#[test]
fn a_cache_without_its_corpora_exits_two() {
    let cache = scratch("corpus-empty");
    let (code, out, err) = run(&["check-corpus", &cache.display().to_string()]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err.matches("cannot read").count(), 2, "{err}");
}
