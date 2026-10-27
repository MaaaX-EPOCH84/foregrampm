// ForegramPM — tests of the bench's command line.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop dead: the product's rules do not apply here.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::process::{Command, Output};

use foregram_graph::Graph;

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
fn a_wrong_call_exits_two_with_the_usage() {
    for args in [
        &[][..],
        // Validating, measuring and drawing are the business of `foregram`.
        &["validate", "x"],
        &["metrics", "x", "y"],
        &["render", "x", "y", "z.svg"],
        &["mesurer", "x"],
        &["--help", "x"],
        &["graphml"],
        &["graphml", "a", "b"],
        &["check-corpus"],
    ] {
        let (code, out, err) = run(args);
        assert_eq!(code, 2, "{args:?}");
        assert_eq!(out, "");
        assert!(err.starts_with("usage: foregram-bench graphml"), "{err}");
    }
}

#[test]
fn help_and_version_exit_zero() {
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("usage: foregram-bench graphml"));
    let (code, out, _) = run(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        format!("foregram-bench {}\n", env!("CARGO_PKG_VERSION"))
    );
}

// ---------------------------------------------------------------- GraphML

/// A fresh directory under the build target, emptied if it existed.
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

/// A GraphML graph of `nodes` nodes in a chain, in the style of Rome.
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

    // What comes out reads back as is.
    assert!(Graph::parse(out.as_bytes()).is_ok());
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
            "{path}:4:1: error[GPH003]: edge \"e\": `target` refers to node \"b\", which does not exist\n\
             {path}: refused (1 error)\n"
        )
    );
    let missing = dir.join("absent.graphml").display().to_string();
    let (code, _, _) = run(&["graphml", &missing]);
    assert_eq!(code, 2);
}

#[test]
fn a_corpus_that_matches_its_graph_log_exits_zero() {
    let cache = scratch("corpus-ok").join("cache");
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
    let elk = cache.join("elk-models");
    write(&elk.join("examples").join("a.elkt"), ELKT);
    write(
        &elk.join("tests").join("deep").join("b.elkt"),
        "node x\nlabel \"L\"",
    );
    write(&elk.join("notes.txt"), "not a graph");
    write(
        &cache.join("../manifest.txt"),
        "[rome]\ncount = 2\n\n[elk-models]\ncount = 2\nimported = 2\n",
    );
    let (code, out, err) = run(&["check-corpus", &cache.display().to_string()]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "rome: 2 graphs, 7 nodes, 5 edges, all valid and as in Graph.log\n\
         north: 2 graphs, 7 nodes, 5 edges, all valid and as in Graph.log\n\
         elk-models: 2 graphs, 4 nodes, 2 edges, all valid; not carried: 2 labels, \
         1 node without size, given 1 × 1, 0 fixed port positions on a container\n"
    );
    assert_eq!(err, "");
}

/// An `.elkt` that uses what the import carries: container, ports, sides,
/// link label with its size, and a node label, which does not pass.
const ELKT: &str = "elk.direction: RIGHT\n\
    node sys {\n\
    \x20 node core { layout [size: 10, 5] portConstraints: FIXED_SIDE port in { side: WEST } label \"Core\" }\n\
    \x20 node store { layout [size: 8, 4] }\n\
    \x20 edge core -> store\n\
    }\n\
    edge e: sys.store -> sys.core.in { label \"x\" { layout [size: 3, 1] } }\n";

#[test]
fn an_elkt_file_comes_out_as_a_canonical_graph() {
    let dir = scratch("elkt-one");
    let path = write(&dir.join("sample.elkt"), ELKT);
    let (code, out, err) = run(&["elkt", &path]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "{\n  \"format\": \"foregram-graph\",\n  \"version\": 2,\n  \"name\": \"sample\",\n  \
         \"direction\": \"right\",\n  \
         \"nodes\": [\n    \
         {\"id\": \"sys\", \"children\": [\n      \
         {\"id\": \"core\", \"width\": 1000, \"height\": 500, \"ports\": [{\"id\": \"in\", \"side\": \"west\"}]},\n      \
         {\"id\": \"store\", \"width\": 800, \"height\": 400}\n    ]}\n  ],\n  \
         \"edges\": [\n    \
         {\"id\": \"e1\", \"source\": \"core\", \"target\": \"store\"},\n    \
         {\"id\": \"e\", \"source\": \"store\", \"target\": \"core\", \"target_port\": \"in\", \
         \"labels\": [{\"width\": 300, \"height\": 100, \"at\": \"center\"}]}\n  ]\n}\n"
    );
    assert_eq!(
        err,
        format!(
            "{path}: not carried: 1 label, 0 nodes without size, given 1 × 1, 0 fixed port positions on a container\n"
        )
    );

    // What comes out reads back as is; a lossless file says nothing.
    assert!(Graph::parse(out.as_bytes()).is_ok());
    let plain = write(
        &dir.join("plain.elkt"),
        "node a { layout [size: 1, 1] }\nedge a -> a",
    );
    let (code, _, err) = run(&["elkt", &plain]);
    assert_eq!((code, err.as_str()), (0, ""));

    // A refusal points into the `.elkt`.
    let bad = write(
        &dir.join("bad.elkt"),
        "node a { layout [size: 1, 1] }\nedge a -> b\n",
    );
    let (code, out, err) = run(&["elkt", &bad]);
    assert_eq!((code, out.as_str()), (1, ""));
    assert_eq!(
        err,
        format!(
            "{bad}:2:11: error[ELK007]: `b` refers to no node or port\n{bad}: refused (1 error)\n"
        )
    );
    let (code, _, _) = run(&["elkt", &dir.join("absent.elkt").display().to_string()]);
    assert_eq!(code, 2);
}

