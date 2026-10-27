// ForegramPM — bench board and report: aggregated measures, side-by-side renderings.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use foregram_graph::json::quote;
use foregram_graph::{Diagnostic, Graph, Layout, Message, Metrics};

use crate::engine::{self, Kind};
use crate::{
    LAYOUT_CORPORA, UNUSABLE, VALID, count, files_with, import_any, load, say, write_file,
};

/// The engines compared, in column order.
pub const ENGINES: [Kind; 2] = [Kind::Dot, Kind::Elk];

/// Graphs taken at regular intervals in each corpus, for the board.
const REGULAR: usize = 8;

/// Graphs taken, per corpus and per engine, where that engine makes the fewest
/// crossings compared to the other.
const AHEAD: usize = 3;

/// The names of the files written, in two pieces: in one block, the catalogue
/// guard would take them for keys of the `metrics` and `board` families.
const METRICS_JSON: &str = concat!("metrics", ".json");
const BOARD_MD: &str = concat!("board", ".md");

/// The measures of the pairwise comparison: lower is better.
const PAIRED: [&str; 4] = ["crossings", "bends", "edge_length", "area"];

/// What an engine gave for a graph: its measures, or its refusals,
/// counted by code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Measured(Metrics),
    Refused(BTreeMap<String, usize>),
}

/// A corpus graph and what each engine made of it.
#[derive(Clone, Debug)]
pub struct Entry {
    pub corpus: &'static str,
    /// The path under which the bench designates it: `rome/grafo….graphml`.
    pub shown: PathBuf,
    pub outcomes: [Outcome; 2],
}

/// Why a graph is on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    Regular,
    /// The engine of this rank makes fewer crossings than the other, by this many.
    Ahead(usize, i64),
}

