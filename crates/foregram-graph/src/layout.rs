// ForegramPM — layout: what an engine returns for a given graph.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use crate::diag::{self, Diagnostic, Pos};
use crate::graph::again;
use crate::graph::{Graph, Node, Point, Port, Rect, Size};
use crate::json::{self, Kind, Value};
use crate::msg::Message;
use crate::read::{self, COORD_MAX, FORMAT_VERSION, Obj};
use crate::read::{field, field_at};

/// Value of the `format` key of a layout.
pub const LAYOUT_FORMAT: &str = "foregram-layout";

/// The layout of a graph by an engine: the position of every node and
/// port, the size of every container, the route of every edge, the
/// position of every label.
///
/// It repeats nothing from the graph. An engine therefore cannot change
/// its dimensions or its edges; it can only forget an element or cite one
/// that does not exist, which [`Layout::check`] reports.
#[derive(Clone, Debug)]
pub struct Layout {
    /// Name of the graph that was laid out.
    pub graph: String,
    /// The engine that produced the layout.
    pub engine: Engine,
    /// Every node, whatever its depth, in a single list.
    pub nodes: Vec<NodePlace>,
    /// The route and the label positions of every edge.
    pub edges: Vec<EdgePlace>,
    /// Source position of the layout, for diagnostics.
    pub pos: Pos,
}

/// The engine that produced the layout, precisely enough to replay it.
#[derive(Clone, Debug)]
pub struct Engine {
    /// Name of the engine.
    pub name: String,
    /// Version of the engine.
    pub version: String,
    /// Options the engine was run with, if any.
    pub options: Option<String>,
}

/// The layout of one node: its position, its size if it is a container,
/// and the anchors of its ports.
#[derive(Clone, Debug)]
pub struct NodePlace {
    /// Identifier of the node.
    pub id: String,
    /// Horizontal coordinate of the top-left corner, relative to the parent
    /// — to the drawing for a top-level node —, in hundredths of a pixel.
    pub x: i64,
    /// Vertical coordinate of the same corner, in hundredths of a pixel.
    pub y: i64,
    /// Size of a container, discovered by the engine. A leaf has none.
    pub size: Option<Size>,
    /// Anchor point of each port, relative to the node's top-left corner.
    pub ports: Vec<PortPlace>,
    /// Source position of the node entry, for diagnostics.
    pub pos: Pos,
}

/// The layout of one port: its anchor point, relative to its node.
#[derive(Clone, Debug)]
pub struct PortPlace {
    /// Identifier of the port.
    pub id: String,
    /// Horizontal coordinate of the anchor, relative to the node's top-left
    /// corner, in hundredths of a pixel.
    pub x: i64,
    /// Vertical coordinate of the anchor, relative to the node's top-left
    /// corner, in hundredths of a pixel.
    pub y: i64,
    /// Source position of the port entry, for diagnostics.
    pub pos: Pos,
}

/// The layout of one edge: its route and the positions of its labels.
#[derive(Clone, Debug)]
pub struct EdgePlace {
    /// Identifier of the edge.
    pub id: String,
    /// The route, in absolute coordinates: a start point, then segments,
    /// straight or curved, up to the target node.
    pub route: Vec<Segment>,
    /// Top-left corner of each label, in graph order, in absolute
    /// coordinates.
    pub labels: Vec<Point>,
    /// Source position of the edge entry, for diagnostics.
    pub pos: Pos,
}

/// A piece of a route. The first element of a route is always a
/// [`Segment::Line`]: the start point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Segment {
    /// A straight segment to this point.
    Line(Point),
    /// A cubic Bézier curve, with its two control points.
    Cubic {
        /// First control point.
        c1: Point,
        /// Second control point.
        c2: Point,
        /// End point of the curve.
        end: Point,
    },
}

impl Segment {
    /// The point where the segment ends.
    pub fn end(&self) -> Point {
        match self {
            Segment::Line(p) => *p,
            Segment::Cubic { end, .. } => *end,
        }
    }

