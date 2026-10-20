// ForegramPM — example: draw a layout as SVG.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! From a graph and its layout to an SVG string, with the default style. The
//! two files are embedded at compile time; a real caller reads them itself,
//! the library never does. The PNG goes the same way, behind the `png`
//! feature, with `foregram_render::png(&graph, &layout, &style, scale)`.

use foregram_graph::{Catalog, Graph, Layout, file_report};
use foregram_render::{Style, svg};

const GRAPH: &[u8] = include_bytes!("../../foregram-graph/tests/data/components.graph.json");
const LAYOUT: &[u8] = include_bytes!("../../foregram-graph/tests/data/components.layout.json");

fn main() {
    let catalog = Catalog::english();

    let parsed =
        Graph::parse(GRAPH).and_then(|graph| Layout::parse(LAYOUT).map(|layout| (graph, layout)));
    let (graph, layout) = match parsed {
        Ok(pair) => pair,
        Err(refusals) => {
            for line in file_report("components", &refusals, catalog) {
                eprintln!("{line}");
            }
            std::process::exit(1);
        }
    };

    let style = Style::default();
    print!("{}", svg(&graph, &layout, &style));
}
