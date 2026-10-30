// ForegramPM — guard of the silent fallbacks: each one says why it is not a fault.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! The product never panics, so a case believed impossible is handled by a
//! fallback: a default value, a saturating sum, a `let … else` that skips.
//! A fallback that hides a broken invariant is a fault that nobody sees;
//! one that honours a contract is fine. This guard reads the sources of the
//! product crates and requires every fallback to carry, within the lines
//! just above it, a comment `// fallback: <reason>`. A new fallback without
//! its reason fails here. What is a fault instead of a fallback becomes an
//! internal error, family `INT`.

// A failing test must stop dead: the product's rules do not apply here.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};

/// The marker a fallback carries, followed by its reason.
const MARKER: &str = "// fallback:";

/// Method names that take a fallback value or saturate instead of failing.
const CALLS: [&str; 5] = ["unwrap_or", "saturating_", "wrapping_", "checked_", ".ok()"];

/// What an `else` block does when it reports instead of skipping.
const REPORTS: [&str; 3] = ["Diagnostic::new", "refuse(", "Err("];

/// How far above a fallback its marker may stand, without a blank line between.
const REACH: usize = 10;

/// The unit tests that follow this marker are not product code.
const MARK: &str = concat!("#[cfg", "(test)]");

/// The `.rs` sources of a directory, subdirectories included, sorted.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{} : {e}", dir.display())) {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// The code of a line, its trailing comment cut off (strings are never
/// looked into: no product string holds `//`).
fn code(line: &str) -> &str {
    line.split("//").next().unwrap_or("")
}

/// Whether a marker with a reason stands within reach above line `at`.
fn marked(lines: &[&str], at: usize) -> bool {
    lines[..at]
        .iter()
        .rev()
        .take(REACH)
        .take_while(|line| !line.trim().is_empty())
        .any(|line| {
            line.split_once(MARKER)
                .is_some_and(|(_, reason)| !reason.trim().is_empty())
        })
}

/// The `let … else` statement starting at line `at`: the index of its
/// `else {` line and whether its block reports, or `None` if it is not one.
fn let_else(lines: &[&str], at: usize) -> Option<(usize, bool)> {
    let first = lines[at];
    if !first.trim_start().starts_with("let ") {
        return None;
    }
    let indent = first.len() - first.trim_start().len();
    let mut head = at;
    loop {
        let text = code(lines[head]).trim_end();
        if text.ends_with(';') {
            return None;
        }
        if text.ends_with('{') {
            if !text.ends_with("else {") {
                return None;
            }
            break;
        }
        head += 1;
        if head >= lines.len() {
            return None;
        }
    }
    let mut reports = false;
    for line in &lines[head + 1..] {
        let own = line.len() - line.trim_start().len();
        if own == indent && line.trim_start().starts_with('}') {
            break;
        }
        reports |= REPORTS.iter().any(|r| code(line).contains(r));
    }
    Some((head, reports))
}

/// The unmarked fallbacks of a source text: line number (from 1) and what.
fn offences(text: &str) -> Vec<(usize, String)> {
    let product = text.split(MARK).next().unwrap();
    let lines: Vec<&str> = product.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if is_comment(line) {
            continue;
        }
        for call in CALLS {
            if code(line).contains(call) && !marked(&lines, i) {
                out.push((i + 1, call.to_string()));
            }
        }
        if let Some((_, reports)) = let_else(&lines, i)
            && !reports
            && !marked(&lines, i)
        {
            out.push((i + 1, "let … else".to_string()));
        }
    }
    out
}

#[test]
fn every_fallback_of_the_product_says_why() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut seen = 0;
    let mut found = Vec::new();
    for krate in ["foregram-graph", "foregram-render", "foregram-source"] {
        let dir = root.join(krate).join("src");
        for path in sources(&dir) {
            seen += 1;
            let text = std::fs::read_to_string(&path).unwrap();
            for (line, what) in offences(&text) {
                found.push(format!(
                    "{krate}/src/{}:{line} : {what}",
                    path.strip_prefix(&dir).unwrap().display()
                ));
            }
        }
    }
    assert!(seen >= 16, "trop peu de sources lues : {seen}");
    assert!(
        found.is_empty(),
        "replis sans motif (`// fallback: …` attendu au-dessus) :\n{}",
        found.join("\n")
    );
}

#[test]
fn the_guard_catches_every_form_and_accepts_a_reason() {
    let flagged = |text: &str| {
        offences(text)
            .iter()
            .map(|(_, w)| w.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(flagged("let a = b.unwrap_or(0);"), ["unwrap_or"]);
    assert_eq!(flagged("let a = b.unwrap_or_default();"), ["unwrap_or"]);
    assert_eq!(flagged("n = n.saturating_add(1);"), ["saturating_"]);
    assert_eq!(flagged("n = n.wrapping_mul(3);"), ["wrapping_"]);
    assert_eq!(flagged("let n = n.checked_add(1)?;"), ["checked_"]);
    assert_eq!(flagged("let n = text.parse::<i64>().ok();"), [".ok()"]);
    assert_eq!(
        flagged("let Some(x) = y else {\n    continue;\n};"),
        ["let … else"]
    );
    assert_eq!(
        flagged("let Some(x) = y\n    .find(f) else {\n    return;\n};"),
        ["let … else"]
    );
    // An `else` that reports is not silent.
    assert!(
        flagged("let Some(x) = y else {\n    d.push(Diagnostic::new(p, m));\n    continue;\n};")
            .is_empty()
    );
    assert!(
        flagged("let Some(x) = y else {\n    self.refuse(p, m);\n    return None;\n};").is_empty()
    );
    assert!(flagged("let Some(x) = y else {\n    return Err(e);\n};").is_empty());
    // A plain `let` and an `if … else` are not fallbacks.
    assert!(flagged("let x = y;\nif a {\n    b\n} else {\n    c\n}").is_empty());
    // A reason just above covers, even a few lines up; a blank line cuts it.
    assert!(flagged("// fallback: no value means zero\nlet a = b.unwrap_or(0);").is_empty());
    assert!(
        flagged("// fallback: the chain\nlet a = b\n    .iter()\n    .max()\n    .unwrap_or(0);")
            .is_empty()
    );
    assert_eq!(
        flagged("// fallback: too far\n\nlet a = b.unwrap_or(0);"),
        ["unwrap_or"]
    );
    assert_eq!(
        flagged("// fallback:\nlet a = b.unwrap_or(0);"),
        ["unwrap_or"]
    );
    // Comments and unit tests are not read.
    assert!(flagged("// b.unwrap_or(0)").is_empty());
    assert!(flagged(&format!("{MARK}\nlet a = b.unwrap_or(0);")).is_empty());
}
