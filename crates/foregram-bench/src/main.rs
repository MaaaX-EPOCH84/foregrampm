// ForegramPM — banc de mesure du placement : ligne de commande.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

mod elk;
mod elkt;
mod engine;
mod graphml;
mod lines;
mod mcd;

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
        (Some("elkt"), [path]) => import_elkt(path.as_ref()),
        (Some("mcd"), [path]) => import_mcd(path.as_ref()),
        (Some("check-corpus"), [cache]) => check_corpus(cache.as_ref()),
        (Some("layout"), [engine, cache, graph]) => match engine_kind(engine) {
            Some(kind) => layout_one(kind, cache.as_ref(), graph.as_ref()),
            None => usage(),
        },
        (Some("layout-corpus"), [engine, cache, out]) => match engine_kind(engine) {
            Some(kind) => layout_corpus(kind, cache.as_ref(), out.as_ref()),
            None => usage(),
        },
        _ => usage(),
    };
    ExitCode::from(status)
}

fn usage() -> u8 {
    eprintln!("{}", say(Message::new("bench.usage")));
    UNUSABLE
}

fn engine_kind(word: &OsString) -> Option<engine::Kind> {
    word.to_str().and_then(engine::Kind::parse)
}

/// Place un graphe avec un moteur de référence et écrit le placement, sous sa
/// forme canonique, sur la sortie standard — même refusé : les refus vont sur
/// la sortie d'erreur, et se lisent mieux avec le fichier sous les yeux.
fn layout_one(kind: engine::Kind, cache: &Path, graph_path: &Path) -> u8 {
    let mut status = VALID;
    let Some(bytes) = load(graph_path, &mut status) else {
        return status;
    };
    let graph = match import_any(graph_path, &bytes) {
        Ok(graph) => graph,
        Err(diags) => {
            refuse(graph_path, &diags, &mut status);
            return status;
        }
    };
    let mut runner = match engine::Tools::locate(kind, cache).and_then(engine::Runner::start) {
        Ok(runner) => runner,
        Err(diag) => {
            eprintln!("{diag}");
            return UNUSABLE;
        }
    };
    let outcome = runner.layout(&graph);
    if let Some(layout) = &outcome.layout {
        print!("{}", layout.to_json());
    }
    if outcome.fit.moved > 0 {
        eprintln!("{}", say(fitted(graph_path, outcome.fit)));
    }
    if outcome.diags.is_empty() {
        let message = Message::new("bench.laid-out")
            .text("path", graph_path.display())
            .text("engine", kind.name());
        eprintln!("{}", say(message));
    } else {
        refuse(graph_path, &outcome.diags, &mut status);
    }
    status
}

fn fitted(path: &Path, fit: engine::fit::Stats) -> Message {
    Message::new("bench.fitted")
        .text("path", path.display())
        .msg("moved", count(fit.moved, "end"))
        .text("max", fit.max)
}

/// Lit un graphe quel que soit son format, d'après l'extension du fichier.
fn import_any(path: &Path, bytes: &[u8]) -> Result<Graph, Vec<Diagnostic>> {
    let name = graph_name(path);
    match path.extension().and_then(|e| e.to_str()) {
        Some("graphml") => graphml::import(&name, bytes, graphml::NODE_SIZE),
        Some("elkt") => elk::import(&name, bytes).map(|imported| imported.graph),
        Some("mcd") => mcd::import(&name, bytes),
        _ => Graph::parse(bytes),
    }
}

/// Les corpus que les moteurs de référence placent : répertoire sous le cache
/// (ou à côté, pour le corpus maison) et extension des fichiers.
const LAYOUT_CORPORA: [(&str, &str); 4] = [
    ("rome", "graphml"),
    ("north", "graphml"),
    ("elk-models", "elkt"),
    ("../mcd", "mcd"),
];

