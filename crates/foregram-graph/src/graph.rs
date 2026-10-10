// ForegramPM — graphe abstrait : ce qu'un moteur de placement reçoit.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use crate::diag::{self, Diagnostic, Pos};
use crate::json::{self, Value};
use crate::msg::Message;
use crate::read::{self, COORD_MAX, FORMAT_VERSION, LAYER_MAX, Obj};
use crate::read::{field, field_at};

/// Valeur de la clé `format` d'un graphe.
pub const GRAPH_FORMAT: &str = "foregram-graph";

/// Un point, en centièmes de pixel ; l'axe des y pointe vers le bas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

/// Une largeur et une hauteur, en centièmes de pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: i64,
    pub height: i64,
}

/// Un rectangle par son coin supérieur gauche et sa taille.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl Rect {
    /// Vrai si le point est sur le contour du rectangle, coins compris.
    pub fn border_has(&self, p: Point) -> bool {
        let (right, bottom) = (self.x + self.width, self.y + self.height);
        let within = p.x >= self.x && p.x <= right && p.y >= self.y && p.y <= bottom;
        within && (p.x == self.x || p.x == right || p.y == self.y || p.y == bottom)
    }

    /// Vrai si un point du contour est sur ce côté. Un coin est sur deux côtés.
    pub fn side_has(&self, side: Side, p: Point) -> bool {
        match side {
            Side::West => p.x == self.x,
            Side::East => p.x == self.x + self.width,
            Side::North => p.y == self.y,
            Side::South => p.y == self.y + self.height,
        }
    }
}

/// Sens dans lequel les couches se succèdent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Right,
    Down,
    Left,
    Up,
}

impl Direction {
    pub(crate) const TABLE: [(&'static str, Direction); 4] = [
        ("right", Direction::Right),
        ("down", Direction::Down),
        ("left", Direction::Left),
        ("up", Direction::Up),
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Right => "right",
            Direction::Down => "down",
            Direction::Left => "left",
            Direction::Up => "up",
        }
    }
}

/// Côté d'un nœud, l'axe des y pointant vers le bas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    North,
    South,
    East,
    West,
}

impl Side {
    pub(crate) const TABLE: [(&'static str, Side); 4] = [
        ("north", Side::North),
        ("south", Side::South),
        ("east", Side::East),
        ("west", Side::West),
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Side::North => "north",
            Side::South => "south",
            Side::East => "east",
            Side::West => "west",
        }
    }
}

/// Endroit du lien auquel une étiquette se rapporte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Source,
    Center,
    Target,
}

impl Anchor {
    pub(crate) const TABLE: [(&'static str, Anchor); 3] = [
        ("source", Anchor::Source),
        ("center", Anchor::Center),
        ("target", Anchor::Target),
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Anchor::Source => "source",
            Anchor::Center => "center",
            Anchor::Target => "target",
        }
    }
}

/// Un graphe à placer : des nœuds rectangulaires, qui peuvent en contenir
/// d'autres, et des liens entre eux.
///
/// Les boucles (un lien d'un nœud à lui-même) et les liens multiples entre
/// deux mêmes nœuds sont admis, de même qu'un lien entre un nœud et l'un de
/// ses descendants. Les identifiants de nœuds et ceux de ports sont uniques
/// dans tout le graphe. Les longueurs sont en centièmes de pixel.
#[derive(Clone, Debug)]
pub struct Graph {
    pub name: String,
    pub direction: Option<Direction>,
    /// Les nœuds de premier niveau ; leurs enfants sont dans [`Node::children`].
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Position dans la source, [`Pos::NONE`] pour un graphe construit en mémoire.
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: String,
    /// Taille d'une feuille. Un nœud qui a des enfants n'en a pas : c'est le
    /// placement qui la donne.
    pub size: Option<Size>,
    /// Couche imposée, comptée de 0 dans le sens de `direction`.
    pub layer: Option<i64>,
    /// Rang imposé dans la couche ; n'a de sens qu'avec `layer`.
    pub order: Option<i64>,
    /// Espace à réserver entre le bord d'un conteneur et ses enfants.
    pub padding: Option<i64>,
    /// Vrai si, sur chaque côté, les ports se succèdent dans l'ordre de la liste.
    pub ports_ordered: bool,
    pub ports: Vec<Port>,
    pub children: Vec<Node>,
    pub pos: Pos,
}

