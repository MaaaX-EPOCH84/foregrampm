// ForegramPM — measurement bench: Graphviz `dot` as a reference engine.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The graph becomes DOT text, `dot -Tjson0` places it, and its JSON output —
// strings, no floats — is read back with the format's strict reader.
// One pixel of the format equals one Graphviz point; positions come out in
// hundredths of a point, routes in thousandths, rounded to hundredths.

use std::path::{Path, PathBuf};
use std::process::Command;

use foregram_graph::json::{self, Kind as J, Value};
use foregram_graph::{
    Anchor, Diagnostic, Direction, EdgePlace, Engine, Graph, Layout, Node, NodePlace, Point,
    PortPlace, Pos, Segment, Side, Size,
};

use super::{Kind, Tools, garbled, number, px};

/// The settings passed to `dot`, as the layout cites them.
pub const OPTIONS: &str = "dot -Tjson0; shape=box fixedsize=true; arrowhead=none; compound=true; \
                           labels as fixed-size HTML tables; rankdir from direction";

/// Margin of a container without `padding`, in points: the one `dot` gives a
/// default `cluster`, written out so a nested container does not inherit
/// its parent's.
const DEFAULT_MARGIN: &str = "8";

/// The executable, in the cache of `corpus/fetch.sh`.
pub fn executable(cache: &Path) -> PathBuf {
    cache.join("graphviz").join("usr").join("bin").join("dot")
}

/// The shared library next to the executable.
fn lib_dir(executable: &Path) -> PathBuf {
    executable
        .parent()
        .and_then(Path::parent)
        .map(|usr| usr.join("lib"))
        .unwrap_or_default()
}

fn command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    command.env("LD_LIBRARY_PATH", lib_dir(executable));
    command
}

/// The version that `dot -V` reports: `dot - graphviz version 16.1.0 (…)`.
pub fn installed_version(executable: &Path) -> Option<String> {
    let output = command(executable).arg("-V").output().ok()?;
    let text = String::from_utf8_lossy(&output.stderr);
    let mut words = text.split_whitespace();
    words.find(|w| *w == "version")?;
    words.next().map(str::to_string)
}

/// Places a graph with `dot`.
pub fn layout(tools: &Tools, graph: &Graph) -> Result<Layout, Diagnostic> {
    let source = lower(graph);
    let mut command = command(&tools.path);
    command.arg("-Tjson0");
    let out = super::run(Kind::Dot, &mut command, source.as_bytes(), super::TIMEOUT)?;
    let engine = Engine {
        name: String::from("dot"),
        version: tools.version.clone(),
        options: Some(String::from(OPTIONS)),
    };
    read(graph, engine, &out)
}

// ------------------------------------------------------------- lowering

/// The graph's nodes in the order of `Graph::all_nodes`, with the rank of
/// each: `dot` knows them as `n<rank>`, and a container as
/// `cluster<rank>`.
struct Names<'a> {
    nodes: Vec<&'a Node>,
}

