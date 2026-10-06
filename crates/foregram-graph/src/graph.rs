// ForegramPM — graphe abstrait : ce qu'un moteur de placement reçoit.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use crate::diag::{self, Diagnostic, Pos};
use crate::json::{self, Value};
use crate::read::{self, COORD_MAX, FORMAT_VERSION, LAYER_MAX, Obj};

/// Valeur de la clé `format` d'un graphe.
pub const GRAPH_FORMAT: &str = "foregram-graph";

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

/// Un graphe à placer : des nœuds rectangulaires et des liens entre eux.
///
/// Les boucles (un lien d'un nœud à lui-même) et les liens multiples entre
/// deux mêmes nœuds sont admis. Les longueurs sont en centièmes de pixel.
#[derive(Clone, Debug)]
pub struct Graph {
    pub name: String,
    pub direction: Option<Direction>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Position dans la source, [`Pos::NONE`] pour un graphe construit en mémoire.
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: String,
    pub width: i64,
    pub height: i64,
    /// Couche imposée, comptée de 0 dans le sens de `direction`.
    pub layer: Option<i64>,
    /// Rang imposé dans la couche ; n'a de sens qu'avec `layer`.
    pub order: Option<i64>,
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    /// Côté imposé du nœud de départ.
    pub source_side: Option<Side>,
    /// Côté imposé du nœud d'arrivée.
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

    /// Valide un graphe, qu'il vienne d'un texte ou qu'il ait été construit
    /// en mémoire. Une liste vide signifie que le graphe est valide.
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        read::check_ident(&self.name, "`name` (graph)", self.pos, &mut diags);

        let mut nodes: BTreeMap<&str, &Node> = BTreeMap::new();
        let mut slots: BTreeMap<(i64, i64), &Node> = BTreeMap::new();
        for (index, node) in self.nodes.iter().enumerate() {
            let who = read::item_name("node", &node.id, index);
            let d = &mut diags;
            read::check_ident(&node.id, &format!("{who}: `id`"), node.pos, d);
            let label = format!("{who}: `width`");
            read::check_range(node.width, 1, COORD_MAX, &label, node.pos, d);
            let label = format!("{who}: `height`");
            read::check_range(node.height, 1, COORD_MAX, &label, node.pos, d);
            if let Some(layer) = node.layer {
                let label = format!("{who}: `layer`");
                read::check_range(layer, 0, LAYER_MAX, &label, node.pos, d);
            }
            if let Some(order) = node.order {
                let label = format!("{who}: `order`");
                read::check_range(order, 0, LAYER_MAX, &label, node.pos, d);
                if node.layer.is_none() {
                    d.push(Diagnostic::new(
                        "G004",
                        node.pos,
                        format!("{who}: `order` without `layer`"),
                    ));
                }
            }
            if let Some(first) = nodes.get(node.id.as_str()) {
                d.push(Diagnostic::new(
                    "G001",
                    node.pos,
                    format!(
                        "node identifier \"{}\" already used{}",
                        node.id,
                        diag::at_line(first.pos)
                    ),
                ));
            } else {
                nodes.insert(&node.id, node);
            }
            if let (Some(layer), Some(order)) = (node.layer, node.order) {
                if let Some(first) = slots.get(&(layer, order)) {
                    d.push(Diagnostic::new(
                        "G005",
                        node.pos,
                        format!(
                            "{who}: same `layer` and same `order` as node \"{}\"",
                            first.id
                        ),
                    ));
                } else {
                    slots.insert((layer, order), node);
                }
            }
        }

        let mut edges: BTreeMap<&str, &Edge> = BTreeMap::new();
        for (index, edge) in self.edges.iter().enumerate() {
            let who = read::item_name("edge", &edge.id, index);
            let d = &mut diags;
            read::check_ident(&edge.id, &format!("{who}: `id`"), edge.pos, d);
            if let Some(first) = edges.get(edge.id.as_str()) {
                d.push(Diagnostic::new(
                    "G002",
                    edge.pos,
                    format!(
                        "edge identifier \"{}\" already used{}",
                        edge.id,
                        diag::at_line(first.pos)
                    ),
                ));
            } else {
                edges.insert(&edge.id, edge);
            }
            for (key, end) in [("source", &edge.source), ("target", &edge.target)] {
                if !nodes.contains_key(end.as_str()) {
                    d.push(Diagnostic::new(
                        "G003",
                        edge.pos,
                        format!("{who}: `{key}` refers to node \"{end}\", which does not exist"),
                    ));
                }
            }
            for (rank, label) in edge.labels.iter().enumerate() {
                let what = format!("{who}, label #{}", rank + 1);
                let text = format!("{what}: `width`");
                read::check_range(label.width, 1, COORD_MAX, &text, edge.pos, d);
                let text = format!("{what}: `height`");
                read::check_range(label.height, 1, COORD_MAX, &text, edge.pos, d);
            }
        }
        diags
    }

    /// Le nœud qui porte cet identifiant.
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// Écrit le graphe sous sa forme canonique : les clés dans un ordre fixe,
    /// un nœud et un lien par ligne, les clés facultatives omises quand elles
    /// n'ont pas de valeur. Deux graphes égaux donnent les mêmes octets.
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
        let nodes: Vec<String> = self.nodes.iter().map(node_line).collect();
        json::push_list(&mut out, "nodes", &nodes, false);
        let edges: Vec<String> = self.edges.iter().map(edge_line).collect();
        json::push_list(&mut out, "edges", &edges, true);
        out.push_str("}\n");
        out
    }
}

