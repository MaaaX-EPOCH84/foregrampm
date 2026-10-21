// ForegramPM — bench: reference engines, ELK and Graphviz, at a pinned
// version, that return their coordinates in the exchange format.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod dot;
pub mod elk;
pub mod fit;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use foregram_graph::{Diagnostic, Graph, Layout, Message, Pos, Rect};

/// Time allowed for an engine on one graph. ELK's driver applies the same.
pub const TIMEOUT: Duration = Duration::from_secs(60);

/// A reference engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `dot`, from Graphviz.
    Dot,
    /// ELK Layered, through the bench's Java driver.
    Elk,
}

impl Kind {
    pub fn parse(word: &str) -> Option<Kind> {
        match word {
            "dot" => Some(Kind::Dot),
            "elk" => Some(Kind::Elk),
            _ => None,
        }
    }

    /// The engine's name, as the layout carries it and the command takes it.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Dot => "dot",
            Kind::Elk => "elk",
        }
    }

    /// The manifest section that pins its version.
    pub fn section(self) -> &'static str {
        match self {
            Kind::Dot => "graphviz",
            Kind::Elk => "elk",
        }
    }
}

/// An engine found in the cache, at the version the manifest pins.
#[derive(Clone, Debug)]
pub struct Tools {
    pub kind: Kind,
    pub version: String,
    /// `dot`: the executable; ELK: the directory of jars and classes.
    pub path: PathBuf,
}

impl Tools {
    /// Looks for the engine in the `corpus/fetch.sh` cache and checks its
    /// version against the neighbouring manifest. Refuses if either is missing or differs.
    pub fn locate(kind: Kind, cache: &Path) -> Result<Tools, Diagnostic> {
        let manifest = cache.parent().unwrap_or(cache).join("manifest.txt");
        let bytes = std::fs::read(&manifest).unwrap_or_default();
        let pinned = manifest_value(&bytes, kind.section(), "version")
            .ok_or_else(|| missing(kind, &manifest))?;
        let (path, found) = match kind {
            Kind::Dot => {
                let path = dot::executable(cache);
                let found = dot::installed_version(&path).ok_or_else(|| missing(kind, &path))?;
                (path, found)
            }
            Kind::Elk => {
                let path = cache.join("elk");
                let found = elk::installed_version(&path).ok_or_else(|| missing(kind, &path))?;
                (path, found)
            }
        };
        if found != pinned {
            let message = Message::new("R002")
                .text("engine", kind.name())
                .text("found", &found)
                .text("pinned", &pinned);
            return Err(Diagnostic::new(Pos::NONE, message));
        }
        Ok(Tools {
            kind,
            version: pinned,
            path,
        })
    }
}

fn missing(kind: Kind, path: &Path) -> Diagnostic {
    let message = Message::new("R001")
        .text("engine", kind.name())
        .text("path", path.display());
    Diagnostic::new(Pos::NONE, message)
}

/// An engine ready to lay out graphs, one after another.
pub enum Runner {
    Dot(Tools),
    Elk(elk::Process),
}

/// What a pass over one graph leaves: the layout if it could be
/// built, the refusals — from the engine, then from the validator —, and the route
/// ends pulled back onto their border.
pub struct Outcome {
    pub layout: Option<Layout>,
    pub diags: Vec<Diagnostic>,
    pub fit: fit::Stats,
}

impl Runner {
    pub fn start(tools: Tools) -> Result<Runner, Diagnostic> {
        match tools.kind {
            Kind::Dot => Ok(Runner::Dot(tools)),
            Kind::Elk => elk::Process::start(tools).map(Runner::Elk),
        }
    }

    /// Lays out a graph. The returned layout is built even if it is refused:
    /// a failure reads better on the file than on its absence.
    pub fn layout(&mut self, graph: &Graph) -> Outcome {
        let built = match self {
            Runner::Dot(tools) => dot::layout(tools, graph),
            Runner::Elk(process) => process.layout(graph),
        };
        match built {
            Err(diag) => Outcome {
                layout: None,
                diags: vec![diag],
                fit: fit::Stats::default(),
            },
            Ok(mut layout) => {
                let fit = fit::fit(graph, &mut layout);
                let diags = layout.check(graph);
                Outcome {
                    layout: Some(layout),
                    diags,
                    fit,
                }
            }
        }
    }
}