impl<'a> Names<'a> {
    fn new(graph: &'a Graph) -> Self {
        Names {
            nodes: graph.all_nodes().into_iter().map(|(n, _)| n).collect(),
        }
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// The node `dot` attaches to: the leaf itself, or the first leaf
    /// of a container, which a group head or tail (`lhead`, `ltail`)
    /// then pulls back onto the group's edge.
    fn stand_in(&self, index: usize) -> usize {
        let mut at = index;
        while let Some(node) = self.nodes.get(at)
            && node.is_container()
        {
            at += 1;
        }
        at
    }
}

/// The DOT text of a graph.
pub fn lower(graph: &Graph) -> String {
    let names = Names::new(graph);
    let mut out = String::from("digraph g {\n  graph [compound=true");
    if let Some(direction) = graph.direction {
        let rankdir = match direction {
            Direction::Right => "LR",
            Direction::Down => "TB",
            Direction::Left => "RL",
            Direction::Up => "BT",
        };
        out.push_str(&format!(", rankdir={rankdir}"));
    }
    out.push_str("]\n  node [shape=box, fixedsize=true, label=\"\"]\n  edge [arrowhead=none]\n");
    let mut index = 0;
    for node in &graph.nodes {
        lower_node(node, &mut index, 1, &mut out);
    }
    for (rank, edge) in graph.edges.iter().enumerate() {
        let (Some(source), Some(target)) = (names.index(&edge.source), names.index(&edge.target))
        else {
            continue;
        };
        let mut attributes = vec![format!("id=\"e{rank}\"")];
        let mut end = |index: usize, port: &Option<String>, side: Option<Side>, group: &str| {
            let node = names.nodes[index];
            if node.is_container() {
                attributes.push(format!("{group}=cluster{index}"));
                return format!("n{}", names.stand_in(index));
            }
            let side = side.or_else(|| {
                port.as_ref()
                    .and_then(|p| node.port(p))
                    .and_then(|p| p.side)
            });
            match side {
                Some(side) => format!("n{index}:{}", compass(side)),
                None => format!("n{index}"),
            }
        };
        let tail = end(source, &edge.source_port, edge.source_side, "ltail");
        let head = end(target, &edge.target_port, edge.target_side, "lhead");
        for label in &edge.labels {
            let key = match label.at {
                Anchor::Source => "taillabel",
                Anchor::Center => "label",
                Anchor::Target => "headlabel",
            };
            attributes.push(format!("{key}={}", html_label(label.width, label.height)));
        }
        out.push_str(&format!("  {tail} -> {head} [{}]\n", attributes.join(", ")));
    }
    out.push_str("}\n");
    out
}

fn lower_node(node: &Node, index: &mut usize, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    let rank = *index;
    *index += 1;
    match node.size {
        Some(size) if !node.is_container() => {
            out.push_str(&format!(
                "{indent}n{rank} [width={}, height={}]\n",
                inches(size.width),
                inches(size.height)
            ));
        }
        _ => {
            let margin = node.padding.map_or(String::from(DEFAULT_MARGIN), px);
            out.push_str(&format!(
                "{indent}subgraph cluster{rank} {{\n{indent}  margin={margin}\n"
            ));
            for child in &node.children {
                lower_node(child, index, depth + 1, out);
            }
            out.push_str(&format!("{indent}}}\n"));
        }
    }
}

/// Hundredths of a point to inches, with six decimals: `dot` takes
/// node sizes only in inches.
fn inches(hundredths: i64) -> String {
    let whole = hundredths / 7200;
    let rest = hundredths % 7200;
    let micro = (rest * 1_000_000 + 3600) / 7200;
    format!("{}.{:06}", whole + micro / 1_000_000, micro % 1_000_000)
}

fn compass(side: Side) -> &'static str {
    match side {
        Side::North => "n",
        Side::South => "s",
        Side::East => "e",
        Side::West => "w",
    }
}

/// A fixed-size label: an empty HTML table. `dot` reads these
/// sizes only in whole points, rounded to nearest.
fn html_label(width: i64, height: i64) -> String {
    let (w, h) = ((width + 50) / 100, (height + 50) / 100);
    format!(
        "<<TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLSPACING=\"0\" CELLPADDING=\"0\" \
         FIXEDSIZE=\"TRUE\" WIDTH=\"{w}\" HEIGHT=\"{h}\">\
         <TR><TD FIXEDSIZE=\"TRUE\" WIDTH=\"{w}\" HEIGHT=\"{h}\"></TD></TR></TABLE>>"
    )
}

// -------------------------------------------------------------- reading

/// An absolute rectangle read from the output, y axis already flipped.
#[derive(Clone, Copy)]
struct Abs {
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}

/// What `dot` returns for an edge: its route and the centre of each label.
type Routed = (Vec<Segment>, Vec<(Anchor, Point)>);