    /// Every point of the segment, control points included.
    pub fn points(&self) -> Vec<Point> {
        match self {
            Segment::Line(p) => vec![*p],
            Segment::Cubic { c1, c2, end } => vec![*c1, *c2, *end],
        }
    }
}

/// A graph node placed in absolute coordinates, with what is needed to
/// attach an edge to it.
struct Placed<'a> {
    node: &'a Node,
    place: &'a NodePlace,
    rect: Option<Rect>,
}

impl Layout {
    /// Reads a layout and validates it on its own, without checking it
    /// against its graph: that is the role of [`Layout::check`].
    pub fn parse(bytes: &[u8]) -> Result<Layout, Vec<Diagnostic>> {
        let root = json::parse(bytes).map_err(|d| vec![d])?;
        let mut diags = Vec::new();
        let layout = read_layout(&root, &mut diags);
        if let Some(layout) = layout
            && diags.is_empty()
        {
            diags = layout.validate();
            if diags.is_empty() {
                return Ok(layout);
            }
        }
        diag::sort(&mut diags);
        Err(diags)
    }

    /// Validates the layout on its own.
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let d = &mut diags;
        read::check_ident(&self.graph, field("graph", "in.layout"), self.pos, d);
        read::check_ident(&self.engine.name, field("name", "in.engine"), self.pos, d);
        read::check_ident(
            &self.engine.version,
            field("version", "in.engine"),
            self.pos,
            d,
        );

        let mut nodes: BTreeMap<&str, &NodePlace> = BTreeMap::new();
        let mut ports: BTreeMap<&str, &PortPlace> = BTreeMap::new();
        for (index, node) in self.nodes.iter().enumerate() {
            let who = read::item_name("node", &node.id, index);
            read::check_ident(&node.id, field_at(who.clone(), "id"), node.pos, d);
            check_coord(node.x, field_at(who.clone(), "x"), node.pos, d);
            check_coord(node.y, field_at(who.clone(), "y"), node.pos, d);
            if let Some(size) = node.size {
                let at = |key| field_at(who.clone(), key);
                read::check_range(size.width, 1, COORD_MAX, at("width"), node.pos, d);
                read::check_range(size.height, 1, COORD_MAX, at("height"), node.pos, d);
            }
            if let Some(first) = nodes.get(node.id.as_str()) {
                let message = again("LYT003", "LYT003.again", first.pos);
                d.push(Diagnostic::new(node.pos, message.text("id", &node.id)));
            } else {
                nodes.insert(&node.id, node);
            }
            for (rank, port) in node.ports.iter().enumerate() {
                let who = read::item_name("port", &port.id, rank);
                read::check_ident(&port.id, field_at(who.clone(), "id"), port.pos, d);
                check_coord(port.x, field_at(who.clone(), "x"), port.pos, d);
                check_coord(port.y, field_at(who.clone(), "y"), port.pos, d);
                if let Some(first) = ports.get(port.id.as_str()) {
                    let message = again("LYT014.twice", "LYT014.twice-again", first.pos);
                    d.push(Diagnostic::new(port.pos, message.text("id", &port.id)));
                } else {
                    ports.insert(&port.id, port);
                }
            }
        }