/// Un point d'attache nommé sur le bord d'un nœud.
#[derive(Clone, Debug)]
pub struct Port {
    pub id: String,
    /// Côté imposé.
    pub side: Option<Side>,
    /// Point d'ancrage imposé, relatif au coin supérieur gauche du nœud ;
    /// exige un côté, et se trouve sur le bord de ce côté.
    pub position: Option<Point>,
    /// Boîte du port, centrée sur son point d'ancrage. Sans elle le port
    /// est un point.
    pub size: Option<Size>,
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    /// Port du nœud de départ, exclusif de `source_side`.
    pub source_port: Option<String>,
    pub target_port: Option<String>,
    /// Côté imposé du nœud de départ, pour un lien sans port.
    pub source_side: Option<Side>,
    /// Côté imposé du nœud d'arrivée, pour un lien sans port.
    pub target_side: Option<Side>,
    pub labels: Vec<Label>,
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub struct Label {
    pub width: i64,
    pub height: i64,
    pub at: Anchor,
}

impl Node {
    /// Vrai si le nœud contient d'autres nœuds.
    pub fn is_container(&self) -> bool {
        !self.children.is_empty()
    }

    /// Le port qui porte cet identifiant.
    pub fn port(&self, id: &str) -> Option<&Port> {
        self.ports.iter().find(|port| port.id == id)
    }
}

impl Graph {
    /// Lit un graphe et le valide. Rend le graphe, ou tous les refus relevés.
    ///
    /// Un défaut de syntaxe JSON arrête la lecture au premier ; les défauts
    /// de structure sont tous rendus ; les défauts de sens ne sont cherchés
    /// que dans un graphe à la structure juste.
    pub fn parse(bytes: &[u8]) -> Result<Graph, Vec<Diagnostic>> {
        let root = json::parse(bytes).map_err(|d| vec![d])?;
        let mut diags = Vec::new();
        let graph = read_graph(&root, &mut diags);
        if let Some(graph) = graph
            && diags.is_empty()
        {
            diags = graph.validate();
            if diags.is_empty() {
                return Ok(graph);
            }
        }
        diag::sort(&mut diags);
        Err(diags)
    }

    /// Tous les nœuds, profondeur d'abord, chacun avec son parent.
    pub fn all_nodes(&self) -> Vec<(&Node, Option<&Node>)> {
        fn walk<'a>(
            nodes: &'a [Node],
            parent: Option<&'a Node>,
            out: &mut Vec<(&'a Node, Option<&'a Node>)>,
        ) {
            for node in nodes {
                out.push((node, parent));
                walk(&node.children, Some(node), out);
            }
        }
        let mut out = Vec::new();
        walk(&self.nodes, None, &mut out);
        out
    }

