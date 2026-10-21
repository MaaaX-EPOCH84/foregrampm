// ForegramPM — the product's command line: validate, rewrite, measure, draw.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! `foregram` reads only the interchange format. Imports of third-party formats
//! (GraphML, ELK, MCD) and the reference engines stay in the bench, whose
//! output arrives here on standard input (`-`).
//!
//! All input/output lives here: the library receives bytes and returns
//! values.
//!
//! Under `--json`, every result and every refusal is a JSON object on one
//! line, all on standard output and in file order; standard error
//! carries only the usage.
//!
//! `--lang` picks the phrase catalogue; without it, English. No
//! environment variable plays a part: same call, same bytes.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use foregram_graph::json::{self, Kind, quote};
use foregram_graph::{
    Catalog, Diagnostic, Graph, LAYOUT_FORMAT, Layout, Message, Metrics, file_report,
};

/// Everything is valid.
const VALID: u8 = 0;
/// At least one file is refused.
const REFUSED: u8 = 1;
/// The call is incorrect, or a file could not be read or written.
const UNUSABLE: u8 = 2;

/// The file name that stands for standard input.
const STDIN: &str = "-";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    ExitCode::from(run(&args))
}

fn run(args: &[OsString]) -> u8 {
    // General options come before the command.
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
    // Standard input can be read only once.
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

/// How results are written: lines of text, refusals on
/// standard error; or, under `--json`, one object per line, all on
/// standard output. Phrases are composed in the catalogue's language.
#[derive(Clone, Copy)]
struct Out {
    json: bool,
    catalog: &'static Catalog,
}

impl Out {
    /// Composes a phrase in the chosen language.
    fn say(self, message: Message) -> String {
        debug_assert_eq!(message.check(self.catalog), Ok(()));
        message.render(self.catalog)
    }

    /// A result: a file that is valid, measured or drawn.
    fn said(self, kind: &str, file: &str, message: Message) {
        if self.json {
            emit(&self.object(kind, file, &message, ""));
        } else {
            emit(&self.say(message));
        }
    }

    /// A file that could not be read, written or checked against the graph.
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

    /// An object of the JSON stream other than a refusal: its kind, its file, its message
    /// as key and arguments, its phrase, then what the kind adds (`extra`,
    /// members preceded by a comma).
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

/// A file read: the name to report it under, and its bytes.
struct Input {
    name: String,
    bytes: Vec<u8>,
}

/// Reads a file, or standard input for `-`. A failure is reported, and sets
/// the exit code to 2.
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

/// Validates the graph, then each placement and its check against the graph.
/// Every file is read, even after a refusal.
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

/// Reads back a graph or a placement and writes it in its canonical form, in
/// version 2. The two are told apart by their `format` key; failing one
/// that says placement, the file is read as a graph, whose
/// refusals will say what is missing.
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

/// The canonical form inside its object of the JSON stream: the only one on several
/// lines, the document as is under `document`, indented one step.
fn canonical(file: &str, text: &str) -> String {
    let document = text.trim_end_matches('\n').replace('\n', "\n  ");
    format!(
        "{{\n  \"kind\": \"canonical\",\n  \"file\": {},\n  \"document\": {document}\n}}\n",
        quote(file)
    )
}

/// The text is a JSON object whose `format` key names the placement.
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

/// Measures each placement for the graph. A placement that fails the
/// check against the graph is refused, not measured.
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

/// Draws a placement for its graph, as SVG or PNG depending on the extension
/// of the output file. A refused placement is not drawn.
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

/// A placement read and checked against the graph, with the name to report it under.
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

/// A line naming a placement: its file, its graph, its engine.
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

/// A line on standard output. A reader that closes the pipe before the
/// end (`foregram metrics … | head`) is not a fault of the call: the
/// lost output is ignored, the exit code still reports validity.
fn emit(line: &str) {
    write_out(&format!("{line}\n"));
}

fn write_out(text: &str) {
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
}

/// A line on standard error, under the same conditions.
fn emit_err(line: &str) {
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}
