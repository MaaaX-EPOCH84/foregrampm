// ForegramPM — ligne de commande du produit : valider, réécrire, mesurer, dessiner.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! `foregram` ne lit que le format d'échange. Les imports de formats tiers
//! (GraphML, ELK, MCD) et les moteurs de référence restent au banc, dont la
//! sortie arrive ici par l'entrée standard (`-`).
//!
//! Toute l'entrée-sortie est ici : la bibliothèque reçoit des octets et rend
//! des valeurs.
//!
//! Sous `--json`, chaque résultat et chaque refus est un objet JSON sur une
//! ligne, tous sur la sortie standard et dans l'ordre des fichiers ; l'erreur
//! standard ne porte plus que l'usage.
//!
//! `--lang` choisit le catalogue des phrases ; sans lui, l'anglais. Aucune
//! variable d'environnement n'y entre : même appel, mêmes octets.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use foregram_graph::json::{self, Kind, quote};
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
    let mut out = Out {
        json: false,
        catalog: Catalog::english(),
    };
    loop {
        match rest.first().and_then(|a| a.to_str()) {
            Some("--lang") => match rest.get(1).and_then(|a| a.to_str()) {
                Some(lang) => match Catalog::for_language(lang) {
                    Some(catalog) => {
                        out.catalog = catalog;
                        rest = rest.get(2..).unwrap_or(&[]);
                    }
                    None => {
                        emit_err(&out.say(Message::new("cli.lang-unknown").text("lang", lang)));
                        return UNUSABLE;
                    }
                },
                None => return usage(out),
            },
            Some("--json") => {
                out.json = true;
                rest = rest.get(1..).unwrap_or(&[]);
            }
            _ => break,
        }
    }
    // L'entrée standard ne se lit qu'une fois.
    if rest.iter().filter(|a| a.as_os_str() == STDIN).count() > 1 {
        return usage(out);
    }

    let command = rest.first().and_then(|a| a.to_str());
    match (command, rest.get(1..).unwrap_or(&[])) {
        (Some("--help"), []) => {
            emit(&out.say(Message::new("cli.usage")));
            VALID
        }
        (Some("--version"), []) => {
            emit(&format!("foregram {}", env!("CARGO_PKG_VERSION")));
            VALID
        }
        (Some("validate"), [graph, layouts @ ..]) => validate(out, graph, layouts),
        (Some("canon"), [file]) => canon(out, file),
        (Some("metrics"), [graph, layouts @ ..]) if !layouts.is_empty() => {
            metrics(out, graph, layouts)
        }
        (Some("render"), [graph, layout, image]) => render(out, graph, layout, image.as_ref(), 1),
        (Some("render"), [graph, layout, image, flag, scale]) if flag == "--scale" => {
            match scale.to_str().and_then(|s| s.parse::<i64>().ok()) {
                Some(scale) => render(out, graph, layout, image.as_ref(), scale),
                None => usage(out),
            }
        }
        _ => usage(out),
    }
}

fn usage(out: Out) -> u8 {
    emit_err(&out.say(Message::new("cli.usage")));
    UNUSABLE
}

/// La façon d'écrire les résultats : des lignes de texte, les refus sur
/// l'erreur standard ; ou, sous `--json`, un objet par ligne, tout sur la
/// sortie standard. Les phrases se composent dans la langue du catalogue.
#[derive(Clone, Copy)]
struct Out {
    json: bool,
    catalog: &'static Catalog,
}

impl Out {
    /// Compose une phrase dans la langue choisie.
    fn say(self, message: Message) -> String {
        debug_assert_eq!(message.check(self.catalog), Ok(()));
        message.render(self.catalog)
    }

    /// Un résultat : un fichier valide, mesuré ou dessiné.
    fn said(self, kind: &str, file: &str, message: Message) {
        if self.json {
            emit(&self.object(kind, file, &message, ""));
        } else {
            emit(&self.say(message));
        }
    }

    /// Un fichier qui n'a pu être lu, écrit ou confronté au graphe.
    fn error(self, file: &str, message: Message) {
        if self.json {
            emit(&self.object("error", file, &message, ""));
        } else {
            emit_err(&self.say(message));
        }
    }