/// Runs a command, gives it `input`, and returns its standard output. Refuses
/// if it fails (R003) or exceeds the time limit (R004).
pub fn run(
    kind: Kind,
    command: &mut Command,
    input: &[u8],
    timeout: Duration,
) -> Result<Vec<u8>, Diagnostic> {
    let failed = |detail: String| {
        let message = Message::new("R003")
            .text("engine", kind.name())
            .text("detail", detail);
        Diagnostic::new(Pos::NONE, message)
    };
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| failed(e.to_string()))?;
    let (Some(mut stdin), Some(mut stdout), Some(mut stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        return Err(failed(String::from("no pipe")));
    };
    // Input is written and outputs are read separately: a full pipe
    // would block the engine before it finished.
    let input = input.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let out_reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut stdout, &mut out).map(|_| out)
    });
    let err_reader = std::thread::spawn(move || {
        let mut err = Vec::new();
        std::io::Read::read_to_end(&mut stderr, &mut err).map(|_| err)
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let message = Message::new("R004")
                    .text("engine", kind.name())
                    .text("millis", timeout.as_millis());
                break Err(Diagnostic::new(Pos::NONE, message));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => break Err(failed(e.to_string())),
        }
    };
    let _ = writer.join();
    let out = out_reader
        .join()
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
    let err = err_reader
        .join()
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
    let status = status?;
    if !status.success() {
        let first = String::from_utf8_lossy(&err);
        let first = first.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        return Err(failed(format!("{status}: {first}")));
    }
    Ok(out)
}

/// In `[section]`, the value of `key = …`, from the first line that carries it.
pub fn manifest_value(bytes: &[u8], section: &str, key: &str) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let header = format!("[{section}]");
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with('[') {
            inside = line.trim() == header;
            continue;
        }
        if inside
            && let Some(rest) = line.strip_prefix(key)
            && let Some(value) = rest.trim_start().strip_prefix('=')
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

/// A number written in pixels — `12.5`, `-0.0`, `1.0E-4` —, carried in hundredths
/// of a pixel: `12.5` gives 1250. Beyond two decimals, rounded to nearest,
/// half away from zero. `None` if it does not fit in an `i64`.
pub fn hundredths(text: &str) -> Option<i64> {
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (mantissa, exponent) = match rest.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i64>().ok()?),
        None => (rest, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    let digits: Vec<u8> = whole
        .bytes()
        .chain(fraction.bytes())
        .map(|b| b.checked_sub(b'0').filter(|d| *d < 10))
        .collect::<Option<_>>()?;
    // Value = digits × 10^(exponent − decimals); in hundredths, two more.
    let shift = exponent
        .checked_sub(i64::try_from(fraction.len()).ok()?)?
        .checked_add(2)?;
    let kept = i64::try_from(digits.len()).ok()?.checked_add(shift)?;
    let kept = usize::try_from(kept.max(0)).ok()?;
    let mut value: i128 = 0;
    for &digit in digits.iter().take(kept) {
        value = value.checked_mul(10)?.checked_add(i128::from(digit))?;
    }
    for _ in digits.len()..kept {
        value = value.checked_mul(10)?;
    }
    if digits.get(kept).is_some_and(|&d| d >= 5) {
        value += 1;
    }
    i64::try_from(if negative { -value } else { value }).ok()
}

