// ForegramPM — guard of the boundary of the product crates: bytes come in, values go out.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! `foregram-graph`, `foregram-render` and `foregram-source` touch no file,
//! no clock, no environment, no standard output: everything comes in as
//! bytes or `&str`, everything goes out as a returned value. The executable
//! does the input and output. This property is read in the sources, line by
//! line, outside comments and outside unit tests.

// A failing test must stop dead: the product's rules do not apply here.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};

/// Modules of `std` the library has no use for.
const MODULES: [&str; 7] = ["fs", "env", "time", "io", "process", "net", "thread"];

/// Macros that read the build environment or write to a stream.
const MACROS: [&str; 7] = [
    "print!",
    "println!",
    "eprint!",
    "eprintln!",
    "dbg!",
    "env!",
    "option_env!",
];

/// The unit tests that follow this marker are allowed to write.
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

/// The `std::…` paths of a line, a group `std::{a, b::c}` unfolded into
/// `std::a` and `std::b::c`.
fn std_paths(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (at, _) in line.match_indices("std::") {
        let rest = &line[at + "std::".len()..];
        if let Some(group) = rest.strip_prefix('{') {
            let inner = group.split('}').next().unwrap_or("");
            for item in inner.split(',') {
                let item = item.trim();
                if !item.is_empty() {
                    out.push(format!("std::{item}"));
                }
            }
        } else {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':' || c == '*'))
                .unwrap_or(rest.len());
            out.push(format!("std::{}", &rest[..end]));
        }
    }
    out
}

/// What a source line uses that is forbidden.
fn offences(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    if line.trim_start().starts_with("//") {
        return out;
    }
    for path in std_paths(line) {
        let module = path.split("::").nth(1).unwrap_or("");
        if module == "*" || MODULES.contains(&module) {
            out.push(path);
        }
    }
    for name in MACROS {
        for (at, _) in line.match_indices(name) {
            let before = line[..at].chars().next_back();
            if !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                out.push(name.to_string());
            }
        }
    }
    out
}

#[test]
fn product_crates_do_no_input_output() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut seen = 0;
    let mut found = Vec::new();
    for krate in ["foregram-graph", "foregram-render", "foregram-source"] {
        let dir = root.join(krate).join("src");
        let files = sources(&dir);
        assert!(
            files.iter().any(|p| p.ends_with("lib.rs")),
            "{krate} : aucun lib.rs trouvé"
        );
        for path in files {
            seen += 1;
            let text = std::fs::read_to_string(&path).unwrap();
            let product = text.split(MARK).next().unwrap();
            for (index, line) in product.lines().enumerate() {
                for what in offences(line) {
                    found.push(format!(
                        "{}/src/{}:{} : {what}",
                        krate,
                        path.strip_prefix(&dir).unwrap().display(),
                        index + 1
                    ));
                }
            }
        }
    }
    assert!(seen >= 16, "trop peu de sources lues : {seen}");
    assert!(
        found.is_empty(),
        "entrées-sorties dans le produit :\n{}",
        found.join("\n")
    );
}

#[test]
fn the_guard_catches_every_form() {
    let cases = [
        ("use std::fs;", "std::fs"),
        (
            "    let t = std::time::Instant::now();",
            "std::time::Instant::now",
        ),
        ("use ::std::env::var;", "std::env::var"),
        (
            "use std::{collections::BTreeMap, fs::read};",
            "std::fs::read",
        ),
        ("use std::{io, fmt};", "std::io"),
        ("use std::*;", "std::*"),
        ("    println!(\"{x}\");", "println!"),
        ("    let d = dbg!(x);", "dbg!"),
        ("    let p = env!(\"CARGO_MANIFEST_DIR\");", "env!"),
    ];
    for (line, expected) in cases {
        assert_eq!(offences(line), vec![expected.to_string()], "{line}");
    }
    let clean = [
        "use std::collections::BTreeMap;",
        "use std::fmt::Write;",
        "    write!(out, \"{x}\").ok();",
        "    let s = my_print!(x);",
        "    // std::fs is forbidden here",
        "use std::{collections::BTreeSet, fmt};",
    ];
    for line in clean {
        assert!(offences(line).is_empty(), "{line}");
    }
}