        let mut edges: BTreeMap<&str, &EdgePlace> = BTreeMap::new();
        for (index, edge) in self.edges.iter().enumerate() {
            let who = read::item_name("edge", &edge.id, index);
            read::check_ident(&edge.id, field_at(who.clone(), "id"), edge.pos, d);
            if let Some(first) = edges.get(edge.id.as_str()) {
                let message = again("LYT006", "LYT006.again", first.pos);
                d.push(Diagnostic::new(edge.pos, message.text("id", &edge.id)));
            } else {
                edges.insert(&edge.id, edge);
            }
            if edge.route.len() < 2 {
                d.push(Diagnostic::new(
                    edge.pos,
                    Message::new("LYT008")
                        .msg("subject", who.clone())
                        .text("count", edge.route.len()),
                ));
            }
            if let Some(Segment::Cubic { .. }) = edge.route.first() {
                d.push(Diagnostic::new(
                    edge.pos,
                    Message::new("LYT017").msg("subject", who.clone()),
                ));
            }
            let points = edge.route.iter().flat_map(Segment::points);
            for point in points.chain(edge.labels.iter().copied()) {
                let coord = |key| Message::new(key).msg("subject", who.clone());
                check_coord(point.x, coord("coord.x"), edge.pos, d);
                check_coord(point.y, coord("coord.y"), edge.pos, d);
            }
        }
        diags
    }

    /// The absolute rectangle of every placed node, containers included, by
    /// identifier. A node without a size, or whose parent is not placed, has
    /// none; a layout that passes [`Layout::check`] gives one to every node of
    /// the graph.
    pub fn absolute_rects<'a>(&self, graph: &'a Graph) -> BTreeMap<&'a str, Rect> {
        let mut rects: BTreeMap<&str, Rect> = BTreeMap::new();
        let places: BTreeMap<&str, _> = self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        for (node, parent) in graph.all_nodes() {
            let Some(place) = places.get(node.id.as_str()) else {
                continue;
            };
            let size = if node.is_container() {
                place.size
            } else {
                node.size
            };
            let origin = match parent {
                None => Some((0, 0)),
                Some(parent) => rects.get(parent.id.as_str()).map(|r| (r.x, r.y)),
            };
            if let (Some(size), Some((ox, oy))) = (size, origin) {
                rects.insert(
                    &node.id,
                    Rect {
                        x: ox + place.x,
                        y: oy + place.y,
                        width: size.width,
                        height: size.height,
                    },
                );
            }
        }
        rects
    }

    /// Checks the layout against its graph. Both must be valid. An empty list
    /// means that the layout covers the graph, all of it and nothing else, that
    /// every child fits in its parent, that every port is on the border of its
    /// node, and that every route starts and ends on the border of its nodes
    /// — or on its ports —, on the imposed side if there is one.
    pub fn check(&self, graph: &Graph) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let d = &mut diags;
        if self.graph != graph.name {
            d.push(Diagnostic::new(
                self.pos,
                Message::new("LYT001")
                    .text("layout", &self.graph)
                    .text("graph", &graph.name),
            ));
        }

        // Nodes: exact coverage, then absolute positions.
        let all = graph.all_nodes();
        let graph_nodes: BTreeMap<&str, &Node> =
            all.iter().map(|(n, _)| (n.id.as_str(), *n)).collect();
        let mut placed: BTreeMap<&str, Placed<'_>> = BTreeMap::new();
        for place in &self.nodes {
            match graph_nodes.get(place.id.as_str()) {
                Some(node) => {
                    placed.insert(
                        &place.id,
                        Placed {
                            node,
                            place,
                            rect: None,
                        },
                    );
                }
                None => d.push(Diagnostic::new(
                    place.pos,
                    Message::new("LYT002").text("id", &place.id),
                )),
            }
        }
        for (node, parent) in &all {
            let Some(mine) = placed.get(node.id.as_str()) else {
                d.push(Diagnostic::new(
                    self.pos,
                    Message::new("LYT004").text("id", &node.id),
                ));
                continue;
            };
            let place = mine.place;
            // The size: the graph's for a leaf, the layout's for a container.
            let size = match (node.is_container(), node.size, place.size) {
                (true, _, Some(size)) => Some(size),
                (true, _, None) => {
                    d.push(Diagnostic::new(
                        place.pos,
                        Message::new("LYT013.container").text("id", &node.id),
                    ));
                    None
                }
                (false, size, None) => size,
                (false, size, Some(_)) => {
                    d.push(Diagnostic::new(
                        place.pos,
                        Message::new("LYT013.leaf").text("id", &node.id),
                    ));
                    size
                }
            };
            // The absolute corner: the parent's, plus the relative position.
            let origin = match parent {
                None => Some(Point { x: 0, y: 0 }),
                Some(parent) => placed
                    .get(parent.id.as_str())
                    .and_then(|p| p.rect)
                    .map(|r| Point { x: r.x, y: r.y }),
            };
            let rect = match (origin, size) {
                (Some(origin), Some(size)) => Some(Rect {
                    x: origin.x + place.x,
                    y: origin.y + place.y,
                    width: size.width,
                    height: size.height,
                }),
                _ => None,
            };
            if let (Some(parent), Some(rect)) = (parent, rect)
                && let Some(parent_rect) = placed.get(parent.id.as_str()).and_then(|p| p.rect)
            {
                let padding = parent.padding.unwrap_or(0);
                let fits = rect.x >= parent_rect.x + padding
                    && rect.y >= parent_rect.y + padding
                    && rect.x + rect.width <= parent_rect.x + parent_rect.width - padding
                    && rect.y + rect.height <= parent_rect.y + parent_rect.height - padding;
                if !fits {
                    d.push(Diagnostic::new(
                        place.pos,
                        Message::new("LYT012")
                            .text("id", &node.id)
                            .text("parent", &parent.id),
                    ));
                }
            }
            if let Some(entry) = placed.get_mut(node.id.as_str()) {
                entry.rect = rect;
            }
            check_ports(node, place, rect, d);
        }

        // Edges.
        let routed: BTreeMap<&str, &EdgePlace> =
            self.edges.iter().map(|e| (e.id.as_str(), e)).collect();
        let graph_edges: BTreeMap<&str, _> =
            graph.edges.iter().map(|e| (e.id.as_str(), e)).collect();
        for edge in &self.edges {
            if !graph_edges.contains_key(edge.id.as_str()) {
                d.push(Diagnostic::new(
                    edge.pos,
                    Message::new("LYT005").text("id", &edge.id),
                ));
            }
        }
        for edge in &graph.edges {
            let Some(place) = routed.get(edge.id.as_str()) else {
                d.push(Diagnostic::new(
                    self.pos,
                    Message::new("LYT007").text("id", &edge.id),
                ));
                continue;
            };
            if place.labels.len() != edge.labels.len() {
                d.push(Diagnostic::new(
                    place.pos,
                    Message::new("LYT011")
                        .text("id", &edge.id)
                        .text("placed", place.labels.len())
                        .text("expected", edge.labels.len()),
                ));
            }
            let start = place.route.first().map(Segment::end);
            let finish = place.route.last().map(Segment::end);
            let ends = [
                (
                    ["LYT009.start", "LYT010.start", "LYT016.start"],
                    start,
                    &edge.source,
                    &edge.source_port,
                    edge.source_side,
                ),
                (
                    ["LYT009.end", "LYT010.end", "LYT016.end"],
                    finish,
                    &edge.target,
                    &edge.target_port,
                    edge.target_side,
                ),
            ];
            for ([off_border, off_side, off_port], point, node_id, port_id, side) in ends {
                let (Some(point), Some(at)) = (point, placed.get(node_id.as_str())) else {
                    continue;
                };
                let Some(rect) = at.rect else {
                    continue;
                };
                let end = |key| {
                    Message::new(key)
                        .text("id", &edge.id)
                        .text("x", point.x)
                        .text("y", point.y)
                };
                if let Some(port_id) = port_id {
                    // The route touches the port: its anchor point, or its box.
                    let anchor = at
                        .place
                        .ports
                        .iter()
                        .find(|p| p.id == *port_id)
                        .map(|p| Point {
                            x: rect.x + p.x,
                            y: rect.y + p.y,
                        });
                    let Some(anchor) = anchor else {
                        continue;
                    };
                    let port_size = at.node.port(port_id).and_then(|p| p.size);
                    // The box is centred on the anchor: compare the doubled
                    // offsets to the size, so that an odd size stays symmetric.
                    let touches = match port_size {
                        None => point == anchor,
                        Some(size) => {
                            (point.x - anchor.x).abs() * 2 <= size.width
                                && (point.y - anchor.y).abs() * 2 <= size.height
                        }
                    };
                    if !touches {
                        d.push(Diagnostic::new(
                            place.pos,
                            end(off_port).text("port", port_id),
                        ));
                    }
                } else if !rect.border_has(point) {
                    d.push(Diagnostic::new(
                        place.pos,
                        end(off_border).text("node", node_id),
                    ));
                } else if let Some(side) = side
                    && !rect.side_has(side, point)
                {
                    let message = end(off_side)
                        .text("side", side.as_str())
                        .text("node", node_id);
                    d.push(Diagnostic::new(place.pos, message));
                }
            }
        }
        diag::sort(&mut diags);
        diags
    }

    /// Writes the layout in its canonical form.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        out.push_str(&format!("  \"format\": {},\n", json::quote(LAYOUT_FORMAT)));
        out.push_str(&format!("  \"version\": {FORMAT_VERSION},\n"));
        out.push_str(&format!("  \"graph\": {},\n", json::quote(&self.graph)));
        out.push_str(&format!(
            "  \"engine\": {{\"name\": {}, \"version\": {}",
            json::quote(&self.engine.name),
            json::quote(&self.engine.version)
        ));
        if let Some(options) = &self.engine.options {
            out.push_str(&format!(", \"options\": {}", json::quote(options)));
        }
        out.push_str("},\n");
        let nodes: Vec<String> = self.nodes.iter().map(node_line).collect();
        json::push_list(&mut out, "nodes", &nodes, false);
        let edges: Vec<String> = self
            .edges
            .iter()
            .map(|e| {
                let mut line = format!(
                    "{{\"id\": {}, \"route\": {}",
                    json::quote(&e.id),
                    route_text(&e.route)
                );
                if !e.labels.is_empty() {
                    line.push_str(&format!(", \"labels\": {}", points_text(&e.labels)));
                }
                line.push('}');
                line
            })
            .collect();
        json::push_list(&mut out, "edges", &edges, true);
        out.push_str("}\n");
        out
    }
}