    fn refuse(self, file: &str, diags: &[Diagnostic], status: &mut u8) {
        if self.json {
            for diag in diags {
                emit(&diag.to_json(file, self.catalog));
            }
        } else {
            for line in file_report(file, diags, self.catalog) {
                emit_err(&line);
            }
        }
        *status = (*status).max(REFUSED);
    }

    /// Un objet du flux JSON hors refus : sa sorte, son fichier, son message
    /// en clé et arguments, sa phrase, puis ce que la sorte ajoute (`extra`,
    /// membres précédés d'une virgule).
    fn object(self, kind: &str, file: &str, message: &Message, extra: &str) -> String {
        format!(
            "{{\"kind\": {}, \"file\": {}, \"key\": {}, \"args\": {}, \"text\": {}{extra}}}",
            quote(kind),
            quote(file),
            quote(message.key),
            message.args_json(),
            quote(&self.say(message.clone()))
        )
    }
}

/// Un fichier lu : le nom sous lequel le dire, et ses octets.
struct Input {
    name: String,
    bytes: Vec<u8>,
}

/// Lit un fichier, ou l'entrée standard pour `-`. Un échec est dit, et porte
/// le code de sortie à 2.
fn load(out: Out, arg: &OsStr, status: &mut u8) -> Option<Input> {
    let (name, read) = if arg == STDIN {
        let mut bytes = Vec::new();
        let read = std::io::stdin().read_to_end(&mut bytes).map(|_| bytes);
        (out.say(Message::new("cli.stdin")), read)
    } else {
        (Path::new(arg).display().to_string(), std::fs::read(arg))
    };
    match read {
        Ok(bytes) => Some(Input { name, bytes }),
        Err(error) => {
            let message = Message::new("cli.unreadable")
                .text("path", &name)
                .text("error", error);
            out.error(&name, message);
            *status = (*status).max(UNUSABLE);
            None
        }
    }
}

/// Valide le graphe, puis chaque placement et sa confrontation au graphe.
/// Chaque fichier est lu, même après un refus.
fn validate(out: Out, graph_arg: &OsStr, layout_args: &[OsString]) -> u8 {
    let mut status = VALID;
    let graph = match load(out, graph_arg, &mut status) {
        None => None,
        Some(input) => match Graph::parse(&input.bytes) {
            Ok(graph) => {
                let message = Message::new("cli.valid-graph")
                    .text("path", &input.name)
                    .msg("nodes", count_nodes(&graph))
                    .msg("edges", count_edges(&graph));
                out.said("valid", &input.name, message);
                Some(graph)
            }
            Err(diags) => {
                out.refuse(&input.name, &diags, &mut status);
                None
            }
        },
    };

    for arg in layout_args {
        let Some(input) = load(out, arg, &mut status) else {
            continue;
        };
        let layout = match Layout::parse(&input.bytes) {
            Ok(layout) => layout,
            Err(diags) => {
                out.refuse(&input.name, &diags, &mut status);
                continue;
            }
        };
        let Some(graph) = &graph else {
            let message = Message::new("cli.not-checked").text("path", &input.name);
            out.error(&input.name, message);
            status = status.max(REFUSED);
            continue;
        };
        let diags = layout.check(graph);
        if diags.is_empty() {
            let message = about_layout("cli.valid-layout", &input.name, &layout);
            out.said("valid", &input.name, message);
        } else {
            out.refuse(&input.name, &diags, &mut status);
        }
    }
    status
}

/// Relit un graphe ou un placement et l'écrit sous sa forme canonique, en
/// version 2. Les deux se distinguent par leur clé `format` ; à défaut d'en
/// porter une qui dise placement, le fichier est lu comme un graphe, dont les
/// refus diront ce qui manque.
fn canon(out: Out, arg: &OsStr) -> u8 {
    let mut status = VALID;
    let Some(input) = load(out, arg, &mut status) else {
        return status;
    };
    let written = if is_layout(&input.bytes) {
        Layout::parse(&input.bytes).map(|layout| layout.to_json())
    } else {
        Graph::parse(&input.bytes).map(|graph| graph.to_json())
    };
    match written {
        Ok(text) if out.json => write_out(&canonical(&input.name, &text)),
        Ok(text) => write_out(&text),
        Err(diags) => out.refuse(&input.name, &diags, &mut status),
    }
    status
}