#[test]
fn every_disagreement_with_graph_log_is_reported() {
    let cache = scratch("corpus-bad").join("cache");
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
    let elk = cache.join("elk-models");
    let unresolved = write(&elk.join("unresolved.elkt"), "node a\nedge a -> b\n");
    write(&elk.join("ok.elkt"), "node a\n");
    write(
        &cache.join("../manifest.txt"),
        "[elk-models]\ncount = 3\nimported = 3\n",
    );
    let manifest = cache
        .parent()
        .unwrap()
        .join("manifest.txt")
        .display()
        .to_string();
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
             {refused}:1:1: error[GML004]: `graphml` must contain exactly one `graph`, it contains 0\n\
             {refused}: refused (1 error)\n\
             {log}: lists graph \"orphan\", which has no file\n\
             rome: 3 graphs, 5 problems\n\
             {unresolved}:2:11: error[ELK007]: `b` refers to no node or port\n\
             {unresolved}: refused (1 error)\n\
             {manifest}: 2 files found and 1 imported, the manifest says 3 files and 3\n\
             elk-models: 2 graphs, 2 problems\n"
        )
    );
}

#[test]
fn a_cache_without_its_corpora_exits_two() {
    let cache = scratch("corpus-empty").join("cache");
    let (code, out, err) = run(&["check-corpus", &cache.display().to_string()]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err.matches("cannot read").count(), 3, "{err}");
}

// ------------------------------------------------------------------- MCD

/// The versioned in-house corpus, and what each model must yield, counted
/// by hand in the `.mcd`: entities plus associations, links.
const IN_HOUSE: [(&str, usize, usize); 4] = [
    ("commande.mcd", 10 + 9, 18),
    ("employe.mcd", 2 + 4, 8),
    ("gestpret.mcd", 2 + 1, 2),
    ("interpreteur_wl.mcd", 11 + 4, 8),
];

#[test]
fn every_in_house_model_converts_to_a_valid_graph() {
    let corpus = format!("{}/../../corpus/mcd", env!("CARGO_MANIFEST_DIR"));
    let mut found: Vec<String> = std::fs::read_dir(&corpus)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".mcd"))
        .collect();
    found.sort();
    let listed: Vec<&str> = IN_HOUSE.iter().map(|(name, _, _)| *name).collect();
    assert_eq!(found, listed, "corpus maison et liste du test divergent");

    for (name, nodes, edges) in IN_HOUSE {
        let (code, out, err) = run(&["mcd", &format!("{corpus}/{name}")]);
        assert_eq!(code, 0, "{name} : {err}");
        assert_eq!(err, "");
        // What comes out reads back as is, and counts what the `.mcd` declares.
        let graph = Graph::parse(out.as_bytes()).unwrap();
        assert_eq!(graph.all_nodes().len(), nodes, "{name}");
        assert_eq!(graph.edges.len(), edges, "{name}");
    }
}