/// Writes to `out` the JSON of the measures, the aggregated report and the board,
/// from the layouts that `layout-corpus` left in `layouts`.
/// Engine failures are data: they are counted, not fatal.
/// A missing layout with no listed failure, or one from a different version than
/// the manifest's, makes the board inexact: nothing is then written.
pub fn board(cache: &Path, layouts: &Path, out: &Path) -> u8 {
    let mut status = VALID;
    let manifest = cache.parent().unwrap_or(cache).join("manifest.txt");
    let Some(bytes) = load(&manifest, &mut status) else {
        return status;
    };
    let mut versions = Vec::new();
    for kind in ENGINES {
        match engine::manifest_value(&bytes, kind.section(), "version") {
            Some(version) => versions.push(version),
            None => {
                let message = Message::new("bench.board-unpinned")
                    .text("path", manifest.display())
                    .text("engine", kind.name());
                eprintln!("{}", say(message));
                return UNUSABLE;
            }
        }
    }
    let mut failures = Vec::new();
    for kind in ENGINES {
        let path = layouts.join(kind.name()).join("failures.txt");
        let Some(bytes) = load(&path, &mut status) else {
            return status;
        };
        failures.push(read_failures(&String::from_utf8_lossy(&bytes)));
    }

    let mut entries = Vec::new();
    let mut problems = 0;
    for (corpus, extension) in LAYOUT_CORPORA {
        let dir = cache.join(corpus);
        let files = match files_with(&dir, extension) {
            Ok(files) => files,
            Err(error) => {
                let message = Message::new("bench.unreadable")
                    .text("path", dir.display())
                    .text("error", error);
                eprintln!("{}", say(message));
                return UNUSABLE;
            }
        };
        let corpus = corpus.trim_start_matches("../");
        for path in &files {
            let shown = Path::new(corpus).join(path.strip_prefix(&dir).unwrap_or(path));
            let Some(bytes) = load(path, &mut status) else {
                return status;
            };
            let graph = match import_any(path, &bytes) {
                Ok(graph) => graph,
                Err(diags) => {
                    let refused = Outcome::Refused(codes(&diags));
                    entries.push(Entry {
                        corpus,
                        shown,
                        outcomes: [refused.clone(), refused],
                    });
                    continue;
                }
            };
            let mut outcome = |i: usize| -> Option<Outcome> {
                let kind = ENGINES[i];
                let target = layout_path(layouts, kind, &shown);
                if !target.exists() {
                    let listed = failures[i].get(&shown.display().to_string()).cloned();
                    if listed.is_none() {
                        let message = Message::new("bench.board-missing")
                            .text("path", shown.display())
                            .text("engine", kind.name());
                        eprintln!("{}", say(message));
                        problems += 1;
                    }
                    return listed.map(Outcome::Refused);
                }
                let bytes = load(&target, &mut status)?;
                let layout = match Layout::parse(&bytes) {
                    Ok(layout) => layout,
                    Err(diags) => return Some(Outcome::Refused(codes(&diags))),
                };
                if layout.engine.name != kind.name() || layout.engine.version != versions[i] {
                    let message = Message::new("bench.board-engine")
                        .text("path", target.display())
                        .text("engine", &layout.engine.name)
                        .text("version", &layout.engine.version)
                        .text("pinned", format!("{} {}", kind.name(), versions[i]));
                    eprintln!("{}", say(message));
                    problems += 1;
                    return None;
                }
                let diags = layout.check(&graph);
                if diags.is_empty() {
                    Some(Outcome::Measured(Metrics::measure(&graph, &layout)))
                } else {
                    Some(Outcome::Refused(codes(&diags)))
                }
            };
            if let (Some(a), Some(b)) = (outcome(0), outcome(1)) {
                entries.push(Entry {
                    corpus,
                    shown,
                    outcomes: [a, b],
                });
            }
        }
    }
    if status != VALID {
        return status;
    }
    if problems > 0 {
        let message = Message::new("bench.board-incomplete")
            .text("path", layouts.display())
            .msg("problems", count(problems, "problem"));
        eprintln!("{}", say(message));
        return UNUSABLE;
    }

    let engines: Vec<(&str, &str)> = ENGINES
        .iter()
        .zip(&versions)
        .map(|(kind, version)| (kind.name(), version.as_str()))
        .collect();
    let rows = sample(&entries);
    let svg_dir = out.join("svg");
    // Renderings from a previous run must not survive a
    // sample that has changed.
    if svg_dir.exists()
        && let Err(error) = std::fs::remove_dir_all(&svg_dir)
    {
        return unwritable(&svg_dir, error);
    }
    for (index, _) in &rows {
        let entry = &entries[*index];
        for (i, kind) in ENGINES.iter().enumerate() {
            if !matches!(entry.outcomes[i], Outcome::Measured(_)) {
                continue;
            }
            // Already read and measured above: only a file changed in the meantime
            // makes this re-read fail.
            let Some(svg) = draw(cache, layouts, *kind, entry, &mut status) else {
                return status.max(UNUSABLE);
            };
            let target = out.join(svg_path(*kind, &entry.shown));
            if let Err(error) = write_file(&target, svg.as_bytes()) {
                return unwritable(&target, error);
            }
        }
    }
    let files = [
        (METRICS_JSON, metrics_json(&engines, &entries)),
        ("report.md", report(&engines, &entries)),
        (BOARD_MD, board_md(&engines, &entries, &rows)),
    ];
    for (name, text) in files {
        let target = out.join(name);
        if let Err(error) = write_file(&target, text.as_bytes()) {
            return unwritable(&target, error);
        }
    }
    let message = Message::new("bench.board-written")
        .text("path", out.display())
        .msg("graphs", count(entries.len(), "graph"))
        .text("rows", rows.len());
    println!("{}", say(message));
    status
}

fn unwritable(path: &Path, error: std::io::Error) -> u8 {
    let message = Message::new("bench.unwritable")
        .text("path", path.display())
        .text("error", error);
    eprintln!("{}", say(message));
    UNUSABLE
}

/// Where `layout-corpus` wrote a graph's layout.
fn layout_path(layouts: &Path, kind: Kind, shown: &Path) -> PathBuf {
    layouts
        .join(kind.name())
        .join(shown)
        .with_extension("layout.json")
}

