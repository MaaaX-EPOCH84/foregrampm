// ForegramPM — banc de mesure du placement : ligne de commande.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

mod graphml;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use foregram_graph::{Catalog, Diagnostic, Graph, Layout, Message};

/// Tout est valide.
const VALID: u8 = 0;
/// Un fichier au moins est refusé.
const REFUSED: u8 = 1;
/// L'appel est incorrect, ou un fichier n'a pas pu être lu.
const UNUSABLE: u8 = 2;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let command = args.first().and_then(|a| a.to_str());
    let status = match (command, args.get(1..).unwrap_or(&[])) {
        (Some("--help"), []) => {
            println!("{}", say(Message::new("bench.usage")));
            VALID
        }
        (Some("--version"), []) => {
            println!("foregram-bench {}", env!("CARGO_PKG_VERSION"));
            VALID
        }
        (Some("validate"), [graph, layouts @ ..]) => validate(graph.as_ref(), layouts),
        (Some("graphml"), [path]) => import_graphml(path.as_ref()),
        (Some("check-corpus"), [cache]) => check_corpus(cache.as_ref()),
        _ => {
            eprintln!("{}", say(Message::new("bench.usage")));
            UNUSABLE
        }
    };
    ExitCode::from(status)
}

/// Valide un graphe et ses placements. Va au bout : un fichier refusé
/// n'empêche pas de lire les suivants.
fn validate(graph_path: &Path, layout_paths: &[OsString]) -> u8 {
    let mut status = VALID;
    let graph = match load(graph_path, &mut status) {
        None => None,
        Some(bytes) => match Graph::parse(&bytes) {
            Ok(graph) => {
                let message = Message::new("bench.valid-graph")
                    .text("path", graph_path.display())
                    .msg("nodes", count(graph.all_nodes().len(), "node"))
                    .msg("edges", count(graph.edges.len(), "edge"));
                println!("{}", say(message));
                Some(graph)
            }
            Err(diags) => {
                refuse(graph_path, &diags, &mut status);
                None
            }
        },
    };

    for path in layout_paths {
        let path: &Path = path.as_ref();
        let Some(bytes) = load(path, &mut status) else {
            continue;
        };
        let layout = match Layout::parse(&bytes) {
            Ok(layout) => layout,
            Err(diags) => {
                refuse(path, &diags, &mut status);
                continue;
            }
        };
        let Some(graph) = &graph else {
            let message = Message::new("bench.not-checked").text("path", path.display());
            eprintln!("{}", say(message));
            status = status.max(REFUSED);
            continue;
        };
        let diags = layout.check(graph);
        if diags.is_empty() {
            let message = Message::new("bench.valid-layout")
                .text("path", path.display())
                .text("graph", &layout.graph)
                .text("engine", &layout.engine.name)
                .text("version", &layout.engine.version);
            println!("{}", say(message));
        } else {
            refuse(path, &diags, &mut status);
        }
    }
    status
}

/// Convertit un fichier GraphML et écrit le graphe obtenu, sous sa forme
/// canonique, sur la sortie standard.
fn import_graphml(path: &Path) -> u8 {
    let mut status = VALID;
    let Some(bytes) = load(path, &mut status) else {
        return status;
    };
    match graphml::import(&graph_name(path), &bytes, graphml::NODE_SIZE) {
        Ok(graph) => print!("{}", graph.to_json()),
        Err(diags) => refuse(path, &diags, &mut status),
    }
    status
}

/// Le nom du graphe importé : celui du fichier, sans `.graphml`.
fn graph_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Les corpus GraphML, sous-répertoires du cache de `corpus/fetch.sh`.
const GRAPHML_CORPORA: [&str; 2] = ["rome", "north"];

