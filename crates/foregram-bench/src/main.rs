// ForegramPM — banc de mesure du placement : ligne de commande.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::OsString;
use std::path::Path;
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
        (_, 1) => "count.error.one",
        _ => "count.error.other",
    };
    Message::new(key).text("n", number)
}
