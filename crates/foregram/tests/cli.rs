// ForegramPM — tests of the product's command line.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop dead: the product's rules do not apply here.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use foregram_graph::json::{self, Kind, Value};
use foregram_graph::{Graph, Layout, Metrics};

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

/// Runs `foregram`, standard input filled with `stdin`.
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

/// A fresh directory under the build target, emptied if it existed.
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

    // Unreadable wins over refused: the code reports the graver case.
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
    // Rewriting the canonical form does not change it.
    let (code, again, _) = run_with(&["canon", "-"], out.as_bytes());
    assert_eq!(code, 0);
    assert_eq!(again, out);
}

#[test]
fn a_source_file_is_rewritten_in_its_canonical_form() {
    // A canonical source file comes out unchanged; the file is told from
    // the exchange format by its first character.
    let path = format!(
        "{}/../../corpus/mcd/gestpret.fg",
        env!("CARGO_MANIFEST_DIR")
    );
    let (code, out, err) = run(&["canon", &path]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.as_bytes(), std::fs::read(&path).unwrap());
    assert_eq!(err, "");

    // Folded on the standard input, unfolded on the output.
    let folded = b"// g\n@notation merise\nentity A \"A\" { #a; b } // c\n";
    let (code, out, err) = run_with(&["canon", "-"], folded);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "// g\n\n@notation merise\n\nentity A {\n  #a\n  b\n} // c\n"
    );
    let (code, again, _) = run_with(&["canon", "-"], out.as_bytes());
    assert_eq!((code, again), (0, out));

    // Under --json, one object on one line, the text as a string.
    let (code, out, err) = run_with(&["--json", "canon", "-"], folded);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "{\"kind\": \"canonical\", \"file\": \"<stdin>\", \"document\": \
         \"// g\\n\\n@notation merise\\n\\nentity A {\\n  #a\\n  b\\n} // c\\n\"}\n"
    );
    assert_eq!(out.lines().count(), 1);
    let parsed = objects(&out);
    assert_eq!(string(&parsed[0], "kind"), "canonical");

    // A refused source file is not rewritten; its refusals carry a `T` code.
    let (code, out, err) = run_with(&["canon", "-"], b"// g\nentity 1a {\n");
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.starts_with("<stdin>:2:8: error[T009]"), "{err}");
    assert!(err.ends_with("<stdin>: refused (2 errors)\n"), "{err}");
    let (code, out, _) = run_with(&["--json", "--lang", "fr", "canon", "-"], b"entity 1a {\n");
    assert_eq!(code, 1);
    let parsed = objects(&out);
    assert_eq!(parsed.len(), 2);
    assert_eq!(string(&parsed[0], "kind"), "diagnostic");
    assert_eq!(string(&parsed[0], "code"), "T009");

    // Blanks and a byte order mark before `{` still mean the exchange format.
    let (code, _, err) = run_with(&["canon", "-"], b"\xef\xbb\xbf \n {\"format\": 1}");
    assert_eq!(code, 1);
    assert!(err.starts_with("<stdin>:"), "{err}");
    assert!(!err.contains("error[T"), "{err}");
    let (code, _, err) = run_with(&["canon", "-"], b"[1]");
    assert_eq!(code, 1);
    assert!(!err.contains("error[T"), "{err}");
    // The empty file is a source file, and its canonical form is empty.
    let (code, out, err) = run_with(&["canon", "-"], b"");
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
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

    // A placement is recognised by its `format` key: this one is valid alone,
    // it is refused only when checked against its graph.
    let layout = data("refuse-hors-bord.layout.json");
    let (code, _, err) = run(&["canon", &layout]);
    assert_eq!(code, 0, "{err}");

    // Neither graph nor placement: read as a graph, which says what is missing.
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
        // Width from the IHDR header, big-endian.
        sizes.push(u32::from_be_bytes(png[16..20].try_into().unwrap()));
    }
    assert_eq!(sizes[1], 2 * sizes[0]);

    // An out-of-range scale is a refusal of the rendering, nothing is written.
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

// ---------------------------------------------------------- standard input

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

    // Refused, standard input is named.
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

// --------------------------------------------------------------- the call

