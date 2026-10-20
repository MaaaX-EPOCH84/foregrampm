// ForegramPM — example: read a graph and its layout, check them, measure.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! The whole library in one pass: bytes in, values or refusals out. The two
//! files are embedded at compile time; a real caller reads them itself, the
//! library never does.

use foregram_graph::{Catalog, Graph, Layout, Metrics, file_report};

const GRAPH: &[u8] = include_bytes!("../tests/data/components.graph.json");
const LAYOUT: &[u8] = include_bytes!("../tests/data/components.layout.json");

fn main() {
    let catalog = Catalog::english();

    let graph = match Graph::parse(GRAPH) {
        Ok(graph) => graph,
        Err(refusals) => {
            for line in file_report("components.graph.json", &refusals, catalog) {
                eprintln!("{line}");
            }
            std::process::exit(1);
        }
    };
    let layout = match Layout::parse(LAYOUT) {
        Ok(layout) => layout,
        Err(refusals) => {
            for line in file_report("components.layout.json", &refusals, catalog) {
                eprintln!("{line}");
            }
            std::process::exit(1);
        }
    };

    // A layout is valid on its own, then checked against its graph.
    let refusals = layout.check(&graph);
    if !refusals.is_empty() {
        for line in file_report("components.layout.json", &refusals, catalog) {
            eprintln!("{line}");
        }
        std::process::exit(1);
    }

    for line in Metrics::measure(&graph, &layout).report() {
        println!("{}", line.render(catalog));
    }
}