/// Builds the layout from the `-Tjson0` output of `dot`.
pub fn read(graph: &Graph, engine: Engine, bytes: &[u8]) -> Result<Layout, Diagnostic> {
    let root = json::parse(bytes).map_err(|d| garbled(Kind::Dot, d.text()))?;
    let names = Names::new(graph);
    // The frame: its bottom-left corner and its top, to flip the y axis.
    let bb = member(&root, "bb").ok_or_else(|| garbled(Kind::Dot, "no bb"))?;
    let [left, _, _, top] = quad(bb)?;
    let flip = |x: i64, y: i64| Point {
        x: x - left,
        y: top - y,
    };

    // The objects: `n<rank>` with its centre, `cluster<rank>` with its frame.
    let mut boxes: Vec<Option<Abs>> = vec![None; names.nodes.len()];
    for object in array(&root, "objects")? {
        let name = string(object, "name")?;
        if let Some(rank) = name.strip_prefix("cluster") {
            let rank = rank_of(rank, names.nodes.len())?;
            let [l, b, r, t] = quad(need(object, "bb")?)?;
            let origin = flip(l, t);
            boxes[rank] = Some(Abs {
                x: origin.x,
                y: origin.y,
                width: r - l,
                height: t - b,
            });
        } else if let Some(rank) = name.strip_prefix('n') {
            let rank = rank_of(rank, names.nodes.len())?;
            let Some(size) = names.nodes[rank].size else {
                continue;
            };
            let (cx, cy) = pair(need(object, "pos")?)?;
            let center = flip(cx, cy);
            boxes[rank] = Some(Abs {
                x: center.x - size.width / 2,
                y: center.y - size.height / 2,
                width: size.width,
                height: size.height,
            });
        }
    }

    // The edges, by their `id` attribute.
    let mut routes: Vec<Option<Routed>> = vec![None; graph.edges.len()];
    for object in array(&root, "edges")? {
        let id = string(object, "id")?;
        let rank = id
            .strip_prefix('e')
            .ok_or_else(|| garbled(Kind::Dot, format!("edge id {id}")))?;
        let rank = rank_of(rank, graph.edges.len())?;
        let route = route(need(object, "pos")?, &flip)?;
        let mut labels = Vec::new();
        for (key, at) in [
            ("tail_lp", Anchor::Source),
            ("lp", Anchor::Center),
            ("head_lp", Anchor::Target),
        ] {
            if let Some(value) = member(object, key) {
                let (x, y) = pair(value)?;
                labels.push((at, flip(x, y)));
            }
        }
        routes[rank] = Some((route, labels));
    }

    // The layout: positions relative to the parent, port anchors.
    let mut nodes = Vec::new();
    for (rank, (node, parent)) in graph.all_nodes().into_iter().enumerate() {
        let Some(abs) = boxes[rank] else {
            continue;
        };
        let origin = parent
            .and_then(|p| names.index(&p.id))
            .and_then(|i| boxes[i])
            .map_or((0, 0), |p| (p.x, p.y));
        let ports = node
            .ports
            .iter()
            .map(|port| {
                let anchor = port_anchor(graph, node, port.id.as_str(), abs, &routes);
                PortPlace {
                    id: port.id.clone(),
                    x: anchor.x,
                    y: anchor.y,
                    pos: Pos::NONE,
                }
            })
            .collect();
        nodes.push(NodePlace {
            id: node.id.clone(),
            x: abs.x - origin.0,
            y: abs.y - origin.1,
            size: node.is_container().then_some(Size {
                width: abs.width,
                height: abs.height,
            }),
            ports,
            pos: Pos::NONE,
        });
    }
    let mut edges = Vec::new();
    for (edge, placed) in graph.edges.iter().zip(routes) {
        let Some((route, placed_labels)) = placed else {
            continue;
        };
        // Each label of the graph takes the position of its end; `dot` places
        // only one per end, the second stays without a position.
        let mut taken = Vec::new();
        let labels = edge
            .labels
            .iter()
            .filter_map(|label| {
                let found = placed_labels
                    .iter()
                    .enumerate()
                    .find(|(i, (at, _))| *at == label.at && !taken.contains(i))?;
                taken.push(found.0);
                let center = found.1.1;
                Some(Point {
                    x: center.x - label.width / 2,
                    y: center.y - label.height / 2,
                })
            })
            .collect();
        edges.push(EdgePlace {
            id: edge.id.clone(),
            route,
            labels,
            pos: Pos::NONE,
        });
    }
    Ok(Layout {
        graph: graph.name.clone(),
        engine,
        nodes,
        edges,
        pos: Pos::NONE,
    })
}

