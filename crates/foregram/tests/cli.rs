// ForegramPM — tests de la ligne de commande du produit.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// Un test qui échoue doit s'arrêter net : les règles du produit ne valent pas ici.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn data(name: &str) -> String {
    format!(
        "{}/../foregram-graph/tests/data/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn golden(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../foregram-render/tests/data/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(path).unwrap()
}

/// Lance `foregram`, l'entrée standard remplie de `stdin`.
fn run_with(args: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_foregram"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let Output {
        status,
        stdout,
        stderr,
    } = child.wait_with_output().unwrap();
    (
        status.code().unwrap(),
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn run(args: &[&str]) -> (i32, String, String) {
    run_with(args, b"")
}

/// Un répertoire neuf sous la cible de compilation, vidé s'il existait.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ---------------------------------------------------------------- validate

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

// ------------------------------------------------------------------- canon

#[test]
fn a_canonical_file_comes_out_unchanged() {
    for name in ["gestpret.graph.json", "components.layout.json"] {
        let path = data(name);
        let (code, out, err) = run(&["canon", &path]);
        assert_eq!(code, 0, "{name} : {err}");
        assert_eq!(out.as_bytes(), std::fs::read(&path).unwrap(), "{name}");
        assert_eq!(err, "");
    }
}

#[test]
fn a_version_one_graph_comes_out_in_version_two_with_keys_in_order() {
    let text = b"{\"edges\": [], \"nodes\": [{\"height\": 2, \"id\": \"A\", \"width\": 1}], \
                 \"name\": \"g\", \"version\": 1, \"format\": \"foregram-graph\"}";
    let (code, out, err) = run_with(&["canon", "-"], text);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "{\n  \"format\": \"foregram-graph\",\n  \"version\": 2,\n  \"name\": \"g\",\n  \
         \"nodes\": [\n    {\"id\": \"A\", \"width\": 1, \"height\": 2}\n  ],\n  \
         \"edges\": []\n}\n"
    );
    // Réécrire la forme canonique ne la change pas.
    let (code, again, _) = run_with(&["canon", "-"], out.as_bytes());
    assert_eq!(code, 0);
    assert_eq!(again, out);
}

#[test]
fn a_refused_file_is_not_rewritten() {
    let graph = data("refuse-lien-orphelin.graph.json");
    let (code, out, err) = run(&["canon", &graph]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(
        err.ends_with(&format!("{graph}: refused (3 errors)\n")),
        "{err}"
    );

    // Un placement se reconnaît à sa clé `format` : celui-ci est valide seul,
    // il n'est refusé que confronté à son graphe.
    let layout = data("refuse-hors-bord.layout.json");
    let (code, _, err) = run(&["canon", &layout]);
    assert_eq!(code, 0, "{err}");

    // Ni graphe ni placement : lu comme un graphe, qui dit ce qui manque.
    let (code, out, err) = run_with(&["canon", "-"], b"[1, 2]");
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.starts_with("<stdin>:1:1: error["), "{err}");
}

// ----------------------------------------------------------------- metrics

#[test]
fn metrics_are_written_one_line_per_measure() {
    let graph = data("components.graph.json");
    let layout = data("components.layout.json");
    let (code, out, err) = run(&["metrics", &graph, &layout]);
    assert_eq!(code, 0, "{err}");
    let mut lines = out.lines();
    assert_eq!(
        lines.next(),
        Some(format!("{layout}: metrics of graph \"components\" (engine hand 1)").as_str())
    );
    assert_eq!(lines.next(), Some("  crossings: 0"));
    assert!(out.contains("\n  bends: 2\n"), "{out}");
    assert!(lines.all(|line| line.starts_with("  ")), "{out}");
}

#[test]
fn a_layout_that_does_not_fit_its_graph_is_not_measured() {
    let graph = data("gestpret.graph.json");
    let good = data("gestpret.layout.json");
    let bad = data("refuse-hors-bord.layout.json");
    let (code, out, err) = run(&["metrics", &graph, &bad, &good]);
    assert_eq!(code, 1);
    assert!(!out.contains(&bad), "{out}");
    assert!(out.starts_with(&format!("{good}: metrics of")), "{out}");
    assert!(
        err.ends_with(&format!("{bad}: refused (5 errors)\n")),
        "{err}"
    );
}

// ------------------------------------------------------------------ render

#[test]
fn an_svg_is_the_one_of_the_renderer() {
    let dir = scratch("render-svg");
    let graph = data("components.graph.json");
    let layout = data("components.layout.json");
    let out_path = dir.join("components.svg");
    let out_file = out_path.display().to_string();
    let (code, out, err) = run(&["render", &graph, &layout, &out_file]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        format!("{layout}: rendered into {out_file}, graph \"components\" (engine hand 1)\n")
    );
    assert_eq!(std::fs::read(&out_path).unwrap(), golden("components.svg"));
}