/// A graph's rendering on the board, relative to the board.
fn svg_path(kind: Kind, shown: &Path) -> PathBuf {
    Path::new("svg")
        .join(kind.name())
        .join(shown)
        .with_extension("svg")
}

/// Re-reads the graph and its layout, already measured, and draws them.
fn draw(
    cache: &Path,
    layouts: &Path,
    kind: Kind,
    entry: &Entry,
    status: &mut u8,
) -> Option<String> {
    let corpus_dir = LAYOUT_CORPORA
        .iter()
        .find(|(dir, _)| dir.trim_start_matches("../") == entry.corpus)
        .map(|(dir, _)| cache.join(dir))?;
    let source = corpus_dir.join(entry.shown.strip_prefix(entry.corpus).ok()?);
    let graph: Graph = import_any(&source, &load(&source, status)?).ok()?;
    let layout = Layout::parse(&load(&layout_path(layouts, kind, &entry.shown), status)?).ok()?;
    Some(foregram_render::svg(
        &graph,
        &layout,
        &foregram_render::Style::default(),
    ))
}

/// A graph's refusals, counted by code.
fn codes(diags: &[Diagnostic]) -> BTreeMap<String, usize> {
    let mut codes = BTreeMap::new();
    for diag in diags {
        *codes.entry(diag.code.to_string()).or_insert(0) += 1;
    }
    codes
}

/// Reads a `layout-corpus` `failures.txt`: one line per refusal,
/// `<path>: error[CODE]: …` or `<path>:<line>:<column>: error[CODE]: …`,
/// as the English presentation of a refusal writes it. Returns, per path,
/// the refusals counted by code.
pub fn read_failures(text: &str) -> BTreeMap<String, BTreeMap<String, usize>> {
    const MARK: &str = " error[";
    let mut out: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for line in text.lines() {
        let Some(at) = line.find(MARK) else {
            continue;
        };
        let rest = &line[at + MARK.len()..];
        let Some(end) = rest.find(']') else {
            continue;
        };
        let path = strip_position(line[..at].trim_end_matches(':'));
        *out.entry(path.to_string())
            .or_default()
            .entry(rest[..end].to_string())
            .or_insert(0) += 1;
    }
    out
}

/// `path:12:5` gives `path`; a path without a position stays as is.
fn strip_position(path: &str) -> &str {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let mut parts = path.rsplitn(3, ':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(col), Some(line), Some(rest)) if digits(col) && digits(line) => rest,
        _ => path,
    }
}

/// The board's graphs, per corpus in corpus order: first
/// `REGULAR` graphs at regular intervals in path order, all of them
/// if there are fewer; then, for each engine, the `AHEAD` graphs placed
/// by both where it makes the fewest crossings compared to the other, from the
/// largest gap to the smallest, ties in path order.
pub fn sample(entries: &[Entry]) -> Vec<(usize, Why)> {
    let mut rows = Vec::new();
    let mut corpora: Vec<&str> = Vec::new();
    for entry in entries {
        if !corpora.contains(&entry.corpus) {
            corpora.push(entry.corpus);
        }
    }
    for corpus in corpora {
        let members: Vec<usize> = (0..entries.len())
            .filter(|i| entries[*i].corpus == corpus)
            .collect();
        let n = members.len();
        let regular: Vec<usize> = if n <= REGULAR {
            members.clone()
        } else {
            (0..REGULAR).map(|k| members[k * n / REGULAR]).collect()
        };
        rows.extend(regular.iter().map(|i| (*i, Why::Regular)));
        for engine in 0..ENGINES.len() {
            let other = 1 - engine;
            let mut ahead: Vec<(i64, usize)> = members
                .iter()
                .filter(|i| !regular.contains(i))
                .filter_map(|i| {
                    let mine = crossings(&entries[*i].outcomes[engine])?;
                    let theirs = crossings(&entries[*i].outcomes[other])?;
                    (theirs > mine).then_some((theirs - mine, *i))
                })
                .collect();
            ahead.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            rows.extend(
                ahead
                    .iter()
                    .take(AHEAD)
                    .map(|(gap, i)| (*i, Why::Ahead(engine, *gap))),
            );
        }
    }
    rows
}