/// The ports of a placed node: exact coverage, anchor on the border, on the
/// right side, and at the imposed position if there is one.
fn check_ports(node: &Node, place: &NodePlace, rect: Option<Rect>, d: &mut Vec<Diagnostic>) {
    for placed in &place.ports {
        let Some(port) = node.port(&placed.id) else {
            d.push(Diagnostic::new(
                placed.pos,
                Message::new("LYT014.unknown")
                    .text("id", &placed.id)
                    .text("node", &node.id),
            ));
            continue;
        };
        check_port(node, port, placed, rect, d);
    }
    for port in &node.ports {
        if !place.ports.iter().any(|p| p.id == port.id) {
            d.push(Diagnostic::new(
                place.pos,
                Message::new("LYT014.missing")
                    .text("id", &port.id)
                    .text("node", &node.id),
            ));
        }
    }
}

fn check_port(
    node: &Node,
    port: &Port,
    placed: &PortPlace,
    rect: Option<Rect>,
    d: &mut Vec<Diagnostic>,
) {
    let at = Point {
        x: placed.x,
        y: placed.y,
    };
    if let Some(fixed) = port.position
        && fixed != at
    {
        d.push(Diagnostic::new(
            placed.pos,
            Message::new("LYT015.fixed")
                .text("id", &port.id)
                .text("x", at.x)
                .text("y", at.y)
                .text("fx", fixed.x)
                .text("fy", fixed.y),
        ));
        return;
    }
    let Some(rect) = rect else {
        return;
    };
    let local = Rect {
        x: 0,
        y: 0,
        width: rect.width,
        height: rect.height,
    };
    let message = |key| {
        Message::new(key)
            .text("id", &port.id)
            .text("x", at.x)
            .text("y", at.y)
            .text("node", &node.id)
    };
    if !local.border_has(at) {
        d.push(Diagnostic::new(placed.pos, message("LYT015.border")));
    } else if let Some(side) = port.side
        && !local.side_has(side, at)
    {
        d.push(Diagnostic::new(
            placed.pos,
            message("LYT015.side").text("side", side.as_str()),
        ));
    }
}

