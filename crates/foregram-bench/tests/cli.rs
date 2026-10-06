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
    for args in [&[][..], &["validate"], &["mesurer", "x"], &["--help", "x"]] {
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
