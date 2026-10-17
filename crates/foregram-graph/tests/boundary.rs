// ForegramPM — garde de la frontière des crates de produit : des octets entrent, des valeurs sortent.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! `foregram-graph` et `foregram-render` ne touchent ni fichier, ni horloge,
//! ni environnement, ni sortie standard : tout entre par des octets ou des
//! `&str`, tout sort par une valeur rendue. L'exécutable fait les
//! entrées-sorties. Cette propriété se lit dans les sources, ligne par
//! ligne, hors commentaires et hors tests unitaires.

// Un test qui échoue doit s'arrêter net : les règles du produit ne valent pas ici.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};

/// Modules de `std` dont la bibliothèque n'a pas l'usage.
const MODULES: [&str; 7] = ["fs", "env", "time", "io", "process", "net", "thread"];

/// Macros qui lisent l'environnement de compilation ou écrivent sur un flux.
const MACROS: [&str; 7] = [
    "print!",
    "println!",
    "eprint!",
    "eprintln!",
    "dbg!",
    "env!",
    "option_env!",
];

/// Les tests unitaires qui suivent cette marque ont le droit d'écrire.
const MARK: &str = concat!("#[cfg", "(test)]");

/// Les sources `.rs` d'un répertoire, sous-répertoires compris, triés.
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

/// Les chemins `std::…` d'une ligne, un groupe `std::{a, b::c}` déplié en
/// `std::a` et `std::b::c`.
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

/// Ce qu'une ligne de source emploie de défendu.
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
    for krate in ["foregram-graph", "foregram-render"] {
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
    assert!(seen >= 12, "trop peu de sources lues : {seen}");
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