/// The anchor of a port, which `dot` does not know: the position the graph
/// fixes; otherwise the end of the first edge that uses it, pulled back onto the edge of the
/// node — `dot` attaches a compass point one point outside; otherwise the middle
/// of its side, north by default. Relative to the node.
fn port_anchor(
    graph: &Graph,
    node: &Node,
    port_id: &str,
    abs: Abs,
    routes: &[Option<Routed>],
) -> Point {
    let Some(port) = node.port(port_id) else {
        return Point { x: 0, y: 0 };
    };
    if let Some(position) = port.position {
        return position;
    }
    for (edge, placed) in graph.edges.iter().zip(routes) {
        let Some((route, _)) = placed else {
            continue;
        };
        let end = if edge.source_port.as_deref() == Some(port_id) {
            route.first()
        } else if edge.target_port.as_deref() == Some(port_id) {
            route.last()
        } else {
            None
        };
        if let Some(end) = end.map(Segment::end) {
            let rect = foregram_graph::Rect {
                x: abs.x,
                y: abs.y,
                width: abs.width,
                height: abs.height,
            };
            let on_border = super::fit::onto_border(rect, port.side, end);
            return Point {
                x: on_border.x - abs.x,
                y: on_border.y - abs.y,
            };
        }
    }
    match port.side.unwrap_or(Side::North) {
        Side::North => Point {
            x: abs.width / 2,
            y: 0,
        },
        Side::South => Point {
            x: abs.width / 2,
            y: abs.height,
        },
        Side::West => Point {
            x: 0,
            y: abs.height / 2,
        },
        Side::East => Point {
            x: abs.width,
            y: abs.height / 2,
        },
    }
}

/// The route: `x,y x,y …`, a point then cubic Bézier triplets. The
/// arrowed ends (`e,x,y`, `s,x,y`) do not appear without an arrowhead.
fn route(value: &Value, flip: &dyn Fn(i64, i64) -> Point) -> Result<Vec<Segment>, Diagnostic> {
    let text = text_of(value)?;
    let mut points = Vec::new();
    for word in text.split_whitespace() {
        let (x, y) = word
            .split_once(',')
            .ok_or_else(|| garbled(Kind::Dot, format!("route point {word}")))?;
        points.push(flip(number(Kind::Dot, x)?, number(Kind::Dot, y)?));
    }
    if points.is_empty() || points.len() % 3 != 1 {
        return Err(garbled(
            Kind::Dot,
            format!("route of {} points", points.len()),
        ));
    }
    let mut route = vec![Segment::Line(points[0])];
    for curve in points[1..].chunks(3) {
        if let [c1, c2, end] = curve {
            route.push(Segment::Cubic {
                c1: *c1,
                c2: *c2,
                end: *end,
            });
        }
    }
    Ok(route)
}

fn rank_of(text: &str, count: usize) -> Result<usize, Diagnostic> {
    text.parse::<usize>()
        .ok()
        .filter(|rank| *rank < count)
        .ok_or_else(|| garbled(Kind::Dot, format!("object n{text}")))
}

fn member<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match &value.kind {
        J::Object(members) => members.iter().find(|m| m.key == key).map(|m| &m.value),
        _ => None,
    }
}

