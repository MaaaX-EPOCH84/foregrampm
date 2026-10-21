// ForegramPM — refusal cases of version 2 of the format, shared between the tests.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(clippy::unwrap_used, clippy::panic, dead_code)]

use foregram_graph::{Diagnostic, Graph, Layout};

pub fn data(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path} : {e}"))
}

pub fn graph(nodes: &str, edges: &str) -> String {
    format!(
        "{{\"format\": \"foregram-graph\", \"version\": 2, \"name\": \"g\",\n\
         \"nodes\": [{nodes}],\n\
         \"edges\": [{edges}]}}"
    )
}

pub fn layout(nodes: &str, edges: &str) -> String {
    format!(
        "{{\"format\": \"foregram-layout\", \"version\": 2, \"graph\": \"g\",\n\
         \"engine\": {{\"name\": \"m\", \"version\": \"1\"}},\n\
         \"nodes\": [{nodes}],\n\
         \"edges\": [{edges}]}}"
    )
}

pub const LEAF_A: &str = "{\"id\": \"A\", \"width\": 100, \"height\": 100}";
pub const LEAF_B: &str = "{\"id\": \"B\", \"width\": 100, \"height\": 100}";

/// A container `C` that encloses `A`, with a port `p` on the east of `A`,
/// and a leaf `B` outside with a port `q` on the west.
pub fn nested() -> Graph {
    let nodes = "{\"id\": \"C\", \"padding\": 10, \"children\": [\
                   {\"id\": \"A\", \"width\": 100, \"height\": 100, \
                    \"ports\": [{\"id\": \"p\", \"side\": \"east\", \"x\": 100, \"y\": 50}]}]},\n\
                 {\"id\": \"B\", \"width\": 100, \"height\": 100, \
                  \"ports\": [{\"id\": \"q\", \"side\": \"west\", \"width\": 20, \"height\": 20}]}";
    let edges = "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\", \
                  \"source_port\": \"p\", \"target_port\": \"q\"},\n\
                 {\"id\": \"f\", \"source\": \"A\", \"target\": \"C\"}";
    Graph::parse(graph(nodes, edges).as_bytes()).unwrap()
}

/// A layout of the graph [`nested`]: `C` at (0, 0), 200 × 200; `A` at
/// (10, 10) inside it; `B` at (300, 0); `e` from port `p` to port `q`;
/// `f` from `A` to the bottom edge of `C`, as a curve.
pub const NESTED_LAYOUT_NODES: &str = "{\"id\": \"C\", \"x\": 0, \"y\": 0, \"width\": 200, \"height\": 200},\n\
     {\"id\": \"A\", \"x\": 10, \"y\": 10, \"ports\": [{\"id\": \"p\", \"x\": 100, \"y\": 50}]},\n\
     {\"id\": \"B\", \"x\": 300, \"y\": 0, \"ports\": [{\"id\": \"q\", \"x\": 0, \"y\": 50}]}";
pub const NESTED_LAYOUT_EDGES: &str = "{\"id\": \"e\", \"route\": [[110, 60], [300, 50]]},\n\
     {\"id\": \"f\", \"route\": [[60, 110], [[60, 150], [80, 180], [100, 200]]]}";

