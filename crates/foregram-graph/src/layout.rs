// ForegramPM — placement : ce qu'un moteur rend pour un graphe donné.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use crate::diag::{self, Diagnostic, Pos};
use crate::graph::{Graph, Node, Side};
use crate::json::{self, Kind, Value};
use crate::read::{self, COORD_MAX, FORMAT_VERSION, Obj};

/// Valeur de la clé `format` d'un placement.
pub const LAYOUT_FORMAT: &str = "foregram-layout";

/// Un point, en centièmes de pixel ; l'axe des y pointe vers le bas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

/// Le placement d'un graphe par un moteur : la position de chaque nœud,
/// le tracé de chaque lien, la position de chaque étiquette.
///
/// Il ne répète rien du graphe. Un moteur ne peut donc pas en changer les
/// dimensions ni les liens ; il ne peut qu'oublier un élément ou en citer
/// un qui n'existe pas, ce que [`Layout::check`] relève.
#[derive(Clone, Debug)]
pub struct Layout {
    /// Nom du graphe placé.
    pub graph: String,
    pub engine: Engine,
    pub nodes: Vec<NodePlace>,
    pub edges: Vec<EdgePlace>,
    pub pos: Pos,
}

/// Le moteur qui a produit le placement, assez précisément pour le rejouer.
#[derive(Clone, Debug)]
pub struct Engine {
    pub name: String,
    pub version: String,
    pub options: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NodePlace {
    pub id: String,
    /// Coin supérieur gauche du nœud.
    pub x: i64,
    pub y: i64,
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub struct EdgePlace {
    pub id: String,
    /// Ligne brisée, du nœud de départ au nœud d'arrivée.
    pub route: Vec<Point>,
    /// Coin supérieur gauche de chaque étiquette, dans l'ordre du graphe.
    pub labels: Vec<Point>,
    pub pos: Pos,
}

impl Layout {
    /// Lit un placement et le valide pour lui-même, sans le confronter à
    /// son graphe : c'est le rôle de [`Layout::check`].
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

    /// Valide le placement pour lui-même.
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let d = &mut diags;
        read::check_ident(&self.graph, "`graph` (layout)", self.pos, d);
        read::check_ident(&self.engine.name, "`name` (engine)", self.pos, d);
        read::check_ident(&self.engine.version, "`version` (engine)", self.pos, d);

        let mut nodes: BTreeMap<&str, &NodePlace> = BTreeMap::new();
        for (index, node) in self.nodes.iter().enumerate() {
            let who = read::item_name("node", &node.id, index);
            read::check_ident(&node.id, &format!("{who}: `id`"), node.pos, d);
            check_coord(node.x, &format!("{who}: `x`"), node.pos, d);
            check_coord(node.y, &format!("{who}: `y`"), node.pos, d);
            if let Some(first) = nodes.get(node.id.as_str()) {
                d.push(Diagnostic::new(
                    "L003",
                    node.pos,
                    format!(
                        "node \"{}\" placed twice{}",
                        node.id,
                        diag::at_line(first.pos)
                    ),
                ));
            } else {
                nodes.insert(&node.id, node);
            }
        }

        let mut edges: BTreeMap<&str, &EdgePlace> = BTreeMap::new();
        for (index, edge) in self.edges.iter().enumerate() {
            let who = read::item_name("edge", &edge.id, index);
            read::check_ident(&edge.id, &format!("{who}: `id`"), edge.pos, d);
            if let Some(first) = edges.get(edge.id.as_str()) {
                d.push(Diagnostic::new(
                    "L006",
                    edge.pos,
                    format!(
                        "edge \"{}\" routed twice{}",
                        edge.id,
                        diag::at_line(first.pos)
                    ),
                ));
            } else {
                edges.insert(&edge.id, edge);
            }
            if edge.route.len() < 2 {
                d.push(Diagnostic::new(
                    "L008",
                    edge.pos,
                    format!(
                        "{who}: a route has at least two points, this one has {}",
                        edge.route.len()
                    ),
                ));
            }
            for point in edge.route.iter().chain(&edge.labels) {
                check_coord(point.x, &format!("{who}: x coordinate"), edge.pos, d);
                check_coord(point.y, &format!("{who}: y coordinate"), edge.pos, d);
            }
        }
        diags
    }

    /// Confronte le placement à son graphe. Les deux doivent être valides.
    /// Une liste vide signifie que le placement couvre le graphe, tout le
    /// graphe et rien d'autre, et que chaque tracé part et arrive sur le
    /// bord de ses nœuds, du côté imposé s'il y en a un.
    pub fn check(&self, graph: &Graph) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let d = &mut diags;
        if self.graph != graph.name {
            d.push(Diagnostic::new(
                "L001",
                self.pos,
                format!(
                    "this layout belongs to graph \"{}\", not to \"{}\"",
                    self.graph, graph.name
                ),
            ));
        }

        let graph_nodes: BTreeMap<&str, &Node> =
            graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let mut placed: BTreeMap<&str, &NodePlace> = BTreeMap::new();
        for node in &self.nodes {
            if graph_nodes.contains_key(node.id.as_str()) {
                placed.insert(&node.id, node);
            } else {
                d.push(Diagnostic::new(
                    "L002",
                    node.pos,
                    format!("node \"{}\" is not in the graph", node.id),
                ));
            }
        }
        for node in &graph.nodes {
            if !placed.contains_key(node.id.as_str()) {
                d.push(Diagnostic::new(
                    "L004",
                    self.pos,
                    format!("node \"{}\" not placed", node.id),
                ));
            }
        }

        let routed: BTreeMap<&str, &EdgePlace> =
            self.edges.iter().map(|e| (e.id.as_str(), e)).collect();
        let graph_edges: BTreeMap<&str, _> =
            graph.edges.iter().map(|e| (e.id.as_str(), e)).collect();
        for edge in &self.edges {
            if !graph_edges.contains_key(edge.id.as_str()) {
                d.push(Diagnostic::new(
                    "L005",
                    edge.pos,
                    format!("edge \"{}\" is not in the graph", edge.id),
                ));
            }
        }
        for edge in &graph.edges {
            let Some(place) = routed.get(edge.id.as_str()) else {
                d.push(Diagnostic::new(
                    "L007",
                    self.pos,
                    format!("edge \"{}\" has no route", edge.id),
                ));
                continue;
            };
            if place.labels.len() != edge.labels.len() {
                d.push(Diagnostic::new(
                    "L011",
                    place.pos,
                    format!(
                        "edge \"{}\": {} label(s) placed, the graph has {}",
                        edge.id,
                        place.labels.len(),
                        edge.labels.len()
                    ),
                ));
            }
            let ends = [
                (
                    "starts at",
                    place.route.first(),
                    &edge.source,
                    edge.source_side,
                ),
                (
                    "ends at",
                    place.route.last(),
                    &edge.target,
                    edge.target_side,
                ),
            ];
            for (verb, point, node_id, side) in ends {
                let (Some(point), Some(node), Some(at)) = (
                    point,
                    graph_nodes.get(node_id.as_str()),
                    placed.get(node_id.as_str()),
                ) else {
                    continue;
                };
                let rect = Rect {
                    x: at.x,
                    y: at.y,
                    width: node.width,
                    height: node.height,
                };
                if !rect.border_has(*point) {
                    d.push(Diagnostic::new(
                        "L009",
                        place.pos,
                        format!(
                            "edge \"{}\": the route {verb} ({}, {}), off the border of node \"{node_id}\"",
                            edge.id, point.x, point.y
                        ),
                    ));
                } else if let Some(side) = side
                    && !rect.side_has(side, *point)
                {
                    d.push(Diagnostic::new(
                        "L010",
                        place.pos,
                        format!(
                            "edge \"{}\": the route {verb} ({}, {}), off the imposed side `{}` of node \"{node_id}\"",
                            edge.id,
                            point.x,
                            point.y,
                            side.as_str()
                        ),
                    ));
                }
            }
        }
        diag::sort(&mut diags);
        diags
    }