/// Hundredths of a pixel written in pixels, exactly: 4950 gives `49.50`.
pub fn px(hundredths: i64) -> String {
    let sign = if hundredths < 0 { "-" } else { "" };
    let abs = hundredths.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

/// A refusal for output that cannot be understood (R005).
pub fn garbled(kind: Kind, detail: impl ToString) -> Diagnostic {
    let message = Message::new("R005")
        .text("engine", kind.name())
        .text("detail", detail.to_string());
    Diagnostic::new(Pos::NONE, message)
}

/// A number read from an engine's output, in hundredths; refuses what does not
/// fit (R006).
pub fn number(kind: Kind, text: &str) -> Result<i64, Diagnostic> {
    hundredths(text).ok_or_else(|| {
        let message = Message::new("R006")
            .text("engine", kind.name())
            .text("text", text);
        Diagnostic::new(Pos::NONE, message)
    })
}

/// The absolute rectangles of a layout's nodes, by identifier: a leaf's
/// size comes from the graph, a container's from the layout.
/// A node the layout forgets, or whose parent has no rectangle,
/// is left out.
pub fn absolute_rects(graph: &Graph, layout: &Layout) -> BTreeMap<String, Rect> {
    let places: BTreeMap<&str, &foregram_graph::NodePlace> =
        layout.nodes.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut rects: BTreeMap<String, Rect> = BTreeMap::new();
    for (node, parent) in graph.all_nodes() {
        let Some(place) = places.get(node.id.as_str()) else {
            continue;
        };
        let origin = match parent {
            None => Some((0, 0)),
            Some(parent) => rects.get(&parent.id).map(|r| (r.x, r.y)),
        };
        let size = if node.is_container() {
            place.size
        } else {
            node.size
        };
        if let (Some((ox, oy)), Some(size)) = (origin, size) {
            rects.insert(
                node.id.clone(),
                Rect {
                    x: ox + place.x,
                    y: oy + place.y,
                    width: size.width,
                    height: size.height,
                },
            );
        }
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_carried_in_hundredths_and_rounded() {
        assert_eq!(hundredths("12.5"), Some(1250));
        assert_eq!(hundredths("-0.0"), Some(0));
        assert_eq!(hundredths("1.0E-4"), Some(0));
        assert_eq!(hundredths("0.005"), Some(1));
        assert_eq!(hundredths("-0.005"), Some(-1));
        assert_eq!(hundredths("18.872"), Some(1887));
        assert_eq!(hundredths("1e30"), None);
        assert_eq!(hundredths("abc"), None);
        assert_eq!(hundredths(""), None);
        assert_eq!(hundredths("."), None);
    }

    #[test]
    fn pixels_are_written_exactly() {
        assert_eq!(px(4950), "49.50");
        assert_eq!(px(1), "0.01");
        assert_eq!(px(-250), "-2.50");
        assert_eq!(px(0), "0.00");
    }

    #[test]
    fn the_manifest_gives_a_value_per_section() {
        let text = b"[graphviz]\nversion = 16.1.0\n\n[elk]\nversion = 0.12.0\nversion = 9\n";
        assert_eq!(
            manifest_value(text, "graphviz", "version").as_deref(),
            Some("16.1.0")
        );
        assert_eq!(
            manifest_value(text, "elk", "version").as_deref(),
            Some("0.12.0")
        );
        assert_eq!(manifest_value(text, "rome", "version"), None);
        assert_eq!(manifest_value(text, "elk", "count"), None);
    }

    #[test]
    fn a_failing_or_slow_engine_is_refused_with_its_code() {
        let failed = run(Kind::Dot, &mut Command::new("false"), b"", TIMEOUT).unwrap_err();
        assert_eq!(failed.code, "R003");
        let absent = run(
            Kind::Dot,
            &mut Command::new("/nonexistent/engine"),
            b"",
            TIMEOUT,
        );
        assert_eq!(absent.unwrap_err().code, "R003");
        let slow = Duration::from_millis(50);
        let slow = run(Kind::Dot, Command::new("sleep").arg("5"), b"", slow).unwrap_err();
        assert_eq!(slow.code, "R004");
        let out = run(Kind::Dot, &mut Command::new("cat"), b"echo", TIMEOUT).unwrap();
        assert_eq!(out, b"echo");
    }

    #[cfg(unix)]
    #[test]
    fn an_engine_at_another_version_than_the_manifest_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join("foregram-bench-pinned");
        let _ = std::fs::remove_dir_all(&root);
        let cache = root.join("cache");
        let bin = dot::executable(&cache);
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(root.join("manifest.txt"), "[graphviz]\nversion = 16.1.0\n").unwrap();
        let script = "#!/bin/sh\necho 'dot - graphviz version 2.43.0 (0)' >&2\n";
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let diag = Tools::locate(Kind::Dot, &cache).unwrap_err();
        assert_eq!(diag.code, "R002");
        assert_eq!(
            diag.text(),
            "dot: version 2.43.0 found, the manifest pins 16.1.0"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_engine_is_refused_before_anything_runs() {
        let cache = std::env::temp_dir().join("foregram-bench-no-cache");
        let diag = Tools::locate(Kind::Dot, &cache).unwrap_err();
        assert_eq!(diag.code, "R001");
        let diag = Tools::locate(Kind::Elk, &cache).unwrap_err();
        assert_eq!(diag.code, "R001");
    }
}