#[test]
fn a_png_is_written_at_the_scale_asked() {
    let dir = scratch("render-png");
    let graph = data("gestpret.graph.json");
    let layout = data("gestpret.layout.json");
    let mut sizes = Vec::new();
    for scale in ["1", "2"] {
        let out_path = dir.join(format!("gestpret-{scale}.png"));
        let out_file = out_path.display().to_string();
        let (code, _, err) = run(&["render", &graph, &layout, &out_file, "--scale", scale]);
        assert_eq!(code, 0, "{err}");
        let png = std::fs::read(&out_path).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        // Largeur de l'en-tête IHDR, gros-boutiste.
        sizes.push(u32::from_be_bytes(png[16..20].try_into().unwrap()));
    }
    assert_eq!(sizes[1], 2 * sizes[0]);

    // Une échelle hors bornes est un refus du rendu, rien n'est écrit.
    let out_path = dir.join("trop.png");
    let out_file = out_path.display().to_string();
    let (code, _, err) = run(&["render", &graph, &layout, &out_file, "--scale", "9"]);
    assert_eq!(code, 1);
    assert!(err.contains("error[V001]"), "{err}");
    assert!(!out_path.exists());
}

#[test]
fn a_refused_layout_is_not_drawn() {
    let dir = scratch("render-refused");
    let out_path = dir.join("x.svg");
    let out_file = out_path.display().to_string();
    let graph = data("gestpret.graph.json");
    let layout = data("refuse-hors-bord.layout.json");
    let (code, _, _) = run(&["render", &graph, &layout, &out_file]);
    assert_eq!(code, 1);
    assert!(!out_path.exists());
}

// --------------------------------------------------------- entrée standard

#[test]
fn a_file_named_dash_is_the_standard_input() {
    let graph = std::fs::read(data("components.graph.json")).unwrap();
    let layout = data("components.layout.json");
    let (code, out, err) = run_with(&["validate", "-", &layout], &graph);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("<stdin>: valid graph (4 nodes, 3 edges)\n"),
        "{out}"
    );

    let dir = scratch("render-stdin");
    let out_path = dir.join("components.svg");
    let out_file = out_path.display().to_string();
    let (code, _, err) = run_with(&["render", "-", &layout, &out_file], &graph);
    assert_eq!(code, 0, "{err}");
    assert_eq!(std::fs::read(&out_path).unwrap(), golden("components.svg"));

    // Refusée, l'entrée standard est nommée.
    let (code, _, err) = run_with(&["validate", "-"], b"{");
    assert_eq!(code, 1);
    assert!(err.ends_with("<stdin>: refused (1 error)\n"), "{err}");
}

#[test]
fn the_standard_input_is_read_once_per_call() {
    let (code, out, err) = run(&["validate", "-", "-"]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(err.starts_with("usage: foregram "), "{err}");
}

// ---------------------------------------------------------------- l'appel

#[test]
fn english_is_the_only_language_for_now() {
    let graph = data("gestpret.graph.json");
    let (code, out, _) = run(&["--lang", "en", "validate", &graph]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("{graph}: valid graph (3 nodes, 2 edges)\n"));

    let (code, out, err) = run(&["--lang", "fr", "validate", &graph]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "unknown language `fr`: the only catalogue is `en`\n");
}

#[test]
fn json_output_is_refused_until_it_exists() {
    let graph = data("gestpret.graph.json");
    let (code, out, err) = run(&["--json", "validate", &graph]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(err.starts_with("`--json` is not available yet"), "{err}");
}

#[test]
fn a_wrong_call_exits_two_with_the_usage() {
    for args in [
        &[][..],
        &["validate"],
        &["canon"],
        &["canon", "a", "b"],
        &["metrics", "g"],
        &["render", "g", "l"],
        &["render", "g", "l", "out.pdf"],
        &["render", "g", "l", "out.png", "--scale", "deux"],
        &["graphml", "x"],
        &["--lang"],
        &["--help", "x"],
    ] {
        let (code, out, err) = run(args);
        assert_eq!(code, 2, "{args:?}");
        assert_eq!(out, "");
        assert!(
            err.starts_with("usage: foregram [--lang"),
            "{args:?} : {err}"
        );
    }
}

#[test]
fn help_and_version_exit_zero() {
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("usage: foregram [--lang"));
    let (code, out, _) = run(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("foregram {}\n", env!("CARGO_PKG_VERSION")));
}
