// ForegramPM — ligne de commande du produit : valider, réécrire, mesurer, dessiner.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! `foregram` ne lit que le format d'échange. Les imports de formats tiers
//! (GraphML, ELK, MCD) et les moteurs de référence restent au banc, dont la
//! sortie arrive ici par l'entrée standard (`-`).
//!
//! Toute l'entrée-sortie est ici : la bibliothèque reçoit des octets et rend
//! des valeurs.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use foregram_graph::json::{self, Kind};
use foregram_graph::{
    Catalog, Diagnostic, Graph, LAYOUT_FORMAT, Layout, Message, Metrics, file_report,
};

/// Tout est valide.
const VALID: u8 = 0;
/// Un fichier au moins est refusé.
const REFUSED: u8 = 1;
/// L'appel est incorrect, ou un fichier n'a pas pu être lu ou écrit.
const UNUSABLE: u8 = 2;

/// Le nom de fichier qui désigne l'entrée standard.
const STDIN: &str = "-";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    ExitCode::from(run(&args))
}

fn run(args: &[OsString]) -> u8 {
    // Les options générales précèdent la commande.
    let mut rest = args;
    loop {
        match rest.first().and_then(|a| a.to_str()) {
            Some("--lang") => match rest.get(1).and_then(|a| a.to_str()) {
                Some("en") => rest = rest.get(2..).unwrap_or(&[]),
                Some(lang) => {
                    emit_err(&say(Message::new("cli.lang-unknown").text("lang", lang)));
                    return UNUSABLE;
                }
                None => return usage(),
            },
            Some("--json") => {
                emit_err(&say(Message::new("cli.json-later")));
                return UNUSABLE;
            }
            _ => break,
        }
    }
    // L'entrée standard ne se lit qu'une fois.
    if rest.iter().filter(|a| a.as_os_str() == STDIN).count() > 1 {
        return usage();
    }

    let command = rest.first().and_then(|a| a.to_str());
    match (command, rest.get(1..).unwrap_or(&[])) {
        (Some("--help"), []) => {
            emit(&say(Message::new("cli.usage")));
            VALID
        }
        (Some("--version"), []) => {
            emit(&format!("foregram {}", env!("CARGO_PKG_VERSION")));
            VALID
        }
        (Some("validate"), [graph, layouts @ ..]) => validate(graph, layouts),
        (Some("canon"), [file]) => canon(file),
        (Some("metrics"), [graph, layouts @ ..]) if !layouts.is_empty() => metrics(graph, layouts),
        (Some("render"), [graph, layout, out]) => render(graph, layout, out.as_ref(), 1),
        (Some("render"), [graph, layout, out, flag, scale]) if flag == "--scale" => {
            match scale.to_str().and_then(|s| s.parse::<i64>().ok()) {
                Some(scale) => render(graph, layout, out.as_ref(), scale),
                None => usage(),
            }
        }
        _ => usage(),
    }
}

fn usage() -> u8 {
    emit_err(&say(Message::new("cli.usage")));
    UNUSABLE
}

/// Un fichier lu : le nom sous lequel le dire, et ses octets.
struct Input {
    name: String,
    bytes: Vec<u8>,
}

/// Lit un fichier, ou l'entrée standard pour `-`. Un échec est dit, et porte
/// le code de sortie à 2.
fn load(arg: &OsStr, status: &mut u8) -> Option<Input> {
    let (name, read) = if arg == STDIN {
        let mut bytes = Vec::new();
        let read = std::io::stdin().read_to_end(&mut bytes).map(|_| bytes);
        (say(Message::new("cli.stdin")), read)
    } else {
        (Path::new(arg).display().to_string(), std::fs::read(arg))
    };
    match read {
        Ok(bytes) => Some(Input { name, bytes }),
        Err(error) => {
            let message = Message::new("cli.unreadable")
                .text("path", &name)
                .text("error", error);
            emit_err(&say(message));
            *status = (*status).max(UNUSABLE);
            None
        }
    }
}

fn refuse(file: &str, diags: &[Diagnostic], status: &mut u8) {
    for line in file_report(file, diags, Catalog::english()) {
        emit_err(&line);
    }
    *status = (*status).max(REFUSED);
}

/// Valide le graphe, puis chaque placement et sa confrontation au graphe.
/// Chaque fichier est lu, même après un refus.
fn validate(graph_arg: &OsStr, layout_args: &[OsString]) -> u8 {
    let mut status = VALID;
    let graph = match load(graph_arg, &mut status) {
        None => None,
        Some(input) => match Graph::parse(&input.bytes) {
            Ok(graph) => {
                let message = Message::new("cli.valid-graph")
                    .text("path", &input.name)
                    .msg("nodes", count_nodes(&graph))
                    .msg("edges", count_edges(&graph));
                emit(&say(message));
                Some(graph)
            }
            Err(diags) => {
                refuse(&input.name, &diags, &mut status);
                None
            }
        },
    };

    for arg in layout_args {
        let Some(input) = load(arg, &mut status) else {
            continue;
        };
        let layout = match Layout::parse(&input.bytes) {
            Ok(layout) => layout,
            Err(diags) => {
                refuse(&input.name, &diags, &mut status);
                continue;
            }
        };
        let Some(graph) = &graph else {
            let message = Message::new("cli.not-checked").text("path", &input.name);
            emit_err(&say(message));
            status = status.max(REFUSED);
            continue;
        };
        let diags = layout.check(graph);
        if diags.is_empty() {
            emit(&say(about_layout("cli.valid-layout", &input.name, &layout)));
        } else {
            refuse(&input.name, &diags, &mut status);
        }
    }
    status
}