#[test]
fn lang_chooses_english_or_french_and_refuses_any_other() {
    let graph = data("gestpret.graph.json");
    let (code, out, _) = run(&["--lang", "en", "validate", &graph]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("{graph}: valid graph (3 nodes, 2 edges)\n"));
    let (code, out, _) = run(&["--lang", "fr", "validate", &graph]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("{graph}: graphe valide (3 nœuds, 2 liens)\n"));

    // Without `--lang`, English, whatever the system language.
    let english = Command::new(env!("CARGO_BIN_EXE_foregram"))
        .args(["validate", &graph])
        .env("LANG", "fr_FR.UTF-8")
        .env("LC_ALL", "fr_FR.UTF-8")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(english.stdout).unwrap(),
        format!("{graph}: valid graph (3 nodes, 2 edges)\n")
    );

    for lang in ["de", "FR", "fr-FR", ""] {
        let (code, out, err) = run(&["--lang", lang, "validate", &graph]);
        assert_eq!(code, 2);
        assert_eq!(out, "");
        assert_eq!(
            err,
            format!("unknown language `{lang}`: the catalogues are `en` and `fr`\n")
        );
    }
    let (code, _, err) = run(&["--lang", "fr", "--help"]);
    assert_eq!(code, 0);
    assert_eq!(err, "");
    let (_, out, _) = run(&["--lang", "fr", "--help"]);
    assert!(out.starts_with("usage\u{a0}: foregram [--lang"), "{out}");
}

#[test]
fn french_takes_the_singular_for_zero_and_one() {
    let graph = |nodes: usize| {
        let nodes: Vec<String> = (0..nodes)
            .map(|i| format!("{{\"id\": \"n{i}\", \"width\": 1, \"height\": 1}}"))
            .collect();
        format!(
            "{{\"format\": \"foregram-graph\", \"version\": 2, \"name\": \"g\",              \"nodes\": [{}], \"edges\": []}}",
            nodes.join(", ")
        )
    };
    for (nodes, en, fr) in [
        (0, "(0 nodes, 0 edges)", "(0 nœud, 0 lien)"),
        (1, "(1 node, 0 edges)", "(1 nœud, 0 lien)"),
        (2, "(2 nodes, 0 edges)", "(2 nœuds, 0 lien)"),
    ] {
        let text = graph(nodes);
        let (_, out, _) = run_with(&["validate", "-"], text.as_bytes());
        assert_eq!(out, format!("<stdin>: valid graph {en}\n"));
        let (_, out, _) = run_with(&["--lang", "fr", "validate", "-"], text.as_bytes());
        assert_eq!(out, format!("<stdin>: graphe valide {fr}\n"));
    }
}

#[test]
fn a_refusal_reads_in_french_with_the_same_code_and_position() {
    let graph = data("refuse-lien-orphelin.graph.json");
    let (code, _, err) = run(&["--lang", "fr", "validate", &graph]);
    assert_eq!(code, 1);
    let nb = '\u{a0}';
    assert_eq!(
        err,
        format!(
            "{graph}:7:5: erreur[G001]{nb}: identifiant de nœud «{nb}A{nb}» déjà employé (ligne 6)\n\
             {graph}:7:5: erreur[S008]{nb}: nœud «{nb}A{nb}»{nb}: `height`{nb}: 0 hors de l'intervalle de 1 à 1000000000\n\
             {graph}:10:5: erreur[G003]{nb}: lien «{nb}e1{nb}»{nb}: `target` désigne le nœud «{nb}B{nb}», qui n'existe pas\n\
             {graph}: refusé (3 erreurs)\n"
        )
    );
}