fn node_line(node: &Node) -> String {
    let mut line = format!(
        "{{\"id\": {}, \"width\": {}, \"height\": {}",
        json::quote(&node.id),
        node.width,
        node.height
    );
    if let Some(layer) = node.layer {
        line.push_str(&format!(", \"layer\": {layer}"));
    }
    if let Some(order) = node.order {
        line.push_str(&format!(", \"order\": {order}"));
    }
    line.push('}');
    line
}

fn edge_line(edge: &Edge) -> String {
    let mut line = format!(
        "{{\"id\": {}, \"source\": {}, \"target\": {}",
        json::quote(&edge.id),
        json::quote(&edge.source),
        json::quote(&edge.target)
    );
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
    let mut obj = Obj::open(root, "graph", diags)?;
    read::header(&mut obj, GRAPH_FORMAT, diags);
    let name = obj
        .req("name", diags)
        .and_then(|v| read::str_of(v, "`name` (graph)", diags));
    let direction = obj
        .opt("direction")
        .and_then(|v| read::choice_of(v, "`direction` (graph)", &Direction::TABLE, diags));
    let nodes = obj
        .req("nodes", diags)
        .and_then(|v| read::array_of(v, "`nodes` (graph)", diags));
    let nodes: Option<Vec<Node>> =
        nodes.map(|items| items.iter().filter_map(|v| read_node(v, diags)).collect());
    let edges = obj
        .req("edges", diags)
        .and_then(|v| read::array_of(v, "`edges` (graph)", diags));
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

fn read_node(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<Node> {
    let mut obj = Obj::open(value, "node", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, "`id` (node)", diags));
    let width = obj
        .req("width", diags)
        .and_then(|v| read::int_of(v, "`width` (node)", diags));
    let height = obj
        .req("height", diags)
        .and_then(|v| read::int_of(v, "`height` (node)", diags));
    let layer = obj
        .opt("layer")
        .and_then(|v| read::int_of(v, "`layer` (node)", diags));
    let order = obj
        .opt("order")
        .and_then(|v| read::int_of(v, "`order` (node)", diags));
    obj.close(diags);
    Some(Node {
        id: id?.to_string(),
        width: width?,
        height: height?,
        layer,
        order,
        pos: value.pos,
    })
}

fn read_edge(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<Edge> {
    let mut obj = Obj::open(value, "edge", diags)?;
    let id = obj
        .req("id", diags)
        .and_then(|v| read::str_of(v, "`id` (edge)", diags));
    let source = obj
        .req("source", diags)
        .and_then(|v| read::str_of(v, "`source` (edge)", diags));
    let target = obj
        .req("target", diags)
        .and_then(|v| read::str_of(v, "`target` (edge)", diags));
    let source_side = obj
        .opt("source_side")
        .and_then(|v| read::choice_of(v, "`source_side` (edge)", &Side::TABLE, diags));
    let target_side = obj
        .opt("target_side")
        .and_then(|v| read::choice_of(v, "`target_side` (edge)", &Side::TABLE, diags));
    let labels = match obj.opt("labels") {
        None => Some(Vec::new()),
        Some(v) => read::array_of(v, "`labels` (edge)", diags)
            .map(|items| items.iter().filter_map(|v| read_label(v, diags)).collect()),
    };
    obj.close(diags);
    Some(Edge {
        id: id?.to_string(),
        source: source?.to_string(),
        target: target?.to_string(),
        source_side,
        target_side,
        labels: labels?,
        pos: value.pos,
    })
}

fn read_label(value: &Value, diags: &mut Vec<Diagnostic>) -> Option<Label> {
    let mut obj = Obj::open(value, "label", diags)?;
    let width = obj
        .req("width", diags)
        .and_then(|v| read::int_of(v, "`width` (label)", diags));
    let height = obj
        .req("height", diags)
        .and_then(|v| read::int_of(v, "`height` (label)", diags));
    let at = obj
        .req("at", diags)
        .and_then(|v| read::choice_of(v, "`at` (label)", &Anchor::TABLE, diags));
    obj.close(diags);
    Some(Label {
        width: width?,
        height: height?,
        at: at?,
    })
}