#[test]
fn a_refused_model_says_where_in_the_mcd() {
    let dir = scratch("mcd-refused");
    let path = write(
        &dir.join("bad.mcd"),
        "entite A\nassociation R\n  A 0,n\n  B 1,1\n",
    );
    let (code, out, err) = run(&["mcd", &path]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert_eq!(
        err,
        format!(
            "{path}:4:3: error[MCD007]: `B` is not a declared entity\n\
             {path}: refused (1 error)\n"
        )
    );
    let missing = dir.join("absent.mcd").display().to_string();
    let (code, _, _) = run(&["mcd", &missing]);
    assert_eq!(code, 2);
}

// ------------------------------------------------------------------- Board

/// The board's fixture set: four small corpora written here, their placements
/// by dot and ELK, frozen, and what the board must draw from them. The placement of
/// `elk-models/e2.elkt` by dot is removed and its `REF003` failure written by
/// hand in `failures.txt`: an engine that returned nothing.
fn board_data(name: &str) -> String {
    format!("{}/tests/data/board/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

fn files_under(dir: &std::path::Path, base: &std::path::Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(&path, base, out);
        } else {
            out.push(path.strip_prefix(base).unwrap().display().to_string());
        }
    }
}

#[test]
fn the_board_is_written_as_expected_and_again_the_same() {
    let out = scratch("board").join("out");
    let cache = board_data("cache");
    let layouts = board_data("layouts");
    let args = [
        "board",
        &cache,
        &out.display().to_string(),
        "--layouts",
        &layouts,
    ];
    let (code, stdout, err) = run(&args);
    assert_eq!(code, 0, "{err}");
    assert_eq!(err, "");
    assert_eq!(
        stdout,
        format!("{}: board written, 6 graphs, 6 rows drawn\n", out.display())
    );
    for name in ["metrics.json", "report.md", "board.md"] {
        let written = std::fs::read_to_string(out.join(name)).unwrap();
        let expected = std::fs::read_to_string(board_data(&format!("expected/{name}"))).unwrap();
        assert_eq!(written, expected, "{name}");
    }
    // One rendering per measured placement: dot has none for elk-models.
    let mut svgs = Vec::new();
    files_under(&out.join("svg"), &out, &mut svgs);
    svgs.sort();
    assert_eq!(
        svgs,
        [
            "svg/dot/mcd/gestpret.svg",
            "svg/dot/north/n1.svg",
            "svg/dot/rome/r1.svg",
            "svg/dot/rome/r2.svg",
            "svg/elk/elk-models/e1.svg",
            "svg/elk/elk-models/e2.svg",
            "svg/elk/mcd/gestpret.svg",
            "svg/elk/north/n1.svg",
            "svg/elk/rome/r1.svg",
            "svg/elk/rome/r2.svg",
        ]
    );
    // A second pass renders the same bytes, and leaves no rendering
    // from a previous sample.
    let first: Vec<Vec<u8>> = svgs
        .iter()
        .map(|f| std::fs::read(out.join(f)).unwrap())
        .collect();
    write(&out.join("svg/dot/rome/old.svg"), "<svg/>");
    let (code, _, err) = run(&args);
    assert_eq!(code, 0, "{err}");
    let mut again = Vec::new();
    files_under(&out.join("svg"), &out, &mut again);
    again.sort();
    assert_eq!(again, svgs);
    let second: Vec<Vec<u8>> = svgs
        .iter()
        .map(|f| std::fs::read(out.join(f)).unwrap())
        .collect();
    assert_eq!(first, second);
    for name in ["metrics.json", "report.md", "board.md"] {
        let written = std::fs::read_to_string(out.join(name)).unwrap();
        let expected = std::fs::read_to_string(board_data(&format!("expected/{name}"))).unwrap();
        assert_eq!(written, expected, "{name}");
    }
}

#[test]
fn a_board_with_a_layout_missing_and_unlisted_writes_nothing() {
    let dir = scratch("board-missing");
    let layouts = dir.join("layouts");
    copy_dir(std::path::Path::new(&board_data("layouts")), &layouts);
    std::fs::remove_file(layouts.join("elk/rome/r2.layout.json")).unwrap();
    let out = dir.join("out");
    let (code, stdout, err) = run(&[
        "board",
        &board_data("cache"),
        &out.display().to_string(),
        "--layouts",
        &layouts.display().to_string(),
    ]);
    assert_eq!(code, 2);
    assert_eq!(stdout, "");
    assert_eq!(
        err,
        format!(
            "rome/r2.graphml: no layout by elk, and no failure listed\n\
             {}: 1 problem, nothing written\n",
            layouts.display()
        )
    );
    assert!(!out.exists());
}

#[test]
fn a_board_with_a_layout_from_another_engine_version_writes_nothing() {
    let dir = scratch("board-version");
    let layouts = dir.join("layouts");
    copy_dir(std::path::Path::new(&board_data("layouts")), &layouts);
    let path = layouts.join("elk/north/n1.layout.json");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"version\": \"0.12.0\""));
    std::fs::write(
        &path,
        text.replace("\"version\": \"0.12.0\"", "\"version\": \"0.11.0\""),
    )
    .unwrap();
    let out = dir.join("out");
    let (code, _, err) = run(&[
        "board",
        &board_data("cache"),
        &out.display().to_string(),
        "--layouts",
        &layouts.display().to_string(),
    ]);
    assert_eq!(code, 2);
    assert!(
        err.starts_with(&format!(
            "{}: laid out by elk 0.11.0, the manifest pins elk 0.12.0\n",
            path.display()
        )),
        "{err}"
    );
    assert!(!out.exists());
}
