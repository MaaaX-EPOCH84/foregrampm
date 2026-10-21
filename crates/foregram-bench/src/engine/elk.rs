// ForegramPM — bench: ELK Layered as a reference engine, through the
// Java driver `java/ElkDriver.java`.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later
//
// A single Java process lays out all the graphs of a pass. The dialogue,
// line by line with tab-separated fields, is described at the top of the
// driver; here it is written from a graph and read back as a layout. The
// driver returns everything in absolute, floating-point pixels; each number is rounded
// to the hundredth once, and relative positions are derived from the
// rounded absolute ones, so that a node's border and a link's end
// land on the same hundredth.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use foregram_graph::{
    Diagnostic, EdgePlace, Engine, Graph, Layout, Message, Node, NodePlace, Point, PortPlace, Pos,
    Segment, Size,
};

use super::{Kind, Tools, garbled, number, px};

/// The settings passed to ELK, as the layout cites them.
pub const OPTIONS: &str = "org.eclipse.elk.layered; hierarchyHandling=INCLUDE_CHILDREN when nested; \
                           portConstraints from the ports; portBorderOffset=-size/2; direction from the graph";

/// The jar whose name carries ELK's version.
const CORE_JAR: &str = "org.eclipse.elk.core-";

/// The ELK version present: that of the jar `org.eclipse.elk.core-<version>.jar`
/// in `<dir>/jars`, if the compiled driver is there too.
pub fn installed_version(dir: &Path) -> Option<String> {
    if !dir.join("classes").join("ElkDriver.class").is_file() {
        return None;
    }
    let jars = std::fs::read_dir(dir.join("jars")).ok()?;
    jars.filter_map(Result::ok).find_map(|entry| {
        let name = entry.file_name();
        let name = name.to_str()?;
        let version = name.strip_prefix(CORE_JAR)?.strip_suffix(".jar")?;
        Some(version.to_string())
    })
}

/// JVM options: a constant identity hash, so that ELK always returns
/// the same layout. `RoutingDirector` (ELK 0.12.0) draws the two candidate
/// routes of a loop between opposite sides from a `HashMap` keyed by
/// `PortSide`, whose hash is the object's, different on each JVM;
/// at equal cost the first wins. HotSpot flag, measured in P8: same
/// layout on the largest Ptolemy, 0.14 s more.
pub const JVM_OPTIONS: [&str; 2] = ["-XX:+UnlockExperimentalVMOptions", "-XX:hashCode=2"];

/// The driver's command line, over the cache's jars and classes.
fn command(classpath: &str) -> Command {
    let mut command = Command::new("java");
    command
        .args(JVM_OPTIONS)
        .arg("-cp")
        .arg(classpath)
        .arg("ElkDriver");
    command
}