/// The refusals specific to version 2, named, so that each test
/// checks its own and the catalogue sees them all.
pub fn cases() -> Vec<(&'static str, Vec<Diagnostic>)> {
    let g = |nodes: &str, edges: &str| Graph::parse(graph(nodes, edges).as_bytes()).unwrap_err();
    let l = |nodes: &str, edges: &str| Layout::parse(layout(nodes, edges).as_bytes()).unwrap_err();
    let check = |nodes: &str, edges: &str| {
        Layout::parse(layout(nodes, edges).as_bytes())
            .unwrap()
            .check(&nested())
    };
    vec![
        (
            "port-twice",
            g(
                "{\"id\": \"A\", \"width\": 1, \"height\": 1, \"ports\": [{\"id\": \"p\"}, {\"id\": \"p\"}]},\n\
                 {\"id\": \"B\", \"width\": 1, \"height\": 1, \"ports\": [{\"id\": \"p\"}]}",
                "",
            ),
        ),
        (
            "edge-port-unknown-or-foreign",
            g(
                "{\"id\": \"A\", \"width\": 1, \"height\": 1, \"ports\": [{\"id\": \"p\"}]},\n\
                 {\"id\": \"B\", \"width\": 1, \"height\": 1}",
                "{\"id\": \"e\", \"source\": \"A\", \"target\": \"B\", \"source_port\": \"zz\"},\n\
                 {\"id\": \"f\", \"source\": \"B\", \"target\": \"A\", \"source_port\": \"p\"},\n\
                 {\"id\": \"g\", \"source\": \"A\", \"target\": \"B\", \"source_port\": \"p\", \"source_side\": \"east\"}",
            ),
        ),
        (
            "ordered-without-side",
            g(
                "{\"id\": \"A\", \"width\": 1, \"height\": 1, \"ports_ordered\": true, \
                  \"ports\": [{\"id\": \"p\", \"side\": \"east\"}, {\"id\": \"q\"}]}",
                "",
            ),
        ),
        (
            "port-position",
            g(
                "{\"id\": \"A\", \"width\": 100, \"height\": 100, \"ports\": [\
                    {\"id\": \"p\", \"x\": 100, \"y\": 50},\n\
                    {\"id\": \"q\", \"side\": \"east\", \"x\": 50, \"y\": 50},\n\
                    {\"id\": \"r\", \"side\": \"east\", \"x\": 100, \"y\": 50, \"width\": 0, \"height\": 1}]},\n\
                 {\"id\": \"C\", \"ports\": [{\"id\": \"s\", \"side\": \"east\", \"x\": 0, \"y\": 0}], \
                  \"children\": [{\"id\": \"D\", \"width\": 1, \"height\": 1}]}",
                "",
            ),
        ),
        (
            "sizes-and-padding",
            g(
                "{\"id\": \"A\"},\n\
                 {\"id\": \"B\", \"width\": 1, \"height\": 1, \"padding\": 5},\n\
                 {\"id\": \"C\", \"width\": 1, \"height\": 1, \"children\": [{\"id\": \"D\", \"width\": 1, \"height\": 1}]}",
                "",
            ),
        ),
        (
            "half-size",
            g(
                "{\"id\": \"E\", \"width\": 1},\n\
                 {\"id\": \"F\", \"height\": 1, \"ports\": [{\"id\": \"p\", \"height\": 1}, {\"id\": \"q\", \"x\": 1}]}",
                "",
            ),
        ),
        (
            "bad-port-identifier",
            g(
                "{\"id\": \"A\", \"width\": 1, \"height\": 1, \"ports\": [{\"id\": \"\"}]}",
                "",
            ),
        ),
        (
            "layout-ports-and-curves",
            l(
                "{\"id\": \"A\", \"x\": 0, \"y\": 0, \"ports\": [{\"id\": \"p\", \"x\": 0, \"y\": 0}, {\"id\": \"p\", \"x\": 1, \"y\": 1}]}",
                "{\"id\": \"e\", \"route\": [[[0, 0], [1, 1], [2, 2]], [3, 3]]}",
            ),
        ),
        (
            "layout-half-size-and-label-point",
            l(
                "{\"id\": \"B\", \"x\": 0, \"y\": 0, \"width\": 1, \"ports\": [{\"id\": \"p\", \"x\": 0}]}",
                "{\"id\": \"e\", \"route\": [[0, 0], [1, 1]], \"labels\": [[1]]}",
            ),
        ),
        (
            "check-nesting-and-ports",
            check(
                "{\"id\": \"C\", \"x\": 0, \"y\": 0, \"width\": 150, \"height\": 150},\n\
                 {\"id\": \"A\", \"x\": 45, \"y\": 10, \"width\": 1, \"height\": 1, \
                  \"ports\": [{\"id\": \"p\", \"x\": 100, \"y\": 40}, {\"id\": \"zz\", \"x\": 0, \"y\": 0}]},\n\
                 {\"id\": \"B\", \"x\": 300, \"y\": 0}",
                "{\"id\": \"e\", \"route\": [[145, 50], [300, 100]]},\n\
                 {\"id\": \"f\", \"route\": [[100, 60], [100, 150]]}",
            ),
        ),
        (
            "check-port-sides",
            check(
                "{\"id\": \"C\", \"x\": 0, \"y\": 0},\n\
                 {\"id\": \"A\", \"x\": 10, \"y\": 10, \"ports\": [{\"id\": \"p\", \"x\": 100, \"y\": 50}]},\n\
                 {\"id\": \"B\", \"x\": 300, \"y\": 0, \"ports\": [{\"id\": \"q\", \"x\": 100, \"y\": 50}]}",
                "",
            ),
        ),
        (
            "check-route-off-port-start",
            check(
                NESTED_LAYOUT_NODES,
                "{\"id\": \"e\", \"route\": [[110, 61], [300, 50]]},\n\
                 {\"id\": \"f\", \"route\": [[60, 110], [100, 200]]}",
            ),
        ),
        (
            "check-port-interior",
            check(
                "{\"id\": \"C\", \"x\": 0, \"y\": 0, \"width\": 200, \"height\": 200},\n\
                 {\"id\": \"A\", \"x\": 10, \"y\": 10, \"ports\": [{\"id\": \"p\", \"x\": 100, \"y\": 50}]},\n\
                 {\"id\": \"B\", \"x\": 300, \"y\": 0, \"ports\": [{\"id\": \"q\", \"x\": 50, \"y\": 50}]}",
                NESTED_LAYOUT_EDGES,
            ),
        ),
    ]
}
