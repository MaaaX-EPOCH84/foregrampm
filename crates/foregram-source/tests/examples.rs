// ForegramPM — tests of the example files: one accepted file per construct, one refused file per code.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

// A failing test must stop short: the product rules do not apply here.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use foregram_graph::Catalog;
use foregram_source::Document;

/// One accepted file per construct of the grammar, in canonical form but
/// `semicolon.fg`, which shows what the canonical form unfolds.
const ACCEPTED: [&str; 11] = [
    "empty.fg",
    "header.fg",
    "element.fg",
    "label.fg",
    "content.fg",
    "attribute.fg",
    "semicolon.fg",
    "container.fg",
    "string.fg",
    "comment.fg",
    "empty-block.fg",
];

/// The one file not in canonical form.
const UNFOLDED: &str = "semicolon.fg";

/// One refused file per code the reader produces, with the one refusal it
/// gets: key, line, column. `SRC015` comes from the lookup, which no notation
/// calls before E5: it has no file.
const REFUSED: [(&str, &str, u32, u32); 14] = [
    ("refuse-SRC001.fg", "SRC001", 2, 5),
    ("refuse-SRC002.fg", "SRC002", 2, 4),
    ("refuse-SRC003.fg", "SRC003", 2, 3),
    ("refuse-SRC004.fg", "SRC004", 2, 4),
    ("refuse-SRC005.fg", "SRC005", 2, 4),
    ("refuse-SRC006.fg", "SRC006", 2, 1),
    ("refuse-SRC007.fg", "SRC007", 1, 10),
    ("refuse-SRC008.fg", "SRC008.top", 4, 1),
    ("refuse-SRC009.fg", "SRC009", 1, 8),
    ("refuse-SRC010.fg", "SRC010", 3, 1),
    ("refuse-SRC011.fg", "SRC011", 1, 1),
    ("refuse-SRC012.fg", "SRC012", 3, 8),
    ("refuse-SRC013.fg", "SRC013", 2, 1),
    ("refuse-SRC014.fg", "SRC014", 33, 75),
];

/// The refused file of the section on `check`, where refusals add up.
const CHECK: &str = "check.fg";

/// The files the documentation shows escaped, their bytes not being text.
const ESCAPED: [&str; 2] = ["refuse-SRC001.fg", "refuse-SRC002.fg"];

fn data_dir() -> String {
    format!("{}/tests/data", env!("CARGO_MANIFEST_DIR"))
}

fn data(name: &str) -> Vec<u8> {
    let path = format!("{}/{name}", data_dir());
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn text(name: &str) -> String {
    String::from_utf8(data(name)).unwrap()
}

fn doc() -> String {
    let path = format!("{}/../../docs/source-format.md", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The bytes as the documentation shows them: control characters other
/// than tab and line feed, and bytes that are not UTF-8, as `\xHH`.
fn escaped(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.utf8_chunks() {
        for c in chunk.valid().chars() {
            if c.is_control() && c != '\n' && c != '\t' {
                let mut buf = [0; 4];
                for b in c.encode_utf8(&mut buf).bytes() {
                    out.push_str(&format!("\\x{b:02x}"));
                }
            } else {
                out.push(c);
            }
        }
        for b in chunk.invalid() {
            out.push_str(&format!("\\x{b:02x}"));
        }
    }
    out
}

/// The section of the documentation that shows the example files.
fn examples_section() -> String {
    let doc = doc();
    doc.split("\n## Example files\n")
        .nth(1)
        .unwrap()
        .to_string()
}

#[test]
fn the_directory_holds_the_listed_files_and_no_other() {
    let listed: BTreeSet<String> = ACCEPTED
        .iter()
        .copied()
        .chain(REFUSED.iter().map(|r| r.0))
        .chain([CHECK])
        .map(String::from)
        .collect();
    assert_eq!(listed.len(), ACCEPTED.len() + REFUSED.len() + 1);
    let found: BTreeSet<String> = std::fs::read_dir(data_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(found, listed);
}

#[test]
fn every_accepted_file_is_read_and_in_canonical_form() {
    for name in ACCEPTED {
        let bytes = data(name);
        let document = Document::parse(&bytes).unwrap_or_else(|diags| {
            let lines: Vec<String> = diags.iter().map(ToString::to_string).collect();
            panic!("{name} refused:\n{}", lines.join("\n"))
        });
        let canonical = document.to_text();
        if name == UNFOLDED {
            assert_ne!(canonical.as_bytes(), bytes, "{name}");
            let again = Document::parse(canonical.as_bytes()).unwrap().to_text();
            assert_eq!(again, canonical, "{name}");
        } else {
            assert_eq!(canonical.as_bytes(), bytes, "{name}");
        }
    }
}

#[test]
fn every_refused_file_gets_its_code_alone() {
    for (name, key, line, col) in REFUSED {
        let Err(diags) = Document::parse(&data(name)) else {
            panic!("{name} accepted")
        };
        let found: Vec<(&str, u32, u32)> = diags
            .iter()
            .map(|d| (d.message.key, d.pos.line, d.pos.col))
            .collect();
        assert_eq!(found, [(key, line, col)], "{name}");
        assert!(name.contains(diags[0].code), "{name}");
    }
    let Err(diags) = Document::parse(&data(CHECK)) else {
        panic!("{CHECK} accepted")
    };
    let codes: Vec<&str> = diags.iter().map(|d| d.code).collect();
    assert_eq!(codes, ["SRC009", "SRC007"]);
}

#[test]
fn every_code_of_the_reader_has_its_file() {
    let catalogue: BTreeSet<&str> = Catalog::english()
        .keys()
        .filter(|k| k.starts_with("SRC") && k[3..].starts_with(|c: char| c.is_ascii_digit()))
        .map(|k| &k[..6])
        .filter(|code| *code != "SRC015")
        .collect();
    let files: BTreeSet<&str> = REFUSED.iter().map(|r| &r.1[..6]).collect();
    assert_eq!(files, catalogue);
}

#[test]
fn the_documentation_shows_every_file_as_it_is() {
    let section = examples_section();
    for name in ACCEPTED.iter().chain(REFUSED.iter().map(|r| &r.0)) {
        let bytes = data(name);
        let shown = escaped(&bytes);
        if !ESCAPED.contains(name) {
            assert_eq!(shown.as_bytes(), bytes, "{name} is text");
        }
        let block = format!("`{name}`");
        let after = section
            .split(&block)
            .nth(1)
            .unwrap_or_else(|| panic!("{name} not named"));
        let first = after.split("```\n").nth(1).unwrap_or_default();
        assert_eq!(first, shown, "{name} not shown as it is");
    }
    // The canonical form of the unfolded file follows it.
    let after = section.split(&format!("`{UNFOLDED}`")).nth(1).unwrap();
    let canonical = Document::parse(&data(UNFOLDED)).unwrap().to_text();
    assert_eq!(after.split("```\n").nth(3).unwrap(), canonical);
    // The file of the section on `check` is shown there.
    assert!(doc().contains(&format!("```\n{}```", text(CHECK))));
}