/// Relit un graphe ou un placement et l'écrit sous sa forme canonique, en
/// version 2. Les deux se distinguent par leur clé `format` ; à défaut d'en
/// porter une qui dise placement, le fichier est lu comme un graphe, dont les
/// refus diront ce qui manque.
fn canon(arg: &OsStr) -> u8 {
    let mut status = VALID;
    let Some(input) = load(arg, &mut status) else {
        return status;
    };
    let written = if is_layout(&input.bytes) {
        Layout::parse(&input.bytes).map(|layout| layout.to_json())
    } else {
        Graph::parse(&input.bytes).map(|graph| graph.to_json())
    };
    match written {
        Ok(text) => write_out(&text),
        Err(diags) => refuse(&input.name, &diags, &mut status),
    }
    status
}

/// Le texte est un objet JSON dont la clé `format` nomme le placement.
fn is_layout(bytes: &[u8]) -> bool {
    let Ok(value) = json::parse(bytes) else {
        return false;
    };
    let Kind::Object(members) = value.kind else {
        return false;
    };
    members
        .iter()
        .any(|m| m.key == "format" && m.value.kind == Kind::Str(LAYOUT_FORMAT.to_string()))
}

/// Mesure chaque placement pour le graphe. Un placement qui ne passe pas la
/// confrontation au graphe est refusé, pas mesuré.
fn metrics(graph_arg: &OsStr, layout_args: &[OsString]) -> u8 {
    let mut status = VALID;
    let Some(graph) = load_graph(graph_arg, &mut status) else {
        return status;
    };
    for arg in layout_args {
        let Some((name, layout)) = load_layout(arg, &graph, &mut status) else {
            continue;
        };
        emit(&say(about_layout("cli.measured", &name, &layout)));
        for line in Metrics::measure(&graph, &layout).report() {
            emit(&format!("  {}", say(line)));
        }
    }
    status
}

/// Dessine un placement pour son graphe, en SVG ou en PNG selon l'extension
/// du fichier de sortie. Un placement refusé n'est pas dessiné.
fn render(graph_arg: &OsStr, layout_arg: &OsStr, out: &Path, scale: i64) -> u8 {
    let svg = match out.extension().and_then(|e| e.to_str()) {
        Some("svg") => true,
        Some("png") => false,
        _ => return usage(),
    };
    let mut status = VALID;
    let Some(graph) = load_graph(graph_arg, &mut status) else {
        return status;
    };
    let Some((name, layout)) = load_layout(layout_arg, &graph, &mut status) else {
        return status;
    };
    let style = foregram_render::Style::default();
    let image = if svg {
        foregram_render::svg(&graph, &layout, &style).into_bytes()
    } else {
        match foregram_render::png(&graph, &layout, &style, scale) {
            Ok(image) => image,
            Err(diag) => {
                refuse(&out.display().to_string(), &[diag], &mut status);
                return status;
            }
        }
    };
    if let Err(error) = std::fs::write(out, &image) {
        let message = Message::new("cli.unwritable")
            .text("path", out.display())
            .text("error", error);
        emit_err(&say(message));
        return UNUSABLE;
    }
    let message = about_layout("cli.rendered", &name, &layout).text("out", out.display());
    emit(&say(message));
    status
}

fn load_graph(arg: &OsStr, status: &mut u8) -> Option<Graph> {
    let input = load(arg, status)?;
    match Graph::parse(&input.bytes) {
        Ok(graph) => Some(graph),
        Err(diags) => {
            refuse(&input.name, &diags, status);
            None
        }
    }
}

/// Un placement lu et confronté au graphe, avec le nom sous lequel le dire.
fn load_layout(arg: &OsStr, graph: &Graph, status: &mut u8) -> Option<(String, Layout)> {
    let input = load(arg, status)?;
    let layout = match Layout::parse(&input.bytes) {
        Ok(layout) => layout,
        Err(diags) => {
            refuse(&input.name, &diags, status);
            return None;
        }
    };
    let diags = layout.check(graph);
    if diags.is_empty() {
        Some((input.name, layout))
    } else {
        refuse(&input.name, &diags, status);
        None
    }
}

/// Une ligne qui nomme un placement : son fichier, son graphe, son moteur.
fn about_layout(key: &'static str, name: &str, layout: &Layout) -> Message {
    Message::new(key)
        .text("path", name)
        .text("graph", &layout.graph)
        .text("engine", &layout.engine.name)
        .text("version", &layout.engine.version)
}

fn count_nodes(graph: &Graph) -> Message {
    Message::count(
        graph.all_nodes().len(),
        "count.node.one",
        "count.node.other",
    )
}

fn count_edges(graph: &Graph) -> Message {
    Message::count(graph.edges.len(), "count.edge.one", "count.edge.other")
}

/// Compose une phrase en anglais, seule langue pour l'instant.
fn say(message: Message) -> String {
    debug_assert_eq!(message.check(Catalog::english()), Ok(()));
    message.render(Catalog::english())
}

/// Une ligne sur la sortie standard. Un lecteur qui ferme le tube avant la
/// fin (`foregram metrics … | head`) n'est pas une faute de l'appel : la
/// sortie perdue est ignorée, le code de sortie dit toujours la validité.
fn emit(line: &str) {
    write_out(&format!("{line}\n"));
}

fn write_out(text: &str) {
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
}

/// Une ligne sur la sortie d'erreur, dans les mêmes conditions.
fn emit_err(line: &str) {
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}