#[test]
fn the_documentation_shows_a_french_refusal_as_it_is() {
    let graph = data("refuse-lien-orphelin.graph.json");
    let (_, _, err) = run(&["--lang", "fr", "validate", &graph]);
    let line = err.lines().find(|l| l.contains("[G003]")).unwrap();
    let line = line.replace(&graph, "example.graph.json");
    let doc = std::fs::read_to_string(format!(
        "{}/../../docs/exchange-format.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(doc.contains(&format!("\n{line}\n")), "{line}");
}

#[test]
fn the_json_stream_is_the_same_in_every_language_but_its_text() {
    let graph = data("gestpret.graph.json");
    let calls: [&[&str]; 4] = [
        &["validate", &data("refuse-lien-orphelin.graph.json")],
        &[
            "validate",
            &graph,
            &data("gestpret.layout.json"),
            &data("refuse-hors-bord.layout.json"),
        ],
        &[
            "metrics",
            &data("components.graph.json"),
            &data("components.layout.json"),
        ],
        &["validate", "absent.json"],
    ];
    for call in calls {
        let (en_code, en, _) = run(&[&["--json"], call].concat());
        let (fr_code, fr, _) = run(&[&["--json", "--lang", "fr"], call].concat());
        assert_eq!(en_code, fr_code);
        let strip = |stream: &str| -> Vec<String> {
            stream
                .lines()
                .map(|line| line.split(", \"text\": ").next().unwrap().to_string())
                .collect()
        };
        assert_eq!(strip(&en), strip(&fr), "{call:?}");
        assert_ne!(en, fr, "{call:?}");
        assert!(objects(&fr).iter().all(|o| !string(o, "text").is_empty()));
    }
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

// -------------------------------------------------------------------- JSON

/// Every line of the stream, read back by the library's strict reader:
/// what a program will read.
fn objects(out: &str) -> Vec<Value> {
    out.lines()
        .map(|line| json::parse(line.as_bytes()).unwrap_or_else(|d| panic!("{line} : {d}")))
        .collect()
}

/// The value of a member of the object.
fn field<'a>(value: &'a Value, key: &str) -> &'a Kind {
    let Kind::Object(members) = &value.kind else {
        panic!("pas un objet : {value:?}");
    };
    &members.iter().find(|m| m.key == key).unwrap().value.kind
}

fn string(value: &Value, key: &str) -> String {
    match field(value, key) {
        Kind::Str(text) => text.clone(),
        other => panic!("{key} : {other:?}"),
    }
}

/// The keys of the object, in written order.
fn keys(value: &Value) -> Vec<&str> {
    let Kind::Object(members) = &value.kind else {
        panic!("pas un objet : {value:?}");
    };
    members.iter().map(|m| m.key.as_str()).collect()
}

#[test]
fn json_results_are_one_object_per_line_on_the_standard_output() {
    let graph = data("gestpret.graph.json");
    let layout = data("gestpret.layout.json");
    let (code, out, err) = run(&["--json", "validate", &graph, &layout]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(err, "");
    let path = json::quote(&graph);
    assert_eq!(
        out.lines().next().unwrap(),
        format!(
            "{{\"kind\": \"valid\", \"file\": {path}, \"key\": \"cli.valid-graph\", \
             \"args\": {{\"path\": {path}, \
             \"nodes\": {{\"key\": \"count.node\", \"args\": {{\"n\": \"3\"}}}}, \
             \"edges\": {{\"key\": \"count.edge\", \"args\": {{\"n\": \"2\"}}}}}}, \
             \"text\": {}}}",
            json::quote(&format!("{graph}: valid graph (3 nodes, 2 edges)"))
        )
    );
    let all = objects(&out);
    assert_eq!(all.len(), 2);
    assert_eq!(string(&all[1], "kind"), "valid");
    assert_eq!(string(&all[1], "file"), layout);
    assert_eq!(string(&all[1], "key"), "cli.valid-layout");
}

#[test]
fn json_refusals_carry_code_place_key_arguments_and_sentence() {
    let graph = data("refuse-lien-orphelin.graph.json");
    let layout = data("gestpret.layout.json");
    let (code, out, err) = run(&["--json", "validate", &graph, &layout]);
    assert_eq!(code, 1);
    assert_eq!(err, "");
    let all = objects(&out);
    assert_eq!(all.len(), 4, "{out}");
    let first = &all[0];
    assert_eq!(
        keys(first),
        ["kind", "file", "code", "line", "col", "key", "args", "text"]
    );
    assert_eq!(string(first, "kind"), "diagnostic");
    assert_eq!(string(first, "file"), graph);
    assert_eq!(string(first, "code"), "G001");
    assert_eq!(field(first, "line"), &Kind::Int(7));
    assert_eq!(field(first, "col"), &Kind::Int(5));
    assert_eq!(string(first, "key"), "G001.again");
    assert_eq!(
        string(first, "text"),
        "node identifier \"A\" already used (line 6)"
    );

    // A message within a message keeps its key and arguments, without a phrase.
    let nested = out.lines().nth(1).unwrap();
    assert!(
        nested.contains(
            "\"args\": {\"field\": {\"key\": \"field.at\", \"args\": {\"subject\": \
             {\"key\": \"subject.node\", \"args\": {\"id\": \"A\"}}, \"key\": \"height\"}}"
        ),
        "{nested}"
    );

    // The placement is not checked against the refused graph: an `error` object.
    assert_eq!(string(&all[3], "kind"), "error");
    assert_eq!(string(&all[3], "key"), "cli.not-checked");
}

#[test]
fn a_json_refusal_without_a_place_has_null_line_and_column() {
    let dir = scratch("json-no-place");
    let out_file = dir.join("trop.png").display().to_string();
    let graph = data("gestpret.graph.json");
    let layout = data("gestpret.layout.json");
    let (code, out, err) = run(&[
        "--json", "render", &graph, &layout, &out_file, "--scale", "9",
    ]);
    assert_eq!(code, 1);
    assert_eq!(err, "");
    let all = objects(&out);
    assert_eq!(all.len(), 1, "{out}");
    assert_eq!(string(&all[0], "code"), "V001");
    assert_eq!(string(&all[0], "file"), out_file);
    assert_eq!(field(&all[0], "line"), &Kind::Null);
    assert_eq!(field(&all[0], "col"), &Kind::Null);
}

#[test]
fn an_unreadable_file_is_a_json_error_with_its_name_escaped() {
    let missing = "/nonexistent/a \"b\"\tc\nd.json";
    let (code, out, err) = run(&["--json", "validate", missing]);
    assert_eq!(code, 2);
    assert_eq!(err, "");
    assert_eq!(out.lines().count(), 1, "{out}");
    let all = objects(&out);
    assert_eq!(string(&all[0], "kind"), "error");
    assert_eq!(string(&all[0], "key"), "cli.unreadable");
    assert_eq!(string(&all[0], "file"), missing);
}

#[test]
fn json_metrics_are_the_ones_of_the_library() {
    let graph = data("components.graph.json");
    let layout = data("components.layout.json");
    let (code, out, err) = run(&["--json", "metrics", &graph, &layout]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(err, "");
    let all = objects(&out);
    assert_eq!(all.len(), 1, "{out}");
    assert_eq!(string(&all[0], "kind"), "metrics");
    assert_eq!(string(&all[0], "key"), "cli.measured");
    let g = Graph::parse(&std::fs::read(&graph).unwrap()).unwrap();
    let l = Layout::parse(&std::fs::read(&layout).unwrap()).unwrap();
    let expected = format!(", \"metrics\": {}}}\n", Metrics::measure(&g, &l).to_json());
    assert!(out.ends_with(&expected), "{out}");
}

#[test]
fn json_canon_wraps_the_canonical_document() {
    for name in ["gestpret.graph.json", "components.layout.json"] {
        let path = data(name);
        let (code, out, err) = run(&["--json", "canon", &path]);
        assert_eq!(code, 0, "{name} : {err}");
        assert_eq!(err, "");
        let canonical = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
        let expected = format!(
            "{{\n  \"kind\": \"canonical\",\n  \"file\": {},\n  \"document\": {}\n}}\n",
            json::quote(&path),
            canonical.trim_end_matches('\n').replace('\n', "\n  ")
        );
        assert_eq!(out, expected, "{name}");
        // A single object, which a JSON reader reads back whole.
        let value = json::parse(out.as_bytes()).unwrap();
        assert_eq!(keys(&value), ["kind", "file", "document"]);
    }
}

#[test]
fn json_canon_of_a_refused_file_is_its_refusals() {
    let path = data("refuse-lien-orphelin.graph.json");
    let (code, out, err) = run(&["--json", "canon", &path]);
    assert_eq!(code, 1);
    assert_eq!(err, "");
    let all = objects(&out);
    assert!(!all.is_empty());
    assert!(all.iter().all(|o| string(o, "kind") == "diagnostic"));
}

#[test]
fn json_render_says_what_was_drawn() {
    let dir = scratch("json-render");
    let out_file = dir.join("g.svg").display().to_string();
    let graph = data("gestpret.graph.json");
    let layout = data("gestpret.layout.json");
    let (code, out, err) = run(&["--json", "render", &graph, &layout, &out_file]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(err, "");
    let all = objects(&out);
    assert_eq!(all.len(), 1);
    assert_eq!(string(&all[0], "kind"), "rendered");
    assert_eq!(string(&all[0], "file"), layout);
    assert!(std::path::Path::new(&out_file).exists());
}

#[test]
fn json_and_lang_come_in_either_order_and_a_wrong_call_stays_text() {
    let graph = data("gestpret.graph.json");
    let (a, out_a, _) = run(&["--json", "--lang", "en", "validate", &graph]);
    let (b, out_b, _) = run(&["--lang", "en", "--json", "validate", &graph]);
    assert_eq!((a, b), (0, 0));
    assert_eq!(out_a, out_b);
    assert_eq!(objects(&out_a).len(), 1);

    let (code, out, err) = run(&["--json", "validate"]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(err.starts_with("usage: foregram "), "{err}");
}

#[test]
fn the_documentation_shows_the_json_stream_as_it_is() {
    let bytes = std::fs::read(data("refuse-lien-orphelin.graph.json")).unwrap();
    let (code, out, _) = run_with(&["--json", "validate", "-"], &bytes);
    assert_eq!(code, 1);
    let doc_path = format!(
        "{}/../../docs/exchange-format.md",
        env!("CARGO_MANIFEST_DIR")
    );
    let doc = std::fs::read_to_string(doc_path).unwrap();
    assert!(
        doc.contains(&format!("```json\n{out}```")),
        "la documentation ne reprend plus le flux JSON tel quel :\n{out}"
    );
}