/// The running driver, ready to lay out the next graph.
pub struct Process {
    tools: Tools,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Process {
    /// Starts `java` on the driver, with all the cache's jars.
    pub fn start(tools: Tools) -> Result<Process, Diagnostic> {
        let failed = |detail: String| {
            let message = Message::new("R003")
                .text("engine", Kind::Elk.name())
                .text("detail", detail);
            Diagnostic::new(Pos::NONE, message)
        };
        let jars = std::fs::read_dir(tools.path.join("jars")).map_err(|e| failed(e.to_string()))?;
        let mut classpath: Vec<String> = jars
            .filter_map(Result::ok)
            .map(|entry| entry.path().display().to_string())
            .filter(|path| path.ends_with(".jar"))
            .collect();
        classpath.sort();
        classpath.push(tools.path.join("classes").display().to_string());
        let mut child = command(&classpath.join(":"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| failed(e.to_string()))?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(failed(String::from("no pipe")));
        };
        Ok(Process {
            tools,
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    /// Lays out a graph: writes its description, reads the reply up to `end`.
    pub fn layout(&mut self, graph: &Graph) -> Result<Layout, Diagnostic> {
        let failed = |detail: String| {
            let message = Message::new("R003")
                .text("engine", Kind::Elk.name())
                .text("detail", detail);
            Diagnostic::new(Pos::NONE, message)
        };
        let request = lower(graph);
        self.stdin
            .write_all(request.as_bytes())
            .and_then(|()| self.stdin.flush())
            .map_err(|e| failed(e.to_string()))?;
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            let read = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| failed(e.to_string()))?;
            if read == 0 {
                return Err(failed(String::from("the driver stopped")));
            }
            let line = line.trim_end_matches(['\n', '\r']).to_string();
            if line == "end" {
                break;
            }
            lines.push(line);
        }
        let engine = Engine {
            name: String::from("elk"),
            version: self.tools.version.clone(),
            options: Some(String::from(OPTIONS)),
        };
        read(graph, engine, &lines)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // Closing the input is enough: the driver stops at the end of the text.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ------------------------------------------------------------- lowering

/// The constraint level of a node's ports, ELK's: the most demanding
/// that its ports, or the sides imposed on the links reaching it, ask for.
fn constraints(graph: &Graph, node: &Node) -> &'static str {
    if node.ports.iter().any(|p| p.position.is_some()) {
        "FIXED_POS"
    } else if node.ports_ordered {
        "FIXED_ORDER"
    } else if node.ports.iter().any(|p| p.side.is_some())
        || graph.edges.iter().any(|e| {
            (e.source == node.id && e.source_side.is_some())
                || (e.target == node.id && e.target_side.is_some())
        })
    {
        "FIXED_SIDE"
    } else {
        "FREE"
    }
}

fn opt<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(|| String::from("-"), |v| v.to_string())
}

/// A graph's description for the driver.
pub fn lower(graph: &Graph) -> String {
    let mut out = format!(
        "graph\t{}\t{}\n",
        graph.name,
        opt(graph.direction.map(|d| d.as_str()))
    );
    for (node, parent) in graph.all_nodes() {
        let size = node.size.filter(|_| !node.is_container());
        out.push_str(&format!(
            "node\t{}\t{}\t{}\t{}\t{}\t{}\n",
            node.id,
            opt(parent.map(|p| &p.id)),
            opt(size.map(|s| px(s.width))),
            opt(size.map(|s| px(s.height))),
            opt(node.padding.map(px)),
            constraints(graph, node)
        ));
        for (index, port) in node.ports.iter().enumerate() {
            out.push_str(&format!(
                "port\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                port.id,
                node.id,
                opt(port.side.map(|s| s.as_str())),
                opt(port.position.map(|p| px(p.x))),
                opt(port.position.map(|p| px(p.y))),
                opt(port.size.map(|s| px(s.width))),
                opt(port.size.map(|s| px(s.height))),
                opt(node.ports_ordered.then_some(index)),
            ));
        }
    }
    for edge in &graph.edges {
        out.push_str(&format!(
            "edge\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            edge.id,
            edge.source,
            edge.target,
            opt(edge.source_port.as_ref()),
            opt(edge.target_port.as_ref()),
            opt(edge.source_side.map(|s| s.as_str())),
            opt(edge.target_side.map(|s| s.as_str())),
        ));
        for label in &edge.labels {
            out.push_str(&format!(
                "label\t{}\t{}\t{}\t{}\n",
                edge.id,
                px(label.width),
                px(label.height),
                label.at.as_str()
            ));
        }
    }
    out.push_str("layout\n");
    out
}

// -------------------------------------------------------------- reading

/// Builds the layout from the lines the driver returned, `end`
/// excluded.
pub fn read(graph: &Graph, engine: Engine, lines: &[String]) -> Result<Layout, Diagnostic> {
    let mut nodes: Vec<NodePlace> = Vec::new();
    // Per placed node: its absolute corner, to which children and ports are related.
    let mut corners: Vec<(String, Point)> = Vec::new();
    let mut edges: Vec<EdgePlace> = Vec::new();
    let all = graph.all_nodes();
    let corner_of = |corners: &[(String, Point)], id: &str| {
        corners.iter().find(|(n, _)| n == id).map(|(_, p)| *p)
    };
    for line in lines {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["layout", name] if *name == graph.name => {}
            ["fail", _, detail] => {
                let message = Message::new("R003")
                    .text("engine", Kind::Elk.name())
                    .text("detail", *detail);
                return Err(Diagnostic::new(Pos::NONE, message));
            }
            ["node", id, x, y, width, height] => {
                let Some((node, parent)) = all.iter().find(|(n, _)| n.id == *id) else {
                    return Err(garbled(Kind::Elk, format!("node {id}")));
                };
                let corner = Point {
                    x: number(Kind::Elk, x)?,
                    y: number(Kind::Elk, y)?,
                };
                let size = Size {
                    width: number(Kind::Elk, width)?,
                    height: number(Kind::Elk, height)?,
                };
                if let Some(given) = node.size.filter(|_| !node.is_container())
                    && given != size
                {
                    let message = Message::new("R007")
                        .text("engine", Kind::Elk.name())
                        .text("id", *id)
                        .text("width", px(size.width))
                        .text("height", px(size.height));
                    return Err(Diagnostic::new(Pos::NONE, message));
                }
                let origin = parent
                    .and_then(|p| corner_of(&corners, &p.id))
                    .unwrap_or(Point { x: 0, y: 0 });
                corners.push((node.id.clone(), corner));
                nodes.push(NodePlace {
                    id: node.id.clone(),
                    x: corner.x - origin.x,
                    y: corner.y - origin.y,
                    size: node.is_container().then_some(size),
                    ports: Vec::new(),
                    pos: Pos::NONE,
                });
            }
            ["port", node_id, id, x, y, width, height] => {
                let Some(corner) = corner_of(&corners, node_id) else {
                    return Err(garbled(Kind::Elk, format!("port {id} of {node_id}")));
                };
                // The driver returns the box's corner; the anchor is its centre.
                let anchor = Point {
                    x: number(Kind::Elk, x)? + number(Kind::Elk, width)? / 2 - corner.x,
                    y: number(Kind::Elk, y)? + number(Kind::Elk, height)? / 2 - corner.y,
                };
                let Some(place) = nodes.iter_mut().find(|n| n.id == *node_id) else {
                    return Err(garbled(Kind::Elk, format!("port {id} of {node_id}")));
                };
                place.ports.push(PortPlace {
                    id: (*id).to_string(),
                    x: anchor.x,
                    y: anchor.y,
                    pos: Pos::NONE,
                });
            }
            ["edge", id, coordinates @ ..] => {
                if coordinates.len() < 4 || coordinates.len() % 2 != 0 {
                    return Err(garbled(
                        Kind::Elk,
                        format!("edge {id} with {} numbers", coordinates.len()),
                    ));
                }
                let mut route = Vec::new();
                for pair in coordinates.chunks(2) {
                    if let [x, y] = pair {
                        route.push(Segment::Line(Point {
                            x: number(Kind::Elk, x)?,
                            y: number(Kind::Elk, y)?,
                        }));
                    }
                }
                edges.push(EdgePlace {
                    id: (*id).to_string(),
                    route,
                    labels: Vec::new(),
                    pos: Pos::NONE,
                });
            }
            ["label", edge_id, x, y] => {
                let Some(place) = edges.iter_mut().find(|e| e.id == *edge_id) else {
                    return Err(garbled(Kind::Elk, format!("label of {edge_id}")));
                };
                place.labels.push(Point {
                    x: number(Kind::Elk, x)?,
                    y: number(Kind::Elk, y)?,
                });
            }
            _ => return Err(garbled(Kind::Elk, line.clone())),
        }
    }
    // A node's ports in the graph's order, for a stable output.
    for place in &mut nodes {
        if let Some(node) = graph.node(&place.id) {
            place
                .ports
                .sort_by_key(|p| node.ports.iter().position(|q| q.id == p.id));
        }
    }
    Ok(Layout {
        graph: graph.name.clone(),
        engine,
        nodes,
        edges,
        pos: Pos::NONE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn components() -> Graph {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../foregram-graph/tests/data/components.graph.json"
        );
        Graph::parse(&std::fs::read(path).unwrap()).unwrap()
    }

    fn engine() -> Engine {
        Engine {
            name: String::from("elk"),
            version: String::from("test"),
            options: None,
        }
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn the_components_graph_becomes_a_request() {
        let expected = "graph\tcomponents\tright\n\
            node\tClient\t-\t100.00\t60.00\t-\tFIXED_SIDE\n\
            port\treq\tClient\teast\t-\t-\t-\t-\t-\n\
            node\tSystem\t-\t-\t-\t10.00\tFREE\n\
            node\tCore\tSystem\t120.00\t60.00\t-\tFIXED_POS\n\
            port\tin\tCore\twest\t-\t-\t-\t-\t0\n\
            port\tout\tCore\teast\t120.00\t30.00\t6.00\t6.00\t1\n\
            node\tStore\tSystem\t80.00\t50.00\t-\tFIXED_SIDE\n\
            edge\te1\tClient\tCore\treq\tin\t-\t-\n\
            label\te1\t30.00\t14.00\tcenter\n\
            edge\te2\tCore\tStore\tout\t-\t-\twest\n\
            edge\te3\tStore\tSystem\t-\t-\t-\t-\n\
            layout\n";
        assert_eq!(lower(&components()), expected);
    }

    #[test]
    fn constraints_follow_the_most_demanding_port() {
        let text = r#"{"format": "foregram-graph", "version": 2, "name": "g", "nodes": [
            {"id": "a", "width": 100, "height": 100},
            {"id": "b", "width": 100, "height": 100, "ports": [{"id": "p"}]},
            {"id": "c", "width": 100, "height": 100, "ports_ordered": true, "ports": [{"id": "q", "side": "north"}]}],
            "edges": [{"id": "e", "source": "a", "target": "b", "target_side": "south"}]}"#;
        let graph = Graph::parse(text.as_bytes()).unwrap();
        let levels: Vec<&str> = graph.nodes.iter().map(|n| constraints(&graph, n)).collect();
        assert_eq!(levels, ["FREE", "FIXED_SIDE", "FIXED_ORDER"]);
        assert!(lower(&graph).contains("edge\te\ta\tb\t-\t-\t-\tsouth\n"));
    }

    /// The driver's reply on `components`, ELK 0.12.0, as it came out.
    const REPLY: &str = "layout\tcomponents\n\
        node\tClient\t12.0\t12.0\t100.0\t60.0\n\
        port\tClient\treq\t112.0\t42.0\t0.0\t0.0\n\
        node\tSystem\t187.0\t32.0\t243.0\t80.0\n\
        node\tCore\t197.0\t42.0\t120.0\t60.0\n\
        port\tCore\tout\t314.0\t69.0\t6.0\t6.0\n\
        port\tCore\tin\t197.0\t42.0\t0.0\t0.0\n\
        node\tStore\t340.0\t47.0\t80.0\t50.0\n\
        edge\te1\t112.0\t42.0\t197.0\t42.0\n\
        edge\te2\t320.0\t72.0\t340.0\t72.0\n\
        edge\te3\t420.0\t72.0\t430.0\t72.0\n\
        label\te1\t132.0\t45.0\n";

    #[test]
    fn the_reply_becomes_a_valid_layout_with_relative_positions() {
        let graph = components();
        let layout = read(&graph, engine(), &lines(REPLY)).unwrap();
        let node = |id: &str| layout.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!((node("Client").x, node("Client").y), (1200, 1200));
        assert_eq!((node("System").x, node("System").y), (18700, 3200));
        assert_eq!(
            node("System").size,
            Some(Size {
                width: 24300,
                height: 8000
            })
        );
        assert_eq!((node("Core").x, node("Core").y), (1000, 1000));
        assert_eq!(node("Core").size, None);
        // Ports come back in the graph's order, anchored at the centre of their box.
        let ports: Vec<(&str, i64, i64)> = node("Core")
            .ports
            .iter()
            .map(|p| (p.id.as_str(), p.x, p.y))
            .collect();
        assert_eq!(ports, [("in", 0, 0), ("out", 12000, 3000)]);
        assert_eq!(layout.edges[0].labels, vec![Point { x: 13200, y: 4500 }]);
        assert_eq!(
            layout.edges[1].route,
            [
                Segment::Line(Point { x: 32000, y: 7200 }),
                Segment::Line(Point { x: 34000, y: 7200 })
            ]
        );
        assert!(
            layout.check(&graph).is_empty(),
            "{:?}",
            layout.check(&graph)
        );
    }

    #[test]
    fn a_failure_a_changed_leaf_and_a_garbled_reply_are_refused() {
        let graph = components();
        let refusal = |text: &str| read(&graph, engine(), &lines(text)).unwrap_err();
        let failed = refusal("fail\tcomponents\tNullPointerException: boom");
        assert_eq!(failed.code, "R003");
        assert_eq!(failed.text(), "elk failed: NullPointerException: boom");
        let grown = refusal("node\tClient\t0\t0\t100.5\t60.0");
        assert_eq!(grown.code, "R007");
        assert_eq!(
            grown.text(),
            "elk changed the size of leaf \"Client\" to 100.50 × 60.00"
        );
        assert_eq!(refusal("node\tNobody\t0\t0\t1\t1").code, "R005");
        assert_eq!(refusal("port\tClient\treq\t0\t0\t0\t0").code, "R005");
        assert_eq!(refusal("edge\te1\t1\t2\t3").code, "R005");
        assert_eq!(refusal("label\te9\t1\t2").code, "R005");
        assert_eq!(refusal("what\tis\tthis").code, "R005");
        assert_eq!(refusal("node\tClient\t1e30\t0\t100.0\t60.0").code, "R006");
    }

    #[test]
    fn the_driver_runs_with_a_constant_identity_hash() {
        let command = command("a.jar:classes");
        let args: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(command.get_program(), "java");
        assert_eq!(
            args,
            [
                "-XX:+UnlockExperimentalVMOptions",
                "-XX:hashCode=2",
                "-cp",
                "a.jar:classes",
                "ElkDriver"
            ]
        );
    }

    #[test]
    fn the_installed_version_needs_the_driver_and_the_core_jar() {
        let dir = std::env::temp_dir().join("foregram-bench-elk-version");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(installed_version(&dir), None);
        std::fs::create_dir_all(dir.join("classes")).unwrap();
        std::fs::create_dir_all(dir.join("jars")).unwrap();
        std::fs::write(dir.join("classes").join("ElkDriver.class"), b"").unwrap();
        assert_eq!(installed_version(&dir), None);
        std::fs::write(
            dir.join("jars").join("org.eclipse.elk.core-0.12.0.jar"),
            b"",
        )
        .unwrap();
        assert_eq!(installed_version(&dir).as_deref(), Some("0.12.0"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