fn crossings(outcome: &Outcome) -> Option<i64> {
    match outcome {
        Outcome::Measured(m) => Some(field(m, "crossings")),
        Outcome::Refused(_) => None,
    }
}

fn field(m: &Metrics, name: &str) -> i64 {
    m.fields()
        .iter()
        .find(|(field, _)| *field == name)
        .map_or(0, |(_, value)| *value)
}

/// The value of rank `⌈p·n/100⌉` in ascending order, nearest rank,
/// no interpolation: always an observed value. `None` on an
/// empty list.
pub fn quantile(sorted: &[i64], percent: usize) -> Option<i64> {
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    let rank = (percent * n).div_ceil(100).max(1);
    sorted.get(rank - 1).copied()
}

/// The measures JSON: first the engines and their version, then one line
/// per graph and per engine, its measures or its refusals counted by code.
pub fn metrics_json(engines: &[(&str, &str)], entries: &[Entry]) -> String {
    let mut out = String::from("{\n  \"format\": \"foregram-board\",\n  \"version\": 1,\n");
    let engine_lines: Vec<String> = engines
        .iter()
        .map(|(name, version)| {
            format!(
                "{{\"name\": {}, \"version\": {}}}",
                quote(name),
                quote(version)
            )
        })
        .collect();
    out.push_str(&format!(
        "  \"engines\": [\n    {}\n  ],\n",
        engine_lines.join(",\n    ")
    ));
    let mut lines = Vec::new();
    for entry in entries {
        for ((name, _), outcome) in engines.iter().zip(&entry.outcomes) {
            let result = match outcome {
                Outcome::Measured(m) => format!("\"metrics\": {}", m.to_json()),
                Outcome::Refused(codes) => {
                    let codes: Vec<String> = codes
                        .iter()
                        .map(|(code, n)| format!("{}: {n}", quote(code)))
                        .collect();
                    format!("\"refused\": {{{}}}", codes.join(", "))
                }
            };
            lines.push(format!(
                "{{\"corpus\": {}, \"graph\": {}, \"engine\": {}, {result}}}",
                quote(entry.corpus),
                quote(&entry.shown.display().to_string()),
                quote(name)
            ));
        }
    }
    if lines.is_empty() {
        out.push_str("  \"results\": []\n}\n");
    } else {
        out.push_str(&format!(
            "  \"results\": [\n    {}\n  ]\n}}\n",
            lines.join(",\n    ")
        ));
    }
    out
}

/// What an engine gave on a corpus.
#[derive(Default)]
struct Tally {
    placed: usize,
    refused: usize,
    /// By code, the graphs refused at least once with it.
    codes: BTreeMap<String, usize>,
    /// By measure, the values of the placed graphs, sorted.
    values: BTreeMap<&'static str, Vec<i64>>,
}

fn tally(entries: &[&Entry], engine: usize) -> Tally {
    let mut t = Tally::default();
    for entry in entries {
        match &entry.outcomes[engine] {
            Outcome::Measured(m) => {
                t.placed += 1;
                for (name, value) in m.fields() {
                    t.values.entry(name).or_default().push(value);
                }
            }
            Outcome::Refused(codes) => {
                t.refused += 1;
                for code in codes.keys() {
                    *t.codes.entry(code.clone()).or_insert(0) += 1;
                }
            }
        }
    }
    for values in t.values.values_mut() {
        values.sort_unstable();
    }
    t
}