    /// Le nœud qui porte cet identifiant, à quelque profondeur qu'il soit.
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.all_nodes()
            .into_iter()
            .map(|(node, _)| node)
            .find(|node| node.id == id)
    }

    /// Le nœud qui porte ce port, avec le port.
    pub fn port(&self, id: &str) -> Option<(&Node, &Port)> {
        self.all_nodes()
            .into_iter()
            .find_map(|(node, _)| node.port(id).map(|port| (node, port)))
    }

    /// Valide un graphe, qu'il vienne d'un texte ou qu'il ait été construit
    /// en mémoire. Une liste vide signifie que le graphe est valide.
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        read::check_ident(&self.name, field("name", "in.graph"), self.pos, &mut diags);

        let mut nodes: BTreeMap<&str, &Node> = BTreeMap::new();
        let mut ports: BTreeMap<&str, (&Node, &Port)> = BTreeMap::new();
        let mut slots: BTreeMap<(i64, i64), &Node> = BTreeMap::new();
        for (index, (node, _)) in self.all_nodes().into_iter().enumerate() {
            let who = read::item_name("node", &node.id, index);
            let d = &mut diags;
            let at = |key| field_at(who.clone(), key);
            read::check_ident(&node.id, at("id"), node.pos, d);
            match (node.is_container(), node.size) {
                (false, None) => d.push(Diagnostic::new(
                    node.pos,
                    Message::new("G011.leaf").msg("subject", who.clone()),
                )),
                (true, Some(_)) => d.push(Diagnostic::new(
                    node.pos,
                    Message::new("G011.container").msg("subject", who.clone()),
                )),
                (_, Some(size)) => {
                    read::check_range(size.width, 1, COORD_MAX, at("width"), node.pos, d);
                    read::check_range(size.height, 1, COORD_MAX, at("height"), node.pos, d);
                }
                (true, None) => {}
            }
            if let Some(padding) = node.padding {
                read::check_range(padding, 0, COORD_MAX, at("padding"), node.pos, d);
                if !node.is_container() {
                    d.push(Diagnostic::new(
                        node.pos,
                        Message::new("G012").msg("subject", who.clone()),
                    ));
                }
            }
            if let Some(layer) = node.layer {
                read::check_range(layer, 0, LAYER_MAX, at("layer"), node.pos, d);
            }
            if let Some(order) = node.order {
                read::check_range(order, 0, LAYER_MAX, at("order"), node.pos, d);
                if node.layer.is_none() {
                    d.push(Diagnostic::new(
                        node.pos,
                        Message::new("G004").msg("subject", who.clone()),
                    ));
                }
            }
            if let Some(first) = nodes.get(node.id.as_str()) {
                let message = again("G001", "G001.again", first.pos);
                d.push(Diagnostic::new(node.pos, message.text("id", &node.id)));
            } else {
                nodes.insert(&node.id, node);
            }
            if let (Some(layer), Some(order)) = (node.layer, node.order) {
                if let Some(first) = slots.get(&(layer, order)) {
                    d.push(Diagnostic::new(
                        node.pos,
                        Message::new("G005")
                            .msg("subject", who.clone())
                            .text("other", &first.id),
                    ));
                } else {
                    slots.insert((layer, order), node);
                }
            }
            for (rank, port) in node.ports.iter().enumerate() {
                let who = read::item_name("port", &port.id, rank);
                let at = |key| field_at(who.clone(), key);
                read::check_ident(&port.id, at("id"), port.pos, d);
                if let Some(first) = ports.get(port.id.as_str()) {
                    let message = again("G006", "G006.again", first.1.pos);
                    d.push(Diagnostic::new(port.pos, message.text("id", &port.id)));
                } else {
                    ports.insert(&port.id, (node, port));
                }
                if let Some(size) = port.size {
                    read::check_range(size.width, 1, COORD_MAX, at("width"), port.pos, d);
                    read::check_range(size.height, 1, COORD_MAX, at("height"), port.pos, d);
                }
                if node.ports_ordered && port.side.is_none() {
                    d.push(Diagnostic::new(
                        port.pos,
                        Message::new("G009")
                            .msg("subject", read::item_name("node", &node.id, index))
                            .text("port", &port.id),
                    ));
                }
                if let Some(position) = port.position {
                    match (port.side, node.size) {
                        (None, _) => d.push(Diagnostic::new(
                            port.pos,
                            Message::new("G010.side").msg("subject", who.clone()),
                        )),
                        (Some(_), None) => d.push(Diagnostic::new(
                            port.pos,
                            Message::new("G010.container").msg("subject", who.clone()),
                        )),
                        (Some(side), Some(size)) => {
                            let rect = Rect {
                                x: 0,
                                y: 0,
                                width: size.width,
                                height: size.height,
                            };
                            if !(rect.border_has(position) && rect.side_has(side, position)) {
                                d.push(Diagnostic::new(
                                    port.pos,
                                    Message::new("G010.border")
                                        .msg("subject", who.clone())
                                        .text("x", position.x)
                                        .text("y", position.y)
                                        .text("side", side.as_str()),
                                ));
                            }
                        }
                    }
                }
            }
        }

        let mut edges: BTreeMap<&str, &Edge> = BTreeMap::new();
        for (index, edge) in self.edges.iter().enumerate() {
            let who = read::item_name("edge", &edge.id, index);
            let d = &mut diags;
            read::check_ident(&edge.id, field_at(who.clone(), "id"), edge.pos, d);
            if let Some(first) = edges.get(edge.id.as_str()) {
                let message = again("G002", "G002.again", first.pos);
                d.push(Diagnostic::new(edge.pos, message.text("id", &edge.id)));
            } else {
                edges.insert(&edge.id, edge);
            }
            let ends = [
                (
                    "source",
                    &edge.source,
                    &edge.source_port,
                    edge.source_side,
                    "source_port",
                    "source_side",
                ),
                (
                    "target",
                    &edge.target,
                    &edge.target_port,
                    edge.target_side,
                    "target_port",
                    "target_side",
                ),
            ];
            for (key, end, port, side, port_key, side_key) in ends {
                if !nodes.contains_key(end.as_str()) {
                    d.push(Diagnostic::new(
                        edge.pos,
                        Message::new("G003")
                            .msg("subject", who.clone())
                            .text("key", key)
                            .text("node", end),
                    ));
                }
                let Some(port) = port else {
                    continue;
                };
                if side.is_some() {
                    d.push(Diagnostic::new(
                        edge.pos,
                        Message::new("G008")
                            .msg("subject", who.clone())
                            .text("side", side_key)
                            .text("port", port_key),
                    ));
                }
                match ports.get(port.as_str()) {
                    None => d.push(Diagnostic::new(
                        edge.pos,
                        Message::new("G007.unknown")
                            .msg("subject", who.clone())
                            .text("key", port_key)
                            .text("port", port),
                    )),
                    Some((owner, _)) if owner.id != *end => d.push(Diagnostic::new(
                        edge.pos,
                        Message::new("G007.other")
                            .msg("subject", who.clone())
                            .text("key", port_key)
                            .text("port", port)
                            .text("owner", &owner.id)
                            .text("node", end),
                    )),
                    Some(_) => {}
                }
            }
            for (rank, label) in edge.labels.iter().enumerate() {
                let what = Message::new("subject.edge-label")
                    .msg("edge", who.clone())
                    .text("rank", rank + 1);
                let at = |key| field_at(what.clone(), key);
                read::check_range(label.width, 1, COORD_MAX, at("width"), edge.pos, d);
                read::check_range(label.height, 1, COORD_MAX, at("height"), edge.pos, d);
            }
        }
        diags
    }

    /// Écrit le graphe sous sa forme canonique : les clés dans un ordre fixe,
    /// un nœud et un lien par ligne, les enfants d'un conteneur sur les
    /// lignes suivantes avec un retrait de plus, les clés facultatives omises
    /// quand elles n'ont pas de valeur. Deux graphes égaux donnent les mêmes
    /// octets.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        out.push_str(&format!("  \"format\": {},\n", json::quote(GRAPH_FORMAT)));
        out.push_str(&format!("  \"version\": {FORMAT_VERSION},\n"));
        out.push_str(&format!("  \"name\": {},\n", json::quote(&self.name)));
        if let Some(direction) = self.direction {
            out.push_str(&format!(
                "  \"direction\": {},\n",
                json::quote(direction.as_str())
            ));
        }
        let nodes: Vec<String> = self.nodes.iter().map(|n| node_text(n, 2)).collect();
        json::push_list(&mut out, "nodes", &nodes, false);
        let edges: Vec<String> = self.edges.iter().map(edge_line).collect();
        json::push_list(&mut out, "edges", &edges, true);
        out.push_str("}\n");
        out
    }
}