/// Place chaque graphe des corpus avec un moteur de référence. Les placements
/// vont dans `<out>/<moteur>/<corpus>/…layout.json`, refusés compris ; chaque
/// échec est listé dans `<out>/<moteur>/failures.txt`, et compté.
fn layout_corpus(kind: engine::Kind, cache: &Path, out: &Path) -> u8 {
    let mut status = VALID;
    let mut runner = match engine::Tools::locate(kind, cache).and_then(engine::Runner::start) {
        Ok(runner) => runner,
        Err(diag) => {
            eprintln!("{diag}");
            return UNUSABLE;
        }
    };
    let engine_dir = out.join(kind.name());
    let failures_path = engine_dir.join("failures.txt");
    let mut failures = String::new();
    for (corpus, extension) in LAYOUT_CORPORA {
        let dir = cache.join(corpus);
        let files = match files_with(&dir, extension) {
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
        let corpus_name = corpus.trim_start_matches("../");
        let (mut valid, mut failed) = (0, 0);
        let mut fit = engine::fit::Stats::default();
        for path in &files {
            let shown = path.strip_prefix(&dir).unwrap_or(path);
            let shown = Path::new(corpus_name).join(shown);
            let Some(bytes) = load(path, &mut status) else {
                failed += 1;
                continue;
            };
            let graph = match import_any(path, &bytes) {
                Ok(graph) => graph,
                Err(diags) => {
                    list_failure(&mut failures, &shown, &diags);
                    failed += 1;
                    continue;
                }
            };
            let outcome = runner.layout(&graph);
            if let Some(layout) = &outcome.layout {
                let target = engine_dir.join(&shown).with_extension("layout.json");
                if let Err(error) = write_file(&target, layout.to_json().as_bytes()) {
                    let message = Message::new("bench.unwritable")
                        .text("path", target.display())
                        .text("error", error);
                    eprintln!("{}", say(message));
                    return UNUSABLE;
                }
            }
            fit.moved += outcome.fit.moved;
            fit.max = fit.max.max(outcome.fit.max);
            if outcome.diags.is_empty() {
                valid += 1;
            } else {
                list_failure(&mut failures, &shown, &outcome.diags);
                failed += 1;
            }
        }
        let message = Message::new("bench.corpus-laid-out")
            .text("engine", kind.name())
            .text("corpus", corpus_name)
            .msg("graphs", count(files.len(), "graph"))
            .text("valid", valid)
            .msg("failed", count(failed, "failure"))
            .msg("moved", count(fit.moved, "end"))
            .text("max", fit.max);
        println!("{}", say(message));
        if failed > 0 {
            status = status.max(REFUSED);
        }
    }
    if let Err(error) = write_file(&failures_path, failures.as_bytes()) {
        let message = Message::new("bench.unwritable")
            .text("path", failures_path.display())
            .text("error", error);
        eprintln!("{}", say(message));
        return UNUSABLE;
    }
    let message = Message::new("bench.failures-listed").text("path", failures_path.display());
    println!("{}", say(message));
    status
}

fn list_failure(failures: &mut String, shown: &Path, diags: &[Diagnostic]) {
    for diag in diags {
        let separator = if diag.pos.is_none() { ": " } else { ":" };
        failures.push_str(&format!("{}{separator}{diag}\n", shown.display()));
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
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

/// Convertit un fichier `.elkt` et écrit le graphe obtenu, sous sa forme
/// canonique, sur la sortie standard. Ce qui n'est pas passé est dit sur la
/// sortie d'erreur.
fn import_elkt(path: &Path) -> u8 {
    let mut status = VALID;
    let Some(bytes) = load(path, &mut status) else {
        return status;
    };
    match elk::import(&graph_name(path), &bytes) {
        Ok(imported) => {
            print!("{}", imported.graph.to_json());
            if imported.notes != elk::Notes::default() {
                eprintln!("{}", say(notes(path, imported.notes)));
            }
        }
        Err(diags) => refuse(path, &diags, &mut status),
    }
    status
}

/// Convertit un MCD et écrit le graphe obtenu, sous sa forme canonique, sur
/// la sortie standard.
fn import_mcd(path: &Path) -> u8 {
    let mut status = VALID;
    let Some(bytes) = load(path, &mut status) else {
        return status;
    };
    match mcd::import(&graph_name(path), &bytes) {
        Ok(graph) => print!("{}", graph.to_json()),
        Err(diags) => refuse(path, &diags, &mut status),
    }
    status
}

fn notes(path: &Path, notes: elk::Notes) -> Message {
    Message::new("bench.not-carried")
        .text("path", path.display())
        .msg("labels", count(notes.labels, "label"))
        .msg("sizeless", count(notes.sizeless, "sizeless"))
        .msg(
            "container-ports",
            count(notes.container_ports, "container-port"),
        )
}

/// Le nom du graphe importé : celui du fichier, sans son extension.
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
        let files = match files_with(&dir, "graphml") {
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
    status.max(check_elk_models(cache))
}

/// Le manifeste des corpus, voisin du cache : `corpus/manifest.txt`.
const MANIFEST: &str = concat!("manifest", ".txt");

/// Convertit chaque `.elkt` d'elk-models. Pas de `Graph.log` ici : la
/// source indépendante est le manifeste, voisin du cache, qui dit combien de
/// fichiers il y a et combien doivent s'importer ; chaque graphe importé est
/// validé.
fn check_elk_models(cache: &Path) -> u8 {
    let mut status = VALID;
    let dir = cache.join("elk-models");
    let manifest = cache.parent().unwrap_or(cache).join(MANIFEST);
    let Some(bytes) = load(&manifest, &mut status) else {
        return status;
    };
    let expected = manifest_counts(&bytes);
    let files = match files_with(&dir, "elkt") {
        Ok(files) => files,
        Err(error) => {
            let message = Message::new("bench.unreadable")
                .text("path", dir.display())
                .text("error", error);
            eprintln!("{}", say(message));
            return UNUSABLE;
        }
    };
    let mut problems = 0;
    let (mut imported, mut nodes, mut edges) = (0, 0, 0);
    let mut notes = elk::Notes::default();
    for path in &files {
        let Some(bytes) = load(path, &mut status) else {
            problems += 1;
            continue;
        };
        match elk::import(&graph_name(path), &bytes) {
            Ok(result) => {
                imported += 1;
                nodes += result.graph.all_nodes().len();
                edges += result.graph.edges.len();
                notes.labels += result.notes.labels;
                notes.sizeless += result.notes.sizeless;
                notes.container_ports += result.notes.container_ports;
            }
            Err(diags) => {
                refuse(path, &diags, &mut status);
                problems += 1;
            }
        }
    }
    if (files.len(), imported) != expected {
        let message = Message::new("bench.manifest-differs")
            .text("path", manifest.display())
            .msg("files", count(files.len(), "file"))
            .text("imported", imported)
            .msg("manifest-files", count(expected.0, "file"))
            .text("manifest-imported", expected.1);
        eprintln!("{}", say(message));
        problems += 1;
    }
    let graphs = count(files.len(), "graph");
    if problems == 0 {
        let message = Message::new("bench.elk-ok")
            .text("corpus", "elk-models")
            .msg("graphs", graphs)
            .msg("nodes", count(nodes, "node"))
            .msg("edges", count(edges, "edge"))
            .msg("labels", count(notes.labels, "label"))
            .msg("sizeless", count(notes.sizeless, "sizeless"))
            .msg(
                "container-ports",
                count(notes.container_ports, "container-port"),
            );
        println!("{}", say(message));
    } else {
        let message = Message::new("bench.corpus-failed")
            .text("corpus", "elk-models")
            .msg("graphs", graphs)
            .msg("problems", count(problems, "problem"));
        eprintln!("{}", say(message));
        status = status.max(REFUSED);
    }
    status
}

/// Dans la section `[elk-models]` du manifeste, `count` et `imported`.
fn manifest_counts(bytes: &[u8]) -> (usize, usize) {
    let text = String::from_utf8_lossy(bytes);
    let section = text.split("[elk-models]").nth(1).unwrap_or("");
    let section = section.split("\n[").next().unwrap_or("");
    let value = |key: &str| {
        section
            .lines()
            .find_map(|line| {
                line.strip_prefix(key)?
                    .trim()
                    .strip_prefix('=')?
                    .trim()
                    .parse::<usize>()
                    .ok()
            })
            .unwrap_or(0)
    };
    (value("count"), value("imported"))
}

/// Les fichiers d'une extension sous un répertoire, sous-répertoires
/// compris, triés par chemin.
fn files_with(dir: &Path, extension: &str) -> std::io::Result<Vec<PathBuf>> {
    fn walk(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(&path, extension, out)?;
            } else if path.extension().is_some_and(|ext| ext == extension) {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(dir, extension, &mut files)?;
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
        ("label", 1) => "count.label.one",
        ("label", _) => "count.label.other",
        ("sizeless", 1) => "count.sizeless.one",
        ("sizeless", _) => "count.sizeless.other",
        ("container-port", 1) => "count.container-port.one",
        ("container-port", _) => "count.container-port.other",
        ("file", 1) => "count.file.one",
        ("file", _) => "count.file.other",
        ("end", 1) => "count.end.one",
        ("end", _) => "count.end.other",
        ("failure", 1) => "count.failure.one",
        ("failure", _) => "count.failure.other",
        (_, 1) => "count.error.one",
        _ => "count.error.other",
    };
    Message::new(key).text("n", number)
}