/// La forme canonique dans son objet du flux JSON : la seule sur plusieurs
/// lignes, le document tel quel sous `document`, décalé d'un cran.
fn canonical(file: &str, text: &str) -> String {
    let document = text.trim_end_matches('\n').replace('\n', "\n  ");
    format!(
        "{{\n  \"kind\": \"canonical\",\n  \"file\": {},\n  \"document\": {document}\n}}\n",
        quote(file)
    )
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
fn metrics(out: Out, graph_arg: &OsStr, layout_args: &[OsString]) -> u8 {
    let mut status = VALID;
    let Some(graph) = load_graph(out, graph_arg, &mut status) else {
        return status;
    };
    for arg in layout_args {
        let Some((name, layout)) = load_layout(out, arg, &graph, &mut status) else {
            continue;
        };
        let header = about_layout("cli.measured", &name, &layout);
        let measured = Metrics::measure(&graph, &layout);
        if out.json {
            let extra = format!(", \"metrics\": {}", measured.to_json());
            emit(&out.object("metrics", &name, &header, &extra));
            continue;
        }
        emit(&out.say(header));
        for line in measured.report() {
            emit(&format!("  {}", out.say(line)));
        }
    }
    status
}

/// Dessine un placement pour son graphe, en SVG ou en PNG selon l'extension
/// du fichier de sortie. Un placement refusé n'est pas dessiné.
fn render(out: Out, graph_arg: &OsStr, layout_arg: &OsStr, image_path: &Path, scale: i64) -> u8 {
    let svg = match image_path.extension().and_then(|e| e.to_str()) {
        Some("svg") => true,
        Some("png") => false,
        _ => return usage(out),
    };
    let mut status = VALID;
    let Some(graph) = load_graph(out, graph_arg, &mut status) else {
        return status;
    };
    let Some((name, layout)) = load_layout(out, layout_arg, &graph, &mut status) else {
        return status;
    };
    let style = foregram_render::Style::default();
    let image = if svg {
        foregram_render::svg(&graph, &layout, &style).into_bytes()
    } else {
        match foregram_render::png(&graph, &layout, &style, scale) {
            Ok(image) => image,
            Err(diag) => {
                out.refuse(&image_path.display().to_string(), &[diag], &mut status);
                return status;
            }
        }
    };
    if let Err(error) = std::fs::write(image_path, &image) {
        let path = image_path.display().to_string();
        let message = Message::new("cli.unwritable")
            .text("path", &path)
            .text("error", error);
        out.error(&path, message);
        return UNUSABLE;
    }
    let message = about_layout("cli.rendered", &name, &layout).text("out", image_path.display());
    out.said("rendered", &name, message);
    status
}

fn load_graph(out: Out, arg: &OsStr, status: &mut u8) -> Option<Graph> {
    let input = load(out, arg, status)?;
    match Graph::parse(&input.bytes) {
        Ok(graph) => Some(graph),
        Err(diags) => {
            out.refuse(&input.name, &diags, status);
            None
        }
    }
}

/// Un placement lu et confronté au graphe, avec le nom sous lequel le dire.
fn load_layout(out: Out, arg: &OsStr, graph: &Graph, status: &mut u8) -> Option<(String, Layout)> {
    let input = load(out, arg, status)?;
    let layout = match Layout::parse(&input.bytes) {
        Ok(layout) => layout,
        Err(diags) => {
            out.refuse(&input.name, &diags, status);
            return None;
        }
    };
    let diags = layout.check(graph);
    if diags.is_empty() {
        Some((input.name, layout))
    } else {
        out.refuse(&input.name, &diags, status);
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
    Message::count(graph.all_nodes().len(), "count.node")
}

fn count_edges(graph: &Graph) -> Message {
    Message::count(graph.edges.len(), "count.edge")
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
