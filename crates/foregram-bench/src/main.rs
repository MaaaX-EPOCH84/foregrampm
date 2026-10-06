// ForegramPM — banc de mesure du placement : ligne de commande.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use foregram_graph::{Diagnostic, Graph, Layout};

const USAGE: &str = "\
usage: foregram-bench validate <graph.json> [<layout.json>...]
       foregram-bench --help | --version

  validate  Validates a graph, then each layout against that graph.

exit codes: 0 everything is valid, 1 a file is refused,
            2 wrong call or unreadable file
";

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
            print!("{USAGE}");
            VALID
        }
        (Some("--version"), []) => {
            println!("foregram-bench {}", env!("CARGO_PKG_VERSION"));
            VALID
        }
        (Some("validate"), [graph, layouts @ ..]) => validate(graph.as_ref(), layouts),
        _ => {
            eprint!("{USAGE}");
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
                println!(
                    "{}: valid graph ({}, {})",
                    graph_path.display(),
                    count(graph.nodes.len(), "node"),
                    count(graph.edges.len(), "edge")
                );
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
            eprintln!(
                "{}: not checked against the graph, which is not valid",
                path.display()
            );
            status = status.max(REFUSED);
            continue;
        };
        let diags = layout.check(graph);
        if diags.is_empty() {
            println!(
                "{}: valid layout of graph \"{}\" (engine {} {})",
                path.display(),
                layout.graph,
                layout.engine.name,
                layout.engine.version
            );
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
            eprintln!("{}: cannot read: {error}", path.display());
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
    eprintln!(
        "{}: refused ({})",
        path.display(),
        count(diags.len(), "error")
    );
    *status = (*status).max(REFUSED);
}

fn count(number: usize, noun: &str) -> String {
    if number < 2 {
        format!("{number} {noun}")
    } else {
        format!("{number} {noun}s")
    }
}