/// Un nœud en forme canonique, `depth` étant son niveau de retrait (2 au
/// premier niveau). Ses enfants suivent sur leurs propres lignes.
fn node_text(node: &Node, depth: usize) -> String {
    let mut line = format!("{{\"id\": {}", json::quote(&node.id));
    if let Some(size) = node.size {
        line.push_str(&format!(
            ", \"width\": {}, \"height\": {}",
            size.width, size.height
        ));
    }
    if let Some(layer) = node.layer {
        line.push_str(&format!(", \"layer\": {layer}"));
    }
    if let Some(order) = node.order {
        line.push_str(&format!(", \"order\": {order}"));
    }
    if let Some(padding) = node.padding {
        line.push_str(&format!(", \"padding\": {padding}"));
    }
    if node.ports_ordered {
        line.push_str(", \"ports_ordered\": true");
    }
    if !node.ports.is_empty() {
        let ports: Vec<String> = node.ports.iter().map(port_text).collect();
        line.push_str(&format!(", \"ports\": [{}]", ports.join(", ")));
    }
    if node.is_container() {
        let pad = "  ".repeat(depth);
        line.push_str(", \"children\": [\n");
        for (i, child) in node.children.iter().enumerate() {
            line.push_str(&pad);
            line.push_str("  ");
            line.push_str(&node_text(child, depth + 1));
            if i + 1 < node.children.len() {
                line.push(',');
            }
            line.push('\n');
        }
        line.push_str(&pad);
        line.push(']');
    }
    line.push('}');
    line
}

fn port_text(port: &Port) -> String {
    let mut text = format!("{{\"id\": {}", json::quote(&port.id));
    if let Some(side) = port.side {
        text.push_str(&format!(", \"side\": {}", json::quote(side.as_str())));
    }
    if let Some(position) = port.position {
        text.push_str(&format!(", \"x\": {}, \"y\": {}", position.x, position.y));
    }
    if let Some(size) = port.size {
        text.push_str(&format!(
            ", \"width\": {}, \"height\": {}",
            size.width, size.height
        ));
    }
    text.push('}');
    text
}