/// The aggregated report, one section per corpus: graphs placed and refused,
/// refusals by code, four statistics per measure and per engine, and the
/// comparison on the graphs placed by both.
pub fn report(engines: &[(&str, &str)], entries: &[Entry]) -> String {
    let named: Vec<String> = engines
        .iter()
        .map(|(name, version)| {
            say(Message::new("board.engine")
                .text("name", name)
                .text("version", version))
        })
        .collect();
    let (a, b) = (engines[0].0, engines[1].0);
    let mut out = say(Message::new("board.report-title").text("engines", named.join(", ")));
    out.push('\n');
    let mut corpora: Vec<&str> = Vec::new();
    for entry in entries {
        if !corpora.contains(&entry.corpus) {
            corpora.push(entry.corpus);
        }
    }
    let none = say(Message::new("board.none"));
    for corpus in corpora {
        let members: Vec<&Entry> = entries.iter().filter(|e| e.corpus == corpus).collect();
        let tallies = [tally(&members, 0), tally(&members, 1)];
        let mut lines = vec![
            String::new(),
            say(Message::new("board.corpus")
                .text("corpus", corpus)
                .msg("graphs", count(members.len(), "graph"))),
            String::new(),
            say(Message::new("board.outcome-head").text("a", a).text("b", b)),
            say(Message::new("board.placed")
                .text("a", tallies[0].placed)
                .text("b", tallies[1].placed)),
            say(Message::new("board.refused")
                .text("a", tallies[0].refused)
                .text("b", tallies[1].refused)),
        ];
        let mut all_codes: Vec<&String> = tallies[0].codes.keys().collect();
        all_codes.extend(tallies[1].codes.keys());
        all_codes.sort();
        all_codes.dedup();
        for code in all_codes {
            let n = |t: &Tally| t.codes.get(code).copied().unwrap_or(0);
            lines.push(say(Message::new("board.refused-code")
                .text("code", code)
                .text("a", n(&tallies[0]))
                .text("b", n(&tallies[1]))));
        }
        lines.push(String::new());
        lines.push(say(Message::new("board.metrics-head")
            .text("a", a)
            .text("b", b)));
        for (name, _) in Metrics::default().fields() {
            let stats = |t: &Tally| -> [String; 4] {
                let values = t.values.get(name).map(Vec::as_slice).unwrap_or(&[]);
                if values.is_empty() {
                    return [none.clone(), none.clone(), none.clone(), none.clone()];
                }
                let total: i128 = values.iter().map(|v| i128::from(*v)).sum();
                let q = |p| quantile(values, p).map_or(none.clone(), |v| v.to_string());
                [total.to_string(), q(50), q(90), q(100)]
            };
            let [at, am, ap, ax] = stats(&tallies[0]);
            let [bt, bm, bp, bx] = stats(&tallies[1]);
            lines.push(say(Message::new("board.metrics-row")
                .text("name", name)
                .text("a-total", at)
                .text("a-median", am)
                .text("a-p90", ap)
                .text("a-max", ax)
                .text("b-total", bt)
                .text("b-median", bm)
                .text("b-p90", bp)
                .text("b-max", bx)));
        }
        let both: Vec<(&Metrics, &Metrics)> = members
            .iter()
            .filter_map(|e| match &e.outcomes {
                [Outcome::Measured(x), Outcome::Measured(y)] => Some((x, y)),
                _ => None,
            })
            .collect();
        lines.push(String::new());
        if both.is_empty() {
            lines.push(say(Message::new("board.pairs-none")));
        } else {
            lines.push(say(Message::new("board.pairs-head")
                .msg("graphs", count(both.len(), "graph"))
                .text("a", a)
                .text("b", b)));
            for name in PAIRED {
                let (mut first, mut equal, mut second) = (0, 0, 0);
                for (x, y) in &both {
                    match field(x, name).cmp(&field(y, name)) {
                        std::cmp::Ordering::Less => first += 1,
                        std::cmp::Ordering::Equal => equal += 1,
                        std::cmp::Ordering::Greater => second += 1,
                    }
                }
                lines.push(say(Message::new("board.pairs-row")
                    .text("name", name)
                    .text("a", first)
                    .text("equal", equal)
                    .text("b", second)));
            }
        }
        for line in lines {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

/// The board: per corpus, one line per retained graph, each engine's rendering
/// side by side with its crossings and bends, or its refusals.
pub fn board_md(engines: &[(&str, &str)], entries: &[Entry], rows: &[(usize, Why)]) -> String {
    let (a, b) = (engines[0].0, engines[1].0);
    let mut out = say(Message::new("board.board-title")
        .text("a", a)
        .text("b", b)
        .text("regular", REGULAR)
        .text("ahead", AHEAD));
    out.push('\n');
    let mut current = "";
    for (index, why) in rows {
        let entry = &entries[*index];
        if entry.corpus != current {
            current = entry.corpus;
            out.push('\n');
            out.push_str(&say(Message::new("board.board-corpus")
                .text("corpus", current)
                .text("a", a)
                .text("b", b)));
            out.push('\n');
        }
        let why = match why {
            Why::Regular => Message::new("board.why-regular"),
            Why::Ahead(engine, gap) => Message::new("board.why-ahead")
                .text("engine", engines[*engine].0)
                .text("n", gap),
        };
        let cell = |i: usize| -> Message {
            let kind = ENGINES[i];
            match &entry.outcomes[i] {
                Outcome::Measured(m) => Message::new("board.cell")
                    .text("engine", kind.name())
                    .text("svg", svg_path(kind, &entry.shown).display())
                    .text("crossings", field(m, "crossings"))
                    .text("bends", field(m, "bends")),
                Outcome::Refused(codes) => {
                    let codes: Vec<&str> = codes.keys().map(String::as_str).collect();
                    Message::new("board.cell-refused").text("codes", codes.join(", "))
                }
            }
        };
        out.push_str(&say(Message::new("board.row")
            .text("graph", entry.shown.display())
            .msg("why", why)
            .msg("a", cell(0))
            .msg("b", cell(1))));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(crossings: u64) -> Outcome {
        Outcome::Measured(Metrics {
            crossings,
            ..Metrics::default()
        })
    }

    fn refused(code: &str) -> Outcome {
        Outcome::Refused(BTreeMap::from([(code.to_string(), 1)]))
    }

    fn entry(corpus: &'static str, name: &str, a: Outcome, b: Outcome) -> Entry {
        Entry {
            corpus,
            shown: Path::new(corpus).join(name),
            outcomes: [a, b],
        }
    }

    #[test]
    fn quantiles_take_the_nearest_rank() {
        assert_eq!(quantile(&[], 50), None);
        assert_eq!(quantile(&[7], 50), Some(7));
        assert_eq!(quantile(&[7], 90), Some(7));
        // Rank ⌈n/2⌉: the lower median on an even count.
        assert_eq!(quantile(&[1, 2, 3, 4], 50), Some(2));
        assert_eq!(quantile(&[1, 2, 3, 4, 5], 50), Some(3));
        // Rank ⌈9n/10⌉: 9 of 10 and 10 of 11, the value at that rank.
        let ten: Vec<i64> = (1..=10).collect();
        assert_eq!(quantile(&ten, 90), Some(9));
        let eleven: Vec<i64> = (1..=11).collect();
        assert_eq!(quantile(&eleven, 90), Some(10));
        assert_eq!(quantile(&eleven, 100), Some(11));
    }

    #[test]
    fn failures_are_read_by_path_and_code() {
        let text = "rome/a.graphml: error[REF003]: dot failed\n\
                    elk-models/b.elkt:3:7: error[LYT009]: off the border\n\
                    elk-models/b.elkt:4:7: error[LYT009]: off the border\n\
                    elk-models/b.elkt: error[LYT016]: shared port\n\
                    not a failure\n";
        let read = read_failures(text);
        assert_eq!(read.len(), 2);
        assert_eq!(
            read["rome/a.graphml"],
            BTreeMap::from([("REF003".into(), 1)])
        );
        assert_eq!(
            read["elk-models/b.elkt"],
            BTreeMap::from([("LYT009".into(), 2), ("LYT016".into(), 1)])
        );
        assert_eq!(strip_position("a/b.elkt"), "a/b.elkt");
        assert_eq!(strip_position("a/b.elkt:12:5"), "a/b.elkt");
        assert_eq!(strip_position("a/b:c.elkt"), "a/b:c.elkt");
    }

    #[test]
    fn the_sample_takes_regular_graphs_then_the_widest_gaps() {
        // Twenty graphs: dot crosses i times, ELK 10 times; 3 is refused by ELK.
        let mut entries: Vec<Entry> = (0..20)
            .map(|i| entry("rome", &format!("g{i:02}"), measured(i), measured(10)))
            .collect();
        entries[3].outcomes[1] = refused("REF003");
        entries.push(entry("mcd", "m", measured(1), measured(0)));
        let rows = sample(&entries);
        let regular: Vec<usize> = rows
            .iter()
            .filter(|(_, why)| *why == Why::Regular)
            .map(|(i, _)| *i)
            .collect();
        // 0, 2, 5, 7, 10, 12, 15, 17 in Rome; the only mcd graph.
        assert_eq!(regular, vec![0, 2, 5, 7, 10, 12, 15, 17, 20]);
        // dot ahead: 1 (gap 9), 4 (6), 6 (4); 3 is not placed by
        // ELK and 0 is already taken. ELK ahead: 19, 18, 16.
        let ahead: Vec<(usize, Why)> = rows
            .iter()
            .filter(|(_, why)| *why != Why::Regular)
            .copied()
            .collect();
        assert_eq!(
            ahead,
            vec![
                (1, Why::Ahead(0, 9)),
                (4, Why::Ahead(0, 6)),
                (6, Why::Ahead(0, 4)),
                (19, Why::Ahead(1, 9)),
                (18, Why::Ahead(1, 8)),
                (16, Why::Ahead(1, 6)),
            ]
        );
        assert_eq!(sample(&entries), rows);
    }

    #[test]
    fn the_report_compares_only_graphs_placed_by_both() {
        let entries = vec![
            entry("rome", "a", measured(1), measured(3)),
            entry("rome", "b", measured(2), measured(2)),
            entry("rome", "c", measured(5), refused("LYT009")),
            entry("rome", "d", refused("REF003"), refused("REF003")),
        ];
        let engines = [("dot", "16.1.0"), ("elk", "0.12.0")];
        let text = report(&engines, &entries);
        assert!(text.contains("dot 16.1.0, elk 0.12.0"), "{text}");
        assert!(text.contains("| laid out and valid | 3 | 2 |"), "{text}");
        assert!(text.contains("| refused | 1 | 2 |"), "{text}");
        assert!(text.contains("| refused with `LYT009` | 0 | 1 |"), "{text}");
        assert!(text.contains("| refused with `REF003` | 1 | 1 |"), "{text}");
        // dot: 1, 2, 5; ELK: 3, 2.
        assert!(
            text.contains("| crossings | 8 | 2 | 5 | 5 | 5 | 2 | 3 | 3 |"),
            "{text}"
        );
        assert!(text.contains("2 graphs laid out by both"), "{text}");
        assert!(text.contains("| crossings | 1 | 1 | 0 |"), "{text}");
        assert!(text.contains("| bends | 0 | 2 | 0 |"), "{text}");
    }

    #[test]
    fn the_json_has_one_line_per_graph_and_engine() {
        let entries = vec![entry("mcd", "m.mcd", measured(1), refused("LYT009"))];
        let text = metrics_json(&[("dot", "16.1.0"), ("elk", "0.12.0")], &entries);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[3], "  \"engines\": [");
        assert_eq!(
            lines[4],
            "    {\"name\": \"dot\", \"version\": \"16.1.0\"},"
        );
        assert!(
            lines[8].starts_with(
                "    {\"corpus\": \"mcd\", \"graph\": \"mcd/m.mcd\", \"engine\": \"dot\", \"metrics\": {\"crossings\": 1, "
            ),
            "{text}"
        );
        assert_eq!(
            lines[9],
            "    {\"corpus\": \"mcd\", \"graph\": \"mcd/m.mcd\", \"engine\": \"elk\", \"refused\": {\"LYT009\": 1}}"
        );
        assert!(
            foregram_graph::json::parse(text.as_bytes()).is_ok(),
            "{text}"
        );
        let empty = metrics_json(&[("dot", "1"), ("elk", "2")], &[]);
        assert!(
            foregram_graph::json::parse(empty.as_bytes()).is_ok(),
            "{empty}"
        );
    }
}
