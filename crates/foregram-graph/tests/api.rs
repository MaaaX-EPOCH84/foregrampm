// ForegramPM — guard of the public surface: every exported name is listed.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! The surface of `foregram-graph` is frozen from version 0.1.0 on: a
//! caller such as LEON links against it. This test reads `src/lib.rs` and
//! compares what it exports — `pub use`, `pub mod`, `pub fn`, `pub const` —
//! with the list below. A name added, removed or renamed fails here, so
//! that the change is a decision and not a side effect.

// A failing test must stop short: the product rules do not apply here.
#![allow(clippy::unwrap_used, clippy::panic)]

/// The frozen surface, sorted.
const SURFACE: &[&str] = &[
    "Anchor",
    "Arg",
    "COORD_MAX",
    "CURVE_STEPS",
    "Catalog",
    "Diagnostic",
    "Direction",
    "Edge",
    "EdgePlace",
    "Engine",
    "FORMAT_VERSION",
    "GRAPH_FORMAT",
    "Graph",
    "IDENT_MAX_CHARS",
    "LAYER_MAX",
    "LAYOUT_FORMAT",
    "Label",
    "Layout",
    "Message",
    "Metrics",
    "Node",
    "NodePlace",
    "Point",
    "Port",
    "PortPlace",
    "Pos",
    "Rect",
    "Segment",
    "Side",
    "Size",
    "extent",
    "file_report",
    "json",
    "polyline",
];

/// The names `src/lib.rs` exports, read from its text.
fn exported(lib: &str) -> Vec<String> {
    let mut names = Vec::new();
    // Items span lines: comments out, then everything from `pub ` to the
    // end of the item — `;` for a `use`, `mod` or `const`, `{` for a body.
    let text: String = lib
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut rest = text.as_str();
    while let Some(i) = rest.find("pub ") {
        let after = &rest[i + 4..];
        let mut words = after.split_whitespace();
        match words.next() {
            Some("use") => {
                let path = after[3..].split(';').next().unwrap().trim();
                let list = match (path.find('{'), path.find('}')) {
                    (Some(b), Some(e)) => &path[b + 1..e],
                    _ => path.rsplit("::").next().unwrap(),
                };
                for name in list.split(',') {
                    let name = name.trim();
                    if !name.is_empty() {
                        names.push(name.to_string());
                    }
                }
            }
            Some("mod" | "fn" | "const" | "struct" | "enum" | "type" | "trait" | "static") => {
                let name = words.next().unwrap();
                let name = name.split(['(', '<', ':', ';']).next().unwrap();
                names.push(name.to_string());
            }
            other => panic!("unexpected item after `pub`: {other:?}"),
        }
        rest = after;
    }
    names.sort();
    names
}

#[test]
fn the_exported_names_are_exactly_the_frozen_surface() {
    let lib = include_str!("../src/lib.rs");
    let found = exported(lib);
    let expected: Vec<String> = SURFACE.iter().map(|s| s.to_string()).collect();
    assert!(
        SURFACE.windows(2).all(|w| w[0] < w[1]),
        "SURFACE must be sorted and without duplicate"
    );
    assert_eq!(found, expected, "src/lib.rs exports differ from SURFACE");
}

#[test]
fn the_reader_sees_every_kind_of_export() {
    let sample = "pub mod a;\npub use b::{C, d};\npub use e::F;\npub fn g(x: i64) -> i64 {\n 0 }\npub const H: i64 = 1;\n// pub use not::Me;\n";
    assert_eq!(exported(sample), ["C", "F", "H", "a", "d", "g"]);
}