fn edge_line(edge: &Edge) -> String {
    let mut line = format!(
        "{{\"id\": {}, \"source\": {}, \"target\": {}",
        json::quote(&edge.id),
        json::quote(&edge.source),
        json::quote(&edge.target)
    );
    if let Some(port) = &edge.source_port {
        line.push_str(&format!(", \"source_port\": {}", json::quote(port)));
    }
    if let Some(port) = &edge.target_port {
        line.push_str(&format!(", \"target_port\": {}", json::quote(port)));
    }
    if let Some(side) = edge.source_side {
        line.push_str(&format!(
            ", \"source_side\": {}",
            json::quote(side.as_str())
        ));
    }
    if let Some(side) = edge.target_side {
        line.push_str(&format!(
            ", \"target_side\": {}",
            json::quote(side.as_str())
        ));
    }
    if !edge.labels.is_empty() {
        let labels: Vec<String> = edge
            .labels
            .iter()
            .map(|label| {
                format!(
                    "{{\"width\": {}, \"height\": {}, \"at\": {}}}",
                    label.width,
                    label.height,
                    json::quote(label.at.as_str())
                )
            })
            .collect();
        line.push_str(&format!(", \"labels\": [{}]", labels.join(", ")));
    }
    line.push('}');
    line
}

fn read_graph(root: &Value, diags: &mut Vec<Diagnostic>) -> Option<Graph> {
    let mut obj = Obj::open(root, "in.graph", diags)?;
    read::header(&mut obj, GRAPH_FORMAT, diags);
    let name = obj
        .req("name", diags)
        .and_then(|v| read::str_of(v, field("name", "in.graph"), diags));
    let direction = obj
        .opt("direction")
        .and_then(|v| read::choice_of(v, field("direction", "in.graph"), &Direction::TABLE, diags));
    let nodes = obj
        .req("nodes", diags)
        .and_then(|v| read::array_of(v, field("nodes", "in.graph"), diags));
    let nodes: Option<Vec<Node>> =
        nodes.map(|items| items.iter().filter_map(|v| read_node(v, diags)).collect());
    let edges = obj
        .req("edges", diags)
        .and_then(|v| read::array_of(v, field("edges", "in.graph"), diags));
    let edges: Option<Vec<Edge>> =
        edges.map(|items| items.iter().filter_map(|v| read_edge(v, diags)).collect());
    obj.close(diags);
    Some(Graph {
        name: name?.to_string(),
        direction,
        nodes: nodes?,
        edges: edges?,
        pos: root.pos,
    })
}

/// Lit `width` et `height`, facultatives ensemble : l'une sans l'autre est
/// un refus.
fn read_size(
    obj: &mut Obj<'_>,
    within: &'static str,
    diags: &mut Vec<Diagnostic>,
) -> Option<Option<Size>> {
    let width = obj.opt("width");
    let height = obj.opt("height");
    match (width, height) {
        (None, None) => Some(None),
        (Some(w), Some(h)) => {
            let width = read::int_of(w, field("width", within), diags);
            let height = read::int_of(h, field("height", within), diags);
            Some(Some(Size {
                width: width?,
                height: height?,
            }))
        }
        (Some(_), None) => {
            diags.push(Diagnostic::new(
                obj.pos,
                Message::new("S003")
                    .text("key", "height")
                    .msg("in", Message::new(within)),
            ));
            None
        }
        (None, Some(_)) => {
            diags.push(Diagnostic::new(
                obj.pos,
                Message::new("S003")
                    .text("key", "width")
                    .msg("in", Message::new(within)),
            ));
            None
        }
    }
}