/// Convertit chaque graphe de Rome et North, et confronte ses nombres de
/// nœuds et de liens à ceux que le `Graph.log` de l'archive annonce : une
/// source indépendante du lecteur, qui voit ce qu'un import aurait perdu.
fn check_corpus(cache: &Path) -> u8 {
    let mut status = VALID;
    for corpus in GRAPHML_CORPORA {
        let dir = cache.join(corpus);
        let log_path = dir.join("Graph.log");
        let Some(log) = load(&log_path, &mut status) else {
            continue;
        };
        let mut problems = 0;
        let mut expected = read_graph_log(&log_path, &log, &mut problems);
        let files = match graphml_files(&dir) {
            Ok(files) => files,
            Err(error) => {
                let message = Message::new("bench.unreadable")
                    .text("path", dir.display())
                    .text("error", error);
                eprintln!("{}", say(message));
                status = status.max(UNUSABLE);
                continue;
            }
        };
        let (mut nodes, mut edges) = (0, 0);
        for path in &files {
            let Some(bytes) = load(path, &mut status) else {
                problems += 1;
                continue;
            };
            let name = graph_name(path);
            let graph = match graphml::import(&name, &bytes, graphml::NODE_SIZE) {
                Ok(graph) => graph,
                Err(diags) => {
                    refuse(path, &diags, &mut status);
                    problems += 1;
                    expected.remove(&name);
                    continue;
                }
            };
            let found = (graph.all_nodes().len(), graph.edges.len());
            nodes += found.0;
            edges += found.1;
            let message = match expected.remove(&name) {
                None => Message::new("bench.log-absent").text("path", path.display()),
                Some(announced) if announced == found => continue,
                Some(announced) => Message::new("bench.log-differs")
                    .text("path", path.display())
                    .msg("nodes", count(found.0, "node"))
                    .msg("edges", count(found.1, "edge"))
                    .msg("log-nodes", count(announced.0, "node"))
                    .msg("log-edges", count(announced.1, "edge")),
            };
            eprintln!("{}", say(message));
            problems += 1;
        }
        for name in expected.keys() {
            let message = Message::new("bench.log-orphan")
                .text("path", log_path.display())
                .text("name", name);
            eprintln!("{}", say(message));
            problems += 1;
        }
        let graphs = count(files.len(), "graph");
        if problems == 0 {
            let message = Message::new("bench.corpus-ok")
                .text("corpus", corpus)
                .msg("graphs", graphs)
                .msg("nodes", count(nodes, "node"))
                .msg("edges", count(edges, "edge"));
            println!("{}", say(message));
        } else {
            let message = Message::new("bench.corpus-failed")
                .text("corpus", corpus)
                .msg("graphs", graphs)
                .msg("problems", count(problems, "problem"));
            eprintln!("{}", say(message));
            status = status.max(REFUSED);
        }
    }
    status
}

/// Les `.graphml` d'un répertoire, triés par nom : l'ordre de parcours du
/// système de fichiers ne doit rien changer à la sortie.
fn graphml_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "graphml") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Lit un `Graph.log` : par graphe, une ligne `name: … nodes: … edges: …`,
/// entre d'autres lignes sans intérêt ici. Rend, par nom de graphe, les
/// nombres annoncés de nœuds et de liens.
fn read_graph_log(
    path: &Path,
    bytes: &[u8],
    problems: &mut usize,
) -> BTreeMap<String, (usize, usize)> {
    let mut graphs = BTreeMap::new();
    let text = String::from_utf8_lossy(bytes);
    for (index, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.first() != Some(&"name:") {
            continue;
        }
        let after = |label: &str| {
            let at = words.iter().position(|word| *word == label)?;
            words.get(at + 1).copied()
        };
        let number = |label: &str| after(label).and_then(|word| word.parse::<usize>().ok());
        match (after("name:"), number("nodes:"), number("edges:")) {
            (Some(name), Some(nodes), Some(edges))
                if graphs.insert(name.to_string(), (nodes, edges)).is_none() => {}
            _ => {
                let message = Message::new("bench.log-unreadable")
                    .text("path", path.display())
                    .text("line", index + 1);
                eprintln!("{}", say(message));
                *problems += 1;
            }
        }
    }
    graphs
}

fn load(path: &Path, status: &mut u8) -> Option<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            let message = Message::new("bench.unreadable")
                .text("path", path.display())
                .text("error", error);
            eprintln!("{}", say(message));
            *status = (*status).max(UNUSABLE);
            None
        }
    }
}

fn refuse(path: &Path, diags: &[Diagnostic], status: &mut u8) {
    for diag in diags {
        if diag.pos.is_none() {
            eprintln!("{}: {diag}", path.display());
        } else {
            eprintln!("{}:{diag}", path.display());
        }
    }
    let message = Message::new("bench.refused")
        .text("path", path.display())
        .msg("count", count(diags.len(), "error"));
    eprintln!("{}", say(message));
    *status = (*status).max(REFUSED);
}

/// Compose une phrase en anglais, seule langue pour l'instant.
fn say(message: Message) -> String {
    debug_assert_eq!(message.check(Catalog::english()), Ok(()));
    message.render(Catalog::english())
}

/// « 1 node », « 3 nodes ». Le choix suit la règle anglaise du pluriel,
/// « one » pour 1 seulement ; une autre langue apportera sa propre règle.
fn count(number: usize, noun: &str) -> Message {
    let key = match (noun, number) {
        ("node", 1) => "count.node.one",
        ("node", _) => "count.node.other",
        ("edge", 1) => "count.edge.one",
        ("edge", _) => "count.edge.other",
        ("graph", 1) => "count.graph.one",
        ("graph", _) => "count.graph.other",
        ("problem", 1) => "count.problem.one",
        ("problem", _) => "count.problem.other",
        (_, 1) => "count.error.one",
        _ => "count.error.other",
    };
    Message::new(key).text("n", number)
}