    /// Écrit le placement sous sa forme canonique.
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
        let nodes: Vec<String> = self
            .nodes
            .iter()
            .map(|n| {
                format!(
                    "{{\"id\": {}, \"x\": {}, \"y\": {}}}",
                    json::quote(&n.id),
                    n.x,
                    n.y
                )
            })
            .collect();
        json::push_list(&mut out, "nodes", &nodes, false);
        let edges: Vec<String> = self
            .edges
            .iter()
            .map(|e| {
                let mut line = format!(
                    "{{\"id\": {}, \"route\": {}",
                    json::quote(&e.id),
                    points(&e.route)
                );
                if !e.labels.is_empty() {
                    line.push_str(&format!(", \"labels\": {}", points(&e.labels)));
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

struct Rect {
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}

impl Rect {
    /// Vrai si le point est sur le contour du rectangle, coins compris.
    fn border_has(&self, p: Point) -> bool {
        let (right, bottom) = (self.x + self.width, self.y + self.height);
        let within = p.x >= self.x && p.x <= right && p.y >= self.y && p.y <= bottom;
        within && (p.x == self.x || p.x == right || p.y == self.y || p.y == bottom)
    }

    /// Vrai si un point du contour est sur ce côté. Un coin est sur deux côtés.
    fn side_has(&self, side: Side, p: Point) -> bool {
        match side {
            Side::West => p.x == self.x,
            Side::East => p.x == self.x + self.width,
            Side::North => p.y == self.y,
            Side::South => p.y == self.y + self.height,
        }
    }
}

fn points(list: &[Point]) -> String {
    let items: Vec<String> = list.iter().map(|p| format!("[{}, {}]", p.x, p.y)).collect();
    format!("[{}]", items.join(", "))
}

fn check_coord(value: i64, label: &str, pos: Pos, diags: &mut Vec<Diagnostic>) {
    read::check_range(value, -COORD_MAX, COORD_MAX, label, pos, diags);
}

fn read_layout(root: &Value, diags: &mut Vec<Diagnostic>) -> Option<Layout> {
    let mut obj = Obj::open(root, "layout", diags)?;
    read::header(&mut obj, LAYOUT_FORMAT, diags);
    let graph = obj
        .req("graph", diags)
        .and_then(|v| read::str_of(v, "`graph` (layout)", diags));
    let engine = obj.req("engine", diags).and_then(|v| read_engine(v, diags));
    let nodes = obj
        .req("nodes", diags)
        .and_then(|v| read::array_of(v, "`nodes` (layout)", diags));
    let nodes: Option<Vec<NodePlace>> =
        nodes.map(|items| items.iter().filter_map(|v| read_node(v, diags)).collect());
    let edges = obj
        .req("edges", diags)
        .and_then(|v| read::array_of(v, "`edges` (layout)", diags));
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
    let mut obj = Obj::open(value, "engine", diags)?;
    let name = obj
        .req("name", diags)
        .and_then(|v| read::str_of(v, "`name` (engine)", diags));
    let version = obj
        .req("version", diags)
        .and_then(|v| read::str_of(v, "`version` (engine)", diags));
    let options = match obj.opt("options") {
        None => Some(None),
        Some(v) => read::str_of(v, "`options` (engine)", diags).map(|s| Some(s.to_string())),
    };
    obj.close(diags);
    Some(Engine {
        name: name?.to_string(),
        version: version?.to_string(),
        options: options?,
    })
}

fn read_node(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<NodePlace> {
    let mut obj = Obj::open(value, "placed node", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, "`id` (placed node)", diags));
    let x = obj
        .req("x", diags)
        .and_then(|v| read::int_of(v, "`x` (placed node)", diags));
    let y = obj
        .req("y", diags)
        .and_then(|v| read::int_of(v, "`y` (placed node)", diags));
    obj.close(diags);
    Some(NodePlace {
        id: id?.to_string(),
        x: x?,
        y: y?,
        pos: value.pos,
    })
}

fn read_edge(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<EdgePlace> {
    let mut obj = Obj::open(value, "routed edge", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, "`id` (routed edge)", diags));
    let route = obj
        .req("route", diags)
        .and_then(|v| read_points(v, "`route` (routed edge)", diags));
    let labels = match obj.opt("labels") {
        None => Some(Vec::new()),
        Some(v) => read_points(v, "`labels` (routed edge)", diags),
    };
    obj.close(diags);
    Some(EdgePlace {
        id: id?.to_string(),
        route: route?,
        labels: labels?,
        pos: value.pos,
    })
}

fn read_points(value: &Value, label: &str, diags: &mut Vec<Diagnostic>) -> Option<Vec<Point>> {
    let items = read::array_of(value, label, diags)?;
    let before = diags.len();
    let points: Vec<Point> = items
        .iter()
        .filter_map(|item| read_point(item, label, diags))
        .collect();
    (diags.len() == before).then_some(points)
}

fn read_point(value: &Value, label: &str, diags: &mut Vec<Diagnostic>) -> Option<Point> {
    if let Kind::Array(items) = &value.kind
        && let [x, y] = items.as_slice()
        && let (Kind::Int(x), Kind::Int(y)) = (&x.kind, &y.kind)
    {
        return Some(Point { x: *x, y: *y });
    }
    diags.push(Diagnostic::new(
        "S001",
        value.pos,
        format!("{label}: a point is written [x, y], two integers"),
    ));
    None
}