fn read_node(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<Node> {
    let mut obj = Obj::open(value, "in.node", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, field("id", "in.node"), diags));
    let size = read_size(&mut obj, "in.node", diags);
    let layer = obj
        .opt("layer")
        .and_then(|v| read::int_of(v, field("layer", "in.node"), diags));
    let order = obj
        .opt("order")
        .and_then(|v| read::int_of(v, field("order", "in.node"), diags));
    let padding = obj
        .opt("padding")
        .and_then(|v| read::int_of(v, field("padding", "in.node"), diags));
    let ports_ordered = match obj.opt("ports_ordered") {
        None => Some(false),
        Some(v) => read::bool_of(v, field("ports_ordered", "in.node"), diags),
    };
    let ports = match obj.opt("ports") {
        None => Some(Vec::new()),
        Some(v) => read::array_of(v, field("ports", "in.node"), diags)
            .map(|items| items.iter().filter_map(|v| read_port(v, diags)).collect()),
    };
    let children = match obj.opt("children") {
        None => Some(Vec::new()),
        Some(v) => read::array_of(v, field("children", "in.node"), diags)
            .map(|items| items.iter().filter_map(|v| read_node(v, diags)).collect()),
    };
    obj.close(diags);
    Some(Node {
        id: id?.to_string(),
        size: size?,
        layer,
        order,
        padding,
        ports_ordered: ports_ordered?,
        ports: ports?,
        children: children?,
        pos: value.pos,
    })
}

fn read_port(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<Port> {
    let mut obj = Obj::open(value, "in.port", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, field("id", "in.port"), diags));
    let side = obj
        .opt("side")
        .and_then(|v| read::choice_of(v, field("side", "in.port"), &Side::TABLE, diags));
    let position = match (obj.opt("x"), obj.opt("y")) {
        (None, None) => Some(None),
        (Some(x), Some(y)) => {
            let x = read::int_of(x, field("x", "in.port"), diags);
            let y = read::int_of(y, field("y", "in.port"), diags);
            Some(Some(Point { x: x?, y: y? }))
        }
        (present, _) => {
            let missing = if present.is_some() { "y" } else { "x" };
            diags.push(Diagnostic::new(
                obj.pos,
                Message::new("S003")
                    .text("key", missing)
                    .msg("in", Message::new("in.port")),
            ));
            None
        }
    };
    let size = read_size(&mut obj, "in.port", diags);
    obj.close(diags);
    Some(Port {
        id: id?.to_string(),
        side,
        position: position?,
        size: size?,
        pos: value.pos,
    })
}

fn read_edge(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<Edge> {
    let mut obj = Obj::open(value, "in.edge", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, field("id", "in.edge"), diags));
    let source = obj
        .req("source", diags)
        .and_then(|v| read::str_of(v, field("source", "in.edge"), diags));
    let target = obj
        .req("target", diags)
        .and_then(|v| read::str_of(v, field("target", "in.edge"), diags));
    let source_port = match obj.opt("source_port") {
        None => Some(None),
        Some(v) => {
            read::str_of(v, field("source_port", "in.edge"), diags).map(|s| Some(s.to_string()))
        }
    };
    let target_port = match obj.opt("target_port") {
        None => Some(None),
        Some(v) => {
            read::str_of(v, field("target_port", "in.edge"), diags).map(|s| Some(s.to_string()))
        }
    };
    let source_side = obj
        .opt("source_side")
        .and_then(|v| read::choice_of(v, field("source_side", "in.edge"), &Side::TABLE, diags));
    let target_side = obj
        .opt("target_side")
        .and_then(|v| read::choice_of(v, field("target_side", "in.edge"), &Side::TABLE, diags));
    let labels = match obj.opt("labels") {
        None => Some(Vec::new()),
        Some(v) => read::array_of(v, field("labels", "in.edge"), diags)
            .map(|items| items.iter().filter_map(|v| read_label(v, diags)).collect()),
    };
    obj.close(diags);
    Some(Edge {
        id: id?.to_string(),
        source: source?.to_string(),
        target: target?.to_string(),
        source_port: source_port?,
        target_port: target_port?,
        source_side,
        target_side,
        labels: labels?,
        pos: value.pos,
    })
}

fn read_label(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<Label> {
    let mut obj = Obj::open(value, "in.label", diags)?;
    let width = obj
        .req("width", diags)
        .and_then(|v| read::int_of(v, field("width", "in.label"), diags));
    let height = obj
        .req("height", diags)
        .and_then(|v| read::int_of(v, field("height", "in.label"), diags));
    let at = obj
        .req("at", diags)
        .and_then(|v| read::choice_of(v, field("at", "in.label"), &Anchor::TABLE, diags));
    obj.close(diags);
    Some(Label {
        width: width?,
        height: height?,
        at: at?,
    })
}

/// Le message d'un doublon, avec la ligne du premier quand elle est connue.
pub(crate) fn again(plain: &'static str, with_line: &'static str, first: Pos) -> Message {
    if first.is_none() {
        Message::new(plain)
    } else {
        Message::new(with_line).text("line", first.line)
    }
}