fn need<'a>(value: &'a Value, key: &str) -> Result<&'a Value, Diagnostic> {
    member(value, key).ok_or_else(|| garbled(Kind::Dot, format!("no {key}")))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], Diagnostic> {
    match member(value, key).map(|v| &v.kind) {
        Some(J::Array(items)) => Ok(items),
        None => Ok(&[]),
        Some(_) => Err(garbled(Kind::Dot, format!("{key} is not an array"))),
    }
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, Diagnostic> {
    text_of(need(value, key)?)
}

fn text_of(value: &Value) -> Result<&str, Diagnostic> {
    match &value.kind {
        J::Str(text) => Ok(text),
        other => Err(garbled(
            Kind::Dot,
            format!("{} where a string is expected", other.name()),
        )),
    }
}

/// `x,y` in hundredths.
fn pair(value: &Value) -> Result<(i64, i64), Diagnostic> {
    let text = text_of(value)?;
    let (x, y) = text
        .split_once(',')
        .ok_or_else(|| garbled(Kind::Dot, format!("point {text}")))?;
    Ok((number(Kind::Dot, x)?, number(Kind::Dot, y)?))
}

/// `llx,lly,urx,ury` in hundredths.
fn quad(value: &Value) -> Result<[i64; 4], Diagnostic> {
    let text = text_of(value)?;
    let parts: Vec<&str> = text.split(',').collect();
    let [a, b, c, d] = parts[..] else {
        return Err(garbled(Kind::Dot, format!("box {text}")));
    };
    Ok([
        number(Kind::Dot, a)?,
        number(Kind::Dot, b)?,
        number(Kind::Dot, c)?,
        number(Kind::Dot, d)?,
    ])
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
            name: String::from("dot"),
            version: String::from("test"),
            options: None,
        }
    }

    #[test]
    fn the_components_graph_becomes_dot_text() {
        let label = html_label(3000, 1400);
        let expected = format!(
            "digraph g {{\n  graph [compound=true, rankdir=LR]\n  node [shape=box, fixedsize=true, label=\"\"]\n  edge [arrowhead=none]\n  n0 [width=1.388889, height=0.833333]\n  subgraph cluster1 {{\n    margin=10.00\n    n2 [width=1.666667, height=0.833333]\n    n3 [width=1.111111, height=0.694444]\n  }}\n  n0:e -> n2:w [id=\"e0\", label={label}]\n  n2:e -> n3:w [id=\"e1\"]\n  n3 -> n2 [id=\"e2\", lhead=cluster1]\n}}\n"
        );
        assert_eq!(lower(&components()), expected);
    }

    #[test]
    fn sizes_go_to_dot_in_inches_and_labels_in_whole_points() {
        assert_eq!(inches(7200), "1.000000");
        assert_eq!(inches(4950), "0.687500");
        assert_eq!(inches(3500), "0.486111");
        assert_eq!(inches(1), "0.000139");
        assert!(html_label(2650, 1649).contains("WIDTH=\"27\" HEIGHT=\"16\""));
    }

    #[test]
    fn a_container_without_padding_gets_the_default_margin_and_a_direction_is_optional() {
        let text = r#"{"format": "foregram-graph", "version": 2, "name": "g", "nodes": [
            {"id": "C", "children": [{"id": "a", "width": 100, "height": 100}]}], "edges": []}"#;
        let graph = Graph::parse(text.as_bytes()).unwrap();
        let dot = lower(&graph);
        assert!(dot.starts_with("digraph g {\n  graph [compound=true]\n"));
        assert!(dot.contains(
            "subgraph cluster0 {\n    margin=8\n    n1 [width=0.013889, height=0.013889]\n  }\n"
        ));
    }

    /// A `-Tjson0` output of `dot` 16.1.0 on the text above, abridged to the
    /// keys read, flipped here: y up, frame from 0 to 126.
    const JSON0: &str = r#"{
      "name": "g", "bb": "0,0,152,126", "_subgraph_cnt": 1,
      "objects": [
        {"name": "cluster1", "bb": "88,8,144,118", "_gvid": 0, "nodes": [2,3]},
        {"_gvid": 1, "name": "n0", "pos": "50,81", "width": "1.3889", "height": "0.83333"},
        {"_gvid": 2, "name": "n2", "pos": "116,90"},
        {"_gvid": 3, "name": "n3", "pos": "116,36"}
      ],
      "edges": [
        {"_gvid": 0, "tail": 1, "head": 2, "id": "e0", "lp": "67,94.571",
         "pos": "100.163,82.609 103.529,84.237 110.234,86.741 115.662,88.375"},
        {"_gvid": 1, "tail": 2, "head": 3, "id": "e1", "pos": "116,60 116,47.871 116,58.24 116,61.401"},
        {"_gvid": 2, "tail": 3, "head": 2, "id": "e2", "pos": "88.003,61.401 80,60 80,70 88,72"}
      ]
    }"#;

    #[test]
    fn the_json0_output_becomes_a_layout_with_the_y_axis_turned() {
        let graph = components();
        let layout = read(&graph, engine(), JSON0.as_bytes()).unwrap();
        let node = |id: &str| layout.nodes.iter().find(|n| n.id == id).unwrap();
        // n0, centre (50, 81) and 100 × 60: corner (0, 126 − 81 − 30) = (0, 15).
        assert_eq!((node("Client").x, node("Client").y), (0, 1500));
        // The container: frame 88..144 × 8..118, i.e. (88, 8) and 56 × 110 once flipped.
        assert_eq!((node("System").x, node("System").y), (8800, 800));
        assert_eq!(
            node("System").size,
            Some(Size {
                width: 5600,
                height: 11000
            })
        );
        // Core, centre (116, 90) and 120 × 60, relative to the container: (56 − 60, 6 − 30)… in hundredths.
        assert_eq!(
            (node("Core").x, node("Core").y),
            (11600 - 6000 - 8800, 3600 - 3000 - 800)
        );
        // The route of e0: a start then a curve, in hundredths.
        let e0 = &layout.edges[0];
        assert_eq!(e0.route[0], Segment::Line(Point { x: 10016, y: 4339 }));
        assert!(
            matches!(e0.route[1], Segment::Cubic { end, .. } if end == Point { x: 11566, y: 3762 })
        );
        // Its label, centred at (67, 94.571): corner (67 − 15, 31.43 − 7).
        assert_eq!(e0.labels, vec![Point { x: 5200, y: 2443 }]);
        assert!(layout.edges[1].labels.is_empty());
        // The ports: `req` takes the start of e0; `in` the arrival of e0; `out` its fixed position.
        let port = |n: &str, p: &str| {
            let port = node(n).ports.iter().find(|q| q.id == p).unwrap();
            (port.x, port.y)
        };
        // `req` is on the east: the start of e0, 16 hundredths too far, comes back onto the edge.
        assert_eq!(port("Client", "req"), (10000, 4339 - 1500));
        assert_eq!(port("Core", "out"), (12000, 3000));
        // `in` is on the west of Core, whose absolute corner is (5600, 600).
        assert_eq!(port("Core", "in"), (0, 3762 - 600));
    }

    #[test]
    fn an_unused_port_sits_in_the_middle_of_its_side() {
        let text = r#"{"format": "foregram-graph", "version": 2, "name": "g", "nodes": [
            {"id": "a", "width": 1000, "height": 400, "ports": [{"id": "p", "side": "south"}, {"id": "q"}]}], "edges": []}"#;
        let graph = Graph::parse(text.as_bytes()).unwrap();
        let json = r#"{"bb": "0,0,10,4", "objects": [{"name": "n0", "pos": "5,2"}], "edges": []}"#;
        let layout = read(&graph, engine(), json.as_bytes()).unwrap();
        let ports: Vec<(i64, i64)> = layout.nodes[0].ports.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(ports, vec![(500, 400), (500, 0)]);
        assert!(layout.check(&graph).is_empty());
    }

    #[test]
    fn a_garbled_output_is_refused_with_its_detail() {
        let graph = components();
        let refusal = |json: &str| read(&graph, engine(), json.as_bytes()).unwrap_err();
        assert_eq!(refusal("not json").code, "R005");
        assert_eq!(refusal(r#"{"objects": []}"#).code, "R005");
        assert_eq!(refusal(r#"{"bb": "0,0,1", "objects": []}"#).code, "R005");
        assert_eq!(
            refusal(r#"{"bb": "0,0,1,1", "objects": [{"name": "n9", "pos": "1,1"}]}"#).code,
            "R005"
        );
        assert_eq!(
            refusal(r#"{"bb": "0,0,1,1", "objects": [{"name": "n0"}]}"#).code,
            "R005"
        );
        assert_eq!(refusal(r#"{"bb": "0,0,1,1", "objects": 3}"#).code, "R005");
        assert_eq!(
            refusal(r#"{"bb": "0,0,1,1", "objects": [], "edges": [{"id": "x"}]}"#).code,
            "R005"
        );
        let two = r#"{"bb": "0,0,1,1", "objects": [], "edges": [{"id": "e0", "pos": "1,1 2,2"}]}"#;
        assert_eq!(refusal(two).code, "R005");
        let word = r#"{"bb": "0,0,1,1", "objects": [], "edges": [{"id": "e0", "pos": "1;1"}]}"#;
        assert_eq!(refusal(word).code, "R005");
        let huge = r#"{"bb": "0,0,1,1", "objects": [{"name": "n0", "pos": "1e30,1"}]}"#;
        let huge = refusal(huge);
        assert_eq!(huge.code, "R006");
        assert_eq!(
            huge.text(),
            "dot output: number `1e30` cannot be carried in hundredths of a pixel"
        );
    }
}