fn node_line(node: &NodePlace) -> String {
    let mut line = format!(
        "{{\"id\": {}, \"x\": {}, \"y\": {}",
        json::quote(&node.id),
        node.x,
        node.y
    );
    if let Some(size) = node.size {
        line.push_str(&format!(
            ", \"width\": {}, \"height\": {}",
            size.width, size.height
        ));
    }
    if !node.ports.is_empty() {
        let ports: Vec<String> = node
            .ports
            .iter()
            .map(|p| {
                format!(
                    "{{\"id\": {}, \"x\": {}, \"y\": {}}}",
                    json::quote(&p.id),
                    p.x,
                    p.y
                )
            })
            .collect();
        line.push_str(&format!(", \"ports\": [{}]", ports.join(", ")));
    }
    line.push('}');
    line
}

fn point_text(p: Point) -> String {
    format!("[{}, {}]", p.x, p.y)
}

fn points_text(list: &[Point]) -> String {
    let items: Vec<String> = list.iter().map(|p| point_text(*p)).collect();
    format!("[{}]", items.join(", "))
}

fn route_text(route: &[Segment]) -> String {
    let items: Vec<String> = route
        .iter()
        .map(|segment| match segment {
            Segment::Line(p) => point_text(*p),
            Segment::Cubic { c1, c2, end } => {
                format!(
                    "[{}, {}, {}]",
                    point_text(*c1),
                    point_text(*c2),
                    point_text(*end)
                )
            }
        })
        .collect();
    format!("[{}]", items.join(", "))
}

fn check_coord(value: i64, label: Message, pos: Pos, diags: &mut Vec<Diagnostic>) {
    read::check_range(value, -COORD_MAX, COORD_MAX, label, pos, diags);
}

fn read_layout(root: &Value, diags: &mut Vec<Diagnostic>) -> Option<Layout> {
    let mut obj = Obj::open(root, "in.layout", diags)?;
    read::header(&mut obj, LAYOUT_FORMAT, diags);
    let graph = obj
        .req("graph", diags)
        .and_then(|v| read::str_of(v, field("graph", "in.layout"), diags));
    let engine = obj.req("engine", diags).and_then(|v| read_engine(v, diags));
    let nodes = obj
        .req("nodes", diags)
        .and_then(|v| read::array_of(v, field("nodes", "in.layout"), diags));
    let nodes: Option<Vec<NodePlace>> =
        nodes.map(|items| items.iter().filter_map(|v| read_node(v, diags)).collect());
    let edges = obj
        .req("edges", diags)
        .and_then(|v| read::array_of(v, field("edges", "in.layout"), diags));
    let edges: Option<Vec<EdgePlace>> =
        edges.map(|items| items.iter().filter_map(|v| read_edge(v, diags)).collect());
    obj.close(diags);
    Some(Layout {
        graph: graph?.to_string(),
        engine: engine?,
        nodes: nodes?,
        edges: edges?,
        pos: root.pos,
    })
}

fn read_engine(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<Engine> {
    let mut obj = Obj::open(value, "in.engine", diags)?;
    let name = obj
        .req("name", diags)
        .and_then(|v| read::str_of(v, field("name", "in.engine"), diags));
    let version = obj
        .req("version", diags)
        .and_then(|v| read::str_of(v, field("version", "in.engine"), diags));
    let options = match obj.opt("options") {
        None => Some(None),
        Some(v) => {
            read::str_of(v, field("options", "in.engine"), diags).map(|s| Some(s.to_string()))
        }
    };
    obj.close(diags);
    Some(Engine {
        name: name?.to_string(),
        version: version?.to_string(),
        options: options?,
    })
}

fn read_node(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<NodePlace> {
    let mut obj = Obj::open(value, "in.placed-node", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, field("id", "in.placed-node"), diags));
    let x = obj
        .req("x", diags)
        .and_then(|v| read::int_of(v, field("x", "in.placed-node"), diags));
    let y = obj
        .req("y", diags)
        .and_then(|v| read::int_of(v, field("y", "in.placed-node"), diags));
    let size = match (obj.opt("width"), obj.opt("height")) {
        (None, None) => Some(None),
        (Some(w), Some(h)) => {
            let width = read::int_of(w, field("width", "in.placed-node"), diags);
            let height = read::int_of(h, field("height", "in.placed-node"), diags);
            Some(Some(Size {
                width: width?,
                height: height?,
            }))
        }
        (present, _) => {
            let missing = if present.is_some() { "height" } else { "width" };
            diags.push(Diagnostic::new(
                obj.pos,
                Message::new("SCH003")
                    .text("key", missing)
                    .msg("in", Message::new("in.placed-node")),
            ));
            None
        }
    };
    let ports = match obj.opt("ports") {
        None => Some(Vec::new()),
        Some(v) => read::array_of(v, field("ports", "in.placed-node"), diags)
            .map(|items| items.iter().filter_map(|v| read_port(v, diags)).collect()),
    };
    obj.close(diags);
    Some(NodePlace {
        id: id?.to_string(),
        x: x?,
        y: y?,
        size: size?,
        ports: ports?,
        pos: value.pos,
    })
}

fn read_port(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<PortPlace> {
    let mut obj = Obj::open(value, "in.placed-port", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, field("id", "in.placed-port"), diags));
    let x = obj
        .req("x", diags)
        .and_then(|v| read::int_of(v, field("x", "in.placed-port"), diags));
    let y = obj
        .req("y", diags)
        .and_then(|v| read::int_of(v, field("y", "in.placed-port"), diags));
    obj.close(diags);
    Some(PortPlace {
        id: id?.to_string(),
        x: x?,
        y: y?,
        pos: value.pos,
    })
}

fn read_edge(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<EdgePlace> {
    let mut obj = Obj::open(value, "in.routed-edge", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, field("id", "in.routed-edge"), diags));
    let route = obj
        .req("route", diags)
        .and_then(|v| read_route(v, field("route", "in.routed-edge"), diags));
    let labels = match obj.opt("labels") {
        None => Some(Vec::new()),
        Some(v) => read_points(v, field("labels", "in.routed-edge"), diags),
    };
    obj.close(diags);
    Some(EdgePlace {
        id: id?.to_string(),
        route: route?,
        labels: labels?,
        pos: value.pos,
    })
}

fn read_points(value: &Value, label: Message, diags: &mut Vec<Diagnostic>) -> Option<Vec<Point>> {
    let items = read::array_of(value, label.clone(), diags)?;
    let before = diags.len();
    let points: Vec<Point> = items
        .iter()
        .filter_map(|item| {
            let point = as_point(item);
            if point.is_none() {
                diags.push(Diagnostic::new(
                    item.pos,
                    Message::new("SCH001.point").msg("field", label.clone()),
                ));
            }
            point
        })
        .collect();
    (diags.len() == before).then_some(points)
}

fn read_route(value: &Value, label: Message, diags: &mut Vec<Diagnostic>) -> Option<Vec<Segment>> {
    let items = read::array_of(value, label.clone(), diags)?;
    let before = diags.len();
    let segments: Vec<Segment> = items
        .iter()
        .filter_map(|item| {
            let segment = as_segment(item);
            if segment.is_none() {
                diags.push(Diagnostic::new(
                    item.pos,
                    Message::new("SCH001.segment").msg("field", label.clone()),
                ));
            }
            segment
        })
        .collect();
    (diags.len() == before).then_some(segments)
}

fn as_point(value: &Value) -> Option<Point> {
    if let Kind::Array(items) = &value.kind
        && let [x, y] = items.as_slice()
        && let (Kind::Int(x), Kind::Int(y)) = (&x.kind, &y.kind)
    {
        return Some(Point { x: *x, y: *y });
    }
    None
}

fn as_segment(value: &Value) -> Option<Segment> {
    if let Some(point) = as_point(value) {
        return Some(Segment::Line(point));
    }
    if let Kind::Array(items) = &value.kind
        && let [c1, c2, end] = items.as_slice()
        && let (Some(c1), Some(c2), Some(end)) = (as_point(c1), as_point(c2), as_point(end))
    {
        return Some(Segment::Cubic { c1, c2, end });
    }
    None
}
