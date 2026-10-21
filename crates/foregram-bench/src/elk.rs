// ForegramPM — placement measurement bench: ELK import (elk-models).
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Extracts from an `.elkt` file, read by [`crate::elkt`], a [`Graph`] of the
//! interchange format: nodes and nesting, sizes, ports, edges and edge
//! labels. ELK's layout options do not carry over, except those the
//! format can express: `direction`, `portConstraints` and `port.side`,
//! `port.index`, `edgeLabels.placement`. The positions and routes written in
//! the file are layout results, not data: they do not
//! carry over either.
//!
//! What does not carry over is counted in [`Notes`], so that nothing is lost
//! silently: node and port labels, edge labels without a size, leaves without
//! a size (ELK treats them as null; here they get 1 × 1, the smallest size of
//! the format), port positions fixed on a container (the format does not
//! allow them, the container's size being only known at layout
//! time).
//!
//! ELK identifiers are only unique within their container; those of the
//! format are unique across the whole graph. A node keeps its identifier if all
//! the nodes of the file have a distinct one, otherwise all take their path
//! (`parent.child`); likewise ports, prefixed with their node when needed, and
//! edges, prefixed with the path of their container. An edge without an
//! identifier gets one, `e1`, `e2`…, the first free.
//!
//! Import refusals carry the `E` family, following those of the reader,
//! and point into the `.elkt`; those of the produced graph are the validator's,
//! with the same position.

use std::collections::BTreeMap;

use foregram_graph::{
    Anchor, Diagnostic, Direction, Edge, Graph, Label, Message, Node, Point, Port, Side, Size,
};

use crate::elkt::{self, ElkEdge, ElkNode, Number, Property, QualifiedId, Value};

/// The imported graph, and what did not carry over into it.
#[derive(Clone, Debug)]
pub struct Imported {
    pub graph: Graph,
    pub notes: Notes,
}

/// What the import did not carry into the graph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Notes {
    /// Labels of nodes, of ports, and of edges without a size.
    pub labels: usize,
    /// Leaves without a size, set to 1 × 1.
    pub sizeless: usize,
    /// Ports of a container whose fixed position is not carried over.
    pub container_ports: usize,
}

/// Size given to a leaf without `layout [size: …]`.
pub const POINT: Size = Size {
    width: 1,
    height: 1,
};

/// Reads and imports an `.elkt`. `name` becomes the graph's name.
pub fn import(name: &str, bytes: &[u8]) -> Result<Imported, Vec<Diagnostic>> {
    let root = elkt::parse(bytes).map_err(|d| vec![d])?;
    let mut index = Index::default();
    let root_id = root.id.as_deref().unwrap_or("");
    index.collect(&root, root_id, "", root_id);
    index.choose();

    let mut importer = Importer {
        index: &index,
        direction: None,
        notes: Notes::default(),
        diags: Vec::new(),
        edges: Vec::new(),
    };
    importer.direction = option(&root.properties, DIRECTION)
        .and_then(|p| choice(p, &DIRECTIONS, &mut importer.diags))
        .flatten();
    let nodes = importer.children(&root, "", &[root_id]);
    let direction = importer.direction;
    importer.notes.labels += root.labels.len();
    if !root.ports.is_empty() {
        let message = Message::new("E011.ports");
        importer.diags.push(Diagnostic::new(root.pos, message));
    }
    let edges = importer.name_edges();
    let graph = Graph {
        name: name.to_string(),
        direction,
        nodes,
        edges,
        pos: root.pos,
    };
    let mut diags = importer.diags;
    if diags.is_empty() {
        diags = graph.validate();
        if diags.is_empty() {
            return Ok(Imported {
                graph,
                notes: importer.notes,
            });
        }
    }
    diags.sort_by(|a, b| (a.pos, a.code).cmp(&(b.pos, b.code)));
    Err(diags)
}

// ----------------------------------------------------------------- options

/// The ELK options the import reads, by their full identifier. A written
/// key designates them whole or by a suffix starting at a dot:
/// `org.eclipse.elk.port.side`, `elk.port.side`, `port.side`, `side`.
const DIRECTION: &str = "org.eclipse.elk.direction";
const PORT_CONSTRAINTS: &str = "org.eclipse.elk.portConstraints";
const PORT_SIDE: &str = "org.eclipse.elk.port.side";
const PORT_INDEX: &str = "org.eclipse.elk.port.index";
const LABEL_PLACEMENT: &str = "org.eclipse.elk.edgeLabels.placement";

/// The last property designating this option, as ELK keeps the last.
fn option<'a>(properties: &'a [Property], id: &str) -> Option<&'a Property> {
    properties.iter().rev().find(|p| {
        p.key == id
            || id
                .strip_suffix(p.key.as_str())
                .is_some_and(|s| s.ends_with('.'))
    })
}

/// The words of an ELK option and their meaning; `None` for `UNDEFINED`, which
/// means absence.
type Words<T> = [(&'static str, Option<T>)];

const DIRECTIONS: [(&str, Option<Direction>); 5] = [
    ("UNDEFINED", None),
    ("RIGHT", Some(Direction::Right)),
    ("DOWN", Some(Direction::Down)),
    ("LEFT", Some(Direction::Left)),
    ("UP", Some(Direction::Up)),
];

const SIDES: [(&str, Option<Side>); 5] = [
    ("UNDEFINED", None),
    ("NORTH", Some(Side::North)),
    ("EAST", Some(Side::East)),
    ("SOUTH", Some(Side::South)),
    ("WEST", Some(Side::West)),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Constraints {
    Free,
    FixedSide,
    FixedOrder,
    FixedPos,
}

const CONSTRAINTS: [(&str, Option<Constraints>); 5] = [
    ("UNDEFINED", None),
    ("FREE", Some(Constraints::Free)),
    ("FIXED_SIDE", Some(Constraints::FixedSide)),
    ("FIXED_ORDER", Some(Constraints::FixedOrder)),
    ("FIXED_POS", Some(Constraints::FixedPos)),
];

const PLACEMENTS: [(&str, Option<Anchor>); 4] = [
    ("UNDEFINED", None),
    ("CENTER", Some(Anchor::Center)),
    ("HEAD", Some(Anchor::Target)),
    ("TAIL", Some(Anchor::Source)),
];

/// The value of a word option, written bare or in quotes. `None`
/// for a word outside the list, which is refused.
fn choice<T: Copy>(
    property: &Property,
    words: &Words<T>,
    diags: &mut Vec<Diagnostic>,
) -> Option<Option<T>> {
    let text = match &property.value {
        Value::Id(text) | Value::Str(text) => text.clone(),
        Value::Number(number) => number.text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_string(),
    };
    if let Some((_, choice)) = words.iter().find(|(word, _)| *word == text) {
        return Some(*choice);
    }
    let list: Vec<&str> = words.iter().map(|(word, _)| *word).collect();
    let message = Message::new("E010")
        .text("key", &property.key)
        .text("value", text)
        .text("words", list.join(", "));
    diags.push(Diagnostic::new(property.pos, message));
    None
}

/// An ELK number, in pixels, carried in hundredths of a pixel: `12.5` gives
/// 1250. Beyond two decimals, rounded to nearest, half away from
/// zero. `None` if the number does not fit in an `i64`.
pub fn hundredths(number: &Number) -> Option<i64> {
    crate::engine::hundredths(&number.text)
}

// ------------------------------------------------------------------- index

/// What a qualified name can designate.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    Root,
    /// A node, by its retained identifier.
    Node(String),
    /// A port, by the retained identifier of its node and its own.
    Port(String, String),
}

/// The names of all nodes and ports, as ELK qualifies them, and the
/// identifiers the graph gives them.
#[derive(Default)]
struct Index {
    /// ELK qualified name (root included) → target.
    names: BTreeMap<String, Target>,
    /// Per node: its written identifier, and its path from the first level.
    nodes: Vec<(String, String)>,
    /// Per port: its written identifier, and the path of its node.
    ports: Vec<(String, String)>,
    /// True if the written identifiers suffice, false if paths are needed.
    plain_nodes: bool,
    plain_ports: bool,
}

impl Index {
    /// `qualified` is the ELK name of the container, `path` its path without the
    /// root; `root` is the root's identifier, or empty.
    fn collect(&mut self, node: &ElkNode, qualified: &str, path: &str, root: &str) {
        if path.is_empty() && qualified == root {
            self.names.insert(root.to_string(), Target::Root);
            for port in &node.ports {
                self.names.insert(join(root, &port.id), Target::Root);
            }
        }
        for child in &node.children {
            let id = child.id.clone().unwrap_or_default();
            let child_qualified = join(qualified, &id);
            let child_path = join(path, &id);
            self.nodes.push((id, child_path.clone()));
            for port in &child.ports {
                self.ports.push((port.id.clone(), child_path.clone()));
            }
            self.collect(child, &child_qualified, &child_path, root);
        }
    }

    /// Decides between written identifiers and paths, then names all the targets.
    fn choose(&mut self) {
        self.plain_nodes = distinct(self.nodes.iter().map(|(id, _)| id.as_str()));
        self.plain_ports = distinct(self.ports.iter().map(|(id, _)| id.as_str()));
        let root = self
            .names
            .iter()
            .find(|(_, target)| **target == Target::Root)
            .map(|(name, _)| name.clone())
            .unwrap_or_default();
        let nodes = std::mem::take(&mut self.nodes);
        for (id, path) in &nodes {
            let chosen = self.node_id(id, path);
            self.names.insert(join(&root, path), Target::Node(chosen));
        }
        let ports = std::mem::take(&mut self.ports);
        for (id, node_path) in &ports {
            let node_id = nodes
                .iter()
                .find(|(_, path)| path == node_path)
                .map_or(String::new(), |(id, path)| self.node_id(id, path));
            let chosen = if self.plain_ports {
                id.clone()
            } else {
                join(&node_id, id)
            };
            self.names.insert(
                join(&join(&root, node_path), id),
                Target::Port(node_id, chosen),
            );
        }
        self.nodes = nodes;
        self.ports = ports;
    }

    fn node_id(&self, id: &str, path: &str) -> String {
        if self.plain_nodes {
            id.to_string()
        } else {
            path.to_string()
        }
    }

    /// Resolves a name written in a container: relative to the container, then to
    /// each of its ancestors, root included, then absolute — like the Xtext
    /// scope ELK uses.
    fn resolve(&self, name: &QualifiedId, scopes: &[&str]) -> Option<&Target> {
        let text = name.text();
        scopes
            .iter()
            .map(|scope| join(scope, &text))
            .chain(std::iter::once(text.clone()))
            .find_map(|candidate| self.names.get(&candidate))
    }
}

fn join(prefix: &str, id: &str) -> String {
    if prefix.is_empty() {
        id.to_string()
    } else {
        format!("{prefix}.{id}")
    }
}

fn distinct<'a>(ids: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    ids.into_iter().all(|id| seen.insert(id))
}

// ---------------------------------------------------------------- import

struct Importer<'i> {
    index: &'i Index,
    direction: Option<Direction>,
    notes: Notes,
    diags: Vec<Diagnostic>,
    /// The edges, awaiting their identifier.
    edges: Vec<Pending>,
}

/// An edge as read, before its identifier: the one it writes if it has one, and
/// the path of the container where it is written.
struct Pending {
    written: Option<String>,
    path: String,
    edge: Edge,
}

impl Importer<'_> {
    /// The children of a container, `path` being its path and `scopes` the
    /// ELK names of the container and its ancestors, from nearest to root.
    fn children(&mut self, container: &ElkNode, path: &str, scopes: &[&str]) -> Vec<Node> {
        for edge in &container.edges {
            self.edge(edge, path, scopes);
        }
        let mut nodes = Vec::new();
        for child in &container.children {
            let written = child.id.clone().unwrap_or_default();
            let child_path = join(path, &written);
            let id = self.index.node_id(&written, &child_path);
            let qualified = join(scopes.first().copied().unwrap_or(""), &written);
            let mut inner: Vec<&str> = vec![&qualified];
            inner.extend_from_slice(scopes);
            let children = self.children(child, &child_path, &inner);
            self.notes.labels += child.labels.len();
            let size = if children.is_empty() {
                match &child.layout.size {
                    Some(size) => self.size(size),
                    None => {
                        self.notes.sizeless += 1;
                        Some(POINT)
                    }
                }
            } else {
                None
            };
            let constraints = option(&child.properties, PORT_CONSTRAINTS)
                .and_then(|p| choice(p, &CONSTRAINTS, &mut self.diags))
                .flatten()
                .unwrap_or(Constraints::Free);
            let ports = self.ports(child, &id, size, constraints);
            nodes.push(Node {
                id,
                size,
                layer: None,
                order: None,
                padding: None,
                ports_ordered: constraints == Constraints::FixedOrder && !ports.is_empty(),
                ports,
                children,
                pos: child.pos,
            });
        }
        nodes
    }

    fn size(&mut self, (width, height): &(Number, Number)) -> Option<Size> {
        let width = self.number(width);
        let height = self.number(height);
        Some(Size {
            width: width?,
            height: height?,
        })
    }

    fn number(&mut self, number: &Number) -> Option<i64> {
        let value = hundredths(number);
        if value.is_none() {
            let message = Message::new("E009").text("text", &number.text);
            self.diags.push(Diagnostic::new(number.pos, message));
        }
        value
    }

    /// The ports of a node. Under `FIXED_SIDE` and above, each port has a
    /// side: the one it writes, or the one ELK would deduce from its position;
    /// under `FIXED_ORDER`, the list follows ELK's order, by side then by
    /// `index` or position; under `FIXED_POS`, the anchor point is fixed,
    /// an absent position being (0, 0) as in ELK.
    fn ports(
        &mut self,
        node: &ElkNode,
        node_id: &str,
        size: Option<Size>,
        constraints: Constraints,
    ) -> Vec<Port> {
        let mut ports = Vec::new();
        let indexed = node
            .ports
            .iter()
            .all(|port| option(&port.properties, PORT_INDEX).is_some());
        for (rank, port) in node.ports.iter().enumerate() {
            self.notes.labels += port.labels.len();
            let id = match self.index.plain_ports {
                true => port.id.clone(),
                false => join(node_id, &port.id),
            };
            let box_size = match &port.layout.size {
                Some(size) => self.size(size),
                None => None,
            };
            let position = match &port.layout.position {
                Some((x, y)) => match (self.number(x), self.number(y)) {
                    (Some(x), Some(y)) => Some(Point { x, y }),
                    _ => None,
                },
                None => None,
            };
            let mut side = None;
            let mut anchor = None;
            if constraints >= Constraints::FixedSide {
                side = option(&port.properties, PORT_SIDE)
                    .and_then(|p| choice(p, &SIDES, &mut self.diags))
                    .flatten()
                    .or_else(|| self.inferred_side(position, box_size, size));
                if constraints == Constraints::FixedPos {
                    let position = position.unwrap_or(Point { x: 0, y: 0 });
                    match (side, size) {
                        (Some(side), Some(size)) => {
                            anchor = Some(on_border(position, box_size, side, size));
                        }
                        (Some(_), None) => self.notes.container_ports += 1,
                        (None, _) => {}
                    }
                }
            }
            let index = match (
                constraints == Constraints::FixedOrder && indexed,
                option(&port.properties, PORT_INDEX),
            ) {
                (
                    true,
                    Some(Property {
                        value: Value::Number(number),
                        ..
                    }),
                ) => self.number(number).map(|n| n.div_euclid(100)),
                _ => None,
            };
            let sort_key = (
                side.map_or(0, |side| match side {
                    Side::North => 0,
                    Side::East => 1,
                    Side::South => 2,
                    Side::West => 3,
                }),
                index.unwrap_or(0),
                order_key(side, position),
                rank,
            );
            ports.push((
                sort_key,
                Port {
                    id,
                    side,
                    position: anchor,
                    size: box_size,
                    pos: port.pos,
                },
            ));
        }
        if constraints == Constraints::FixedOrder {
            ports.sort_by_key(|(key, _)| *key);
        }
        ports.into_iter().map(|(_, port)| port).collect()
    }

    /// The side ELK gives to a port without `side` on a node with fixed sides
    /// (`ElkUtil.calcPortSide`), from the port's position, or the layout
    /// direction if the node has no size. Absent positions and sizes are
    /// zero, as in ELK.
    fn inferred_side(
        &self,
        position: Option<Point>,
        box_size: Option<Size>,
        node: Option<Size>,
    ) -> Option<Side> {
        let direction = self.direction.unwrap_or(Direction::Right);
        let Some(node) = node.filter(|size| *size != POINT) else {
            return Some(match direction {
                Direction::Right => Side::East,
                Direction::Down => Side::South,
                Direction::Left => Side::West,
                Direction::Up => Side::North,
            });
        };
        let Point { x, y } = position.unwrap_or(Point { x: 0, y: 0 });
        let (w, h) = box_size.map_or((0, 0), |s| (s.width, s.height));
        match direction {
            Direction::Left | Direction::Right if x < 0 => return Some(Side::West),
            Direction::Left | Direction::Right if x + w > node.width => return Some(Side::East),
            Direction::Up | Direction::Down if y < 0 => return Some(Side::North),
            Direction::Up | Direction::Down if y + h > node.height => return Some(Side::South),
            _ => {}
        }
        // The port's center, in fractions of the node's size, multiplied
        // by 2 × width × height to stay in integers.
        let cx = (2 * x + w) * node.height;
        let cy = (2 * y + h) * node.width;
        let whole = 2 * node.width * node.height;
        Some(if cx + cy <= whole && cx <= cy {
            Side::West
        } else if cx + cy >= whole && cx >= cy {
            Side::East
        } else if 2 * cy < whole {
            Side::North
        } else {
            Side::South
        })
    }

    fn edge(&mut self, edge: &ElkEdge, path: &str, scopes: &[&str]) {
        let ends = edge.sources.len() + edge.targets.len();
        if ends != 2 {
            let message = Message::new("E008").text("count", ends);
            self.diags.push(Diagnostic::new(edge.pos, message));
            return;
        }
        let (Some(source), Some(target)) = (edge.sources.first(), edge.targets.first()) else {
            return;
        };
        let source = self.end(source, scopes);
        let target = self.end(target, scopes);
        let mut labels = Vec::new();
        for label in &edge.labels {
            let Some(size) = &label.layout.size else {
                self.notes.labels += 1;
                continue;
            };
            let at = option(&label.properties, LABEL_PLACEMENT)
                .and_then(|p| choice(p, &PLACEMENTS, &mut self.diags))
                .flatten()
                .unwrap_or(Anchor::Center);
            if let Some(size) = self.size(size) {
                labels.push(Label {
                    width: size.width,
                    height: size.height,
                    at,
                });
            }
        }
        let (Some((source, source_port)), Some((target, target_port))) = (source, target) else {
            return;
        };
        self.edges.push(Pending {
            written: edge.id.clone(),
            path: path.to_string(),
            edge: Edge {
                id: String::new(),
                source,
                target,
                source_port,
                target_port,
                source_side: None,
                target_side: None,
                labels,
                pos: edge.pos,
            },
        });
    }

    /// The node and, where applicable, the port an endpoint designates.
    fn end(&mut self, name: &QualifiedId, scopes: &[&str]) -> Option<(String, Option<String>)> {
        match self.index.resolve(name, scopes) {
            Some(Target::Node(id)) => Some((id.clone(), None)),
            Some(Target::Port(node, port)) => Some((node.clone(), Some(port.clone()))),
            Some(Target::Root) => {
                let message = Message::new("E011").text("name", name.text());
                self.diags.push(Diagnostic::new(name.pos, message));
                None
            }
            None => {
                let message = Message::new("E007").text("name", name.text());
                self.diags.push(Diagnostic::new(name.pos, message));
                None
            }
        }
    }

    /// Puts the edges back in file order and names them: the written identifier,
    /// prefixed with the path of its container if the written identifiers
    /// repeat; otherwise the first free `e<n>`.
    fn name_edges(&mut self) -> Vec<Edge> {
        let mut pending = std::mem::take(&mut self.edges);
        pending.sort_by_key(|p| p.edge.pos);
        let plain = distinct(pending.iter().filter_map(|p| p.written.as_deref()));
        let taken: Vec<String> = pending
            .iter()
            .filter_map(|p| {
                let written = p.written.as_deref()?;
                Some(if plain {
                    written.to_string()
                } else {
                    join(&p.path, written)
                })
            })
            .collect();
        let mut next = 1;
        let mut generated = Vec::new();
        for mut pending in pending {
            pending.edge.id = match pending.written {
                Some(id) if plain => id,
                Some(id) => join(&pending.path, &id),
                None => loop {
                    let candidate = format!("e{next}");
                    next += 1;
                    if !taken.contains(&candidate) {
                        break candidate;
                    }
                },
            };
            generated.push(pending.edge);
        }
        generated
    }
}

/// Ordering key of the ports on one side, clockwise:
/// by x on the north, by y on the east, reversed on the south and west.
fn order_key(side: Option<Side>, position: Option<Point>) -> i64 {
    let Point { x, y } = position.unwrap_or(Point { x: 0, y: 0 });
    match side {
        Some(Side::North) => x,
        Some(Side::East) => y,
        Some(Side::South) => -x,
        Some(Side::West) | None => -y,
    }
}

/// The anchor point of a port with a fixed position: the center of its box,
/// brought back onto the edge of its side. ELK's box has its corner at `position`;
/// the format's is centered on the anchor.
fn on_border(position: Point, box_size: Option<Size>, side: Side, node: Size) -> Point {
    let (w, h) = box_size.map_or((0, 0), |s| (s.width, s.height));
    let cx = (2 * position.x + w).div_euclid(2).clamp(0, node.width);
    let cy = (2 * position.y + h).div_euclid(2).clamp(0, node.height);
    match side {
        Side::West => Point { x: 0, y: cy },
        Side::East => Point {
            x: node.width,
            y: cy,
        },
        Side::North => Point { x: cx, y: 0 },
        Side::South => Point {
            x: cx,
            y: node.height,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foregram_graph::Pos;

    fn graph(text: &str) -> Graph {
        import("g", text.as_bytes()).unwrap().graph
    }

    fn refusals(text: &str) -> Vec<(&'static str, u32, u32)> {
        import("g", text.as_bytes())
            .unwrap_err()
            .iter()
            .map(|d| (d.code, d.pos.line, d.pos.col))
            .collect()
    }

    fn texts(text: &str) -> Vec<String> {
        import("g", text.as_bytes())
            .unwrap_err()
            .iter()
            .map(Diagnostic::to_string)
            .collect()
    }

    fn num(text: &str) -> Number {
        Number {
            text: text.to_string(),
            pos: Pos::NONE,
        }
    }

    #[test]
    fn numbers_are_carried_in_hundredths_and_rounded() {
        let cases = [
            ("0", 0),
            ("12", 1200),
            ("12.5", 1250),
            ("-3.25", -325),
            ("+7", 700),
            ("30.004", 3000),
            ("30.005", 3001),
            ("-30.005", -3001),
            ("0.999", 100),
            ("1e2", 10000),
            ("1.5E-2", 2),
            ("25e-4", 0),
            ("0.005", 1),
            ("0.004", 0),
            ("92233720368547758.07", 9223372036854775807),
        ];
        for (text, expected) in cases {
            assert_eq!(hundredths(&num(text)), Some(expected), "{text}");
        }
        assert_eq!(hundredths(&num("92233720368547758.08")), None);
        assert_eq!(hundredths(&num("1e40")), None);
        assert_eq!(hundredths(&num("1e-400")), Some(0));
    }

    const SAMPLE: &str = "elk.direction: DOWN\n\
        node a {\n\
        \x20 layout [ size: 10, 20.5 ]\n\
        \x20 portConstraints: FIXED_SIDE\n\
        \x20 label \"A\"\n\
        \x20 port out { layout [ size: 2, 2 ] side: EAST label \"o\" }\n\
        }\n\
        node sys {\n\
        \x20 node core { layout [ size: 5, 5 ] portConstraints: FIXED_ORDER port in { side: WEST } }\n\
        \x20 node store\n\
        \x20 edge core -> store\n\
        }\n\
        edge e1: a.out -> sys.core.in {\n\
        \x20 label \"x\" { layout [ size: 3, 1 ] edgeLabels.placement: HEAD }\n\
        \x20 label \"no size\"\n\
        }\n\
        edge sys.store -> sys\n";

    #[test]
    fn the_sample_becomes_a_version_2_graph() {
        let imported = import("sample", SAMPLE.as_bytes()).unwrap();
        assert_eq!(
            imported.graph.to_json(),
            "{\n\
            \x20 \"format\": \"foregram-graph\",\n\
            \x20 \"version\": 2,\n\
            \x20 \"name\": \"sample\",\n\
            \x20 \"direction\": \"down\",\n\
            \x20 \"nodes\": [\n\
            \x20   {\"id\": \"a\", \"width\": 1000, \"height\": 2050, \"ports\": [{\"id\": \"out\", \"side\": \"east\", \"width\": 200, \"height\": 200}]},\n\
            \x20   {\"id\": \"sys\", \"children\": [\n\
            \x20     {\"id\": \"core\", \"width\": 500, \"height\": 500, \"ports_ordered\": true, \"ports\": [{\"id\": \"in\", \"side\": \"west\"}]},\n\
            \x20     {\"id\": \"store\", \"width\": 1, \"height\": 1}\n\
            \x20   ]}\n\
            \x20 ],\n\
            \x20 \"edges\": [\n\
            \x20   {\"id\": \"e2\", \"source\": \"core\", \"target\": \"store\"},\n\
            \x20   {\"id\": \"e1\", \"source\": \"a\", \"target\": \"core\", \"source_port\": \"out\", \"target_port\": \"in\", \"labels\": [{\"width\": 300, \"height\": 100, \"at\": \"target\"}]},\n\
            \x20   {\"id\": \"e3\", \"source\": \"store\", \"target\": \"sys\"}\n\
            \x20 ]\n\
            }\n"
        );
        assert_eq!(
            imported.notes,
            Notes {
                labels: 3,
                sizeless: 1,
                container_ports: 0,
            }
        );
        // Positions come from the `.elkt`.
        assert_eq!(
            imported.graph.nodes[1].children[0].pos,
            Pos { line: 9, col: 3 }
        );
        assert_eq!(imported.graph.edges[1].pos, Pos { line: 13, col: 1 });
    }

    #[test]
    fn names_are_resolved_like_elk_and_kept_plain_when_distinct() {
        // Relative to the container first, then to the ancestors, then absolute.
        let g = graph(
            "graph G\nnode x { layout [size: 1, 1] }\nnode p {\n node x { layout [size: 1, 1] }\n node q { node x { layout [size: 1, 1] }\n  edge x -> p.x\n  edge G.x -> q.x }\n}\nedge x -> p.q.x",
        );
        let edges: Vec<(&str, &str)> = g
            .edges
            .iter()
            .map(|e| (e.source.as_str(), e.target.as_str()))
            .collect();
        assert_eq!(edges, [("p.q.x", "p.x"), ("x", "p.q.x"), ("x", "p.q.x")]);
        assert_eq!(g.nodes[0].id, "x");
        assert_eq!(g.nodes[1].children[1].id, "p.q");
    }

    #[test]
    fn repeated_port_names_are_prefixed_with_their_node() {
        let g = graph(
            "node a { layout [size: 1, 1] port p }\nnode b { layout [size: 1, 1] port p }\nedge a.p -> b.p",
        );
        assert_eq!(g.nodes[0].ports[0].id, "a.p");
        assert_eq!(g.edges[0].source_port.as_deref(), Some("a.p"));
        assert_eq!(g.edges[0].target_port.as_deref(), Some("b.p"));
        // Repeated node names carry the ports along with them.
        let g = graph(
            "node a { node p { layout [size: 1, 1] port q } }\nnode b { node p { layout [size: 1, 1] port q } }",
        );
        assert_eq!(g.nodes[1].children[0].ports[0].id, "b.p.q");
    }

    #[test]
    fn generated_edge_identifiers_skip_the_written_ones() {
        let g = graph(
            "node a { layout [size: 1, 1] }\nedge a -> a\nedge e1: a -> a\nedge e3: a -> a\nedge a -> a",
        );
        let ids: Vec<&str> = g.edges.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["e2", "e1", "e3", "e4"]);
        // Identifiers repeated from one container to another: prefixed with the path.
        let g = graph(
            "node a { node x { layout [size: 1, 1] } edge e1: x -> x }\n\
             node b { node x { layout [size: 1, 1] } edge e1: x -> x edge x -> x }\nedge e1: a -> b",
        );
        let ids: Vec<&str> = g.edges.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["a.e1", "b.e1", "e2", "e1"]);
    }

    #[test]
    fn port_sides_follow_the_constraints() {
        // FREE: nothing is carried, even if written.
        let g = graph("node a { layout [size: 10, 10] port p { side: WEST } }");
        assert_eq!(g.nodes[0].ports[0].side, None);
        // FIXED_SIDE without a side: deduced from the position, the way ELK does.
        let text = |pos: &str| {
            format!(
                "node a {{ layout [size: 10, 10] portConstraints: FIXED_SIDE \
                 port p {{ layout [position: {pos} size: 2, 2] }} }}"
            )
        };
        let side = |pos: &str| graph(&text(pos)).nodes[0].ports[0].side;
        assert_eq!(side("-1, 4"), Some(Side::West));
        assert_eq!(side("9, 4"), Some(Side::East));
        assert_eq!(side("4, 0"), Some(Side::North));
        assert_eq!(side("4, 8"), Some(Side::South));
        assert_eq!(side("1, 4"), Some(Side::West));
        assert_eq!(side("7, 4"), Some(Side::East));
        // Vertical direction: the north and south edges win.
        let g = graph(&format!("elk.direction: DOWN\n{}", text("4, 9")));
        assert_eq!(g.nodes[0].ports[0].side, Some(Side::South));
        // A node without a size: the side facing the direction.
        let g = graph("node a { portConstraints: FIXED_SIDE port p }");
        assert_eq!(g.nodes[0].ports[0].side, Some(Side::East));
        assert_eq!(g.nodes[0].size, Some(POINT));
        // UNDEFINED means absence, and a written `side` wins over the position.
        let g = graph(
            "node a { layout [size: 10, 10] portConstraints: FIXED_SIDE \
             port p { layout [position: -1, 4] side: UNDEFINED } port q { layout [position: -1, 4] side: NORTH } }",
        );
        assert_eq!(g.nodes[0].ports[0].side, Some(Side::West));
        assert_eq!(g.nodes[0].ports[1].side, Some(Side::North));
    }

    #[test]
    fn fixed_order_sorts_ports_by_side_then_index_or_position() {
        let g = graph(
            "node a { layout [size: 10, 10] portConstraints: FIXED_ORDER\n\
             port w2 { side: WEST index: 2 }\n\
             port e1 { side: EAST index: 1 }\n\
             port w1 { side: WEST index: 1 }\n\
             port n { side: NORTH index: 0 } }",
        );
        let ids: Vec<&str> = g.nodes[0].ports.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["n", "e1", "w1", "w2"]);
        assert!(g.nodes[0].ports_ordered);
        // Without an index: by position, reversed on the west and south.
        let g = graph(
            "node a { layout [size: 10, 10] portConstraints: FIXED_ORDER\n\
             port w1 { layout [position: 0, 2] side: WEST }\n\
             port w2 { layout [position: 0, 8] side: WEST }\n\
             port s1 { layout [position: 2, 10] side: SOUTH }\n\
             port s2 { layout [position: 8, 10] side: SOUTH } }",
        );
        let ids: Vec<&str> = g.nodes[0].ports.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["s2", "s1", "w2", "w1"]);
        // Indexes on only some of the ports: ignored, file order.
        let g = graph(
            "node a { layout [size: 10, 10] portConstraints: FIXED_ORDER\n\
             port b { side: WEST index: 1 } port a { side: WEST } }",
        );
        let ids: Vec<&str> = g.nodes[0].ports.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["b", "a"]);
        // FIXED_SIDE does not reorder and does not fix the order.
        let g = graph(
            "node a { layout [size: 10, 10] portConstraints: FIXED_SIDE\n\
             port w { side: WEST } port n { side: NORTH } }",
        );
        assert_eq!(g.nodes[0].ports[0].id, "w");
        assert!(!g.nodes[0].ports_ordered);
    }

    #[test]
    fn fixed_positions_are_anchored_on_the_border() {
        let g = graph(
            "node a { layout [size: 10, 10] portConstraints: FIXED_POS\n\
             port w { layout [position: -2, 3 size: 2, 2] side: WEST }\n\
             port e { layout [position: 10, 1 size: 2, 2] }\n\
             port n { layout [position: 4, -2 size: 2, 2] }\n\
             port s { layout [position: 20, 10] side: SOUTH } }",
        );
        let anchors: Vec<(Option<Side>, Option<Point>)> = g.nodes[0]
            .ports
            .iter()
            .map(|p| (p.side, p.position))
            .collect();
        assert_eq!(
            anchors,
            [
                (Some(Side::West), Some(Point { x: 0, y: 400 })),
                (Some(Side::East), Some(Point { x: 1000, y: 200 })),
                (Some(Side::North), Some(Point { x: 500, y: 0 })),
                // Outside the edge: brought back to the corner.
                (Some(Side::South), Some(Point { x: 1000, y: 1000 })),
            ]
        );
        assert!(!g.nodes[0].ports_ordered);
        // Without a position, (0, 0) as in ELK: the corner, on the side it is given.
        let g = graph(
            "node a { layout [size: 10, 10] portConstraints: FIXED_POS\n port p { side: NORTH } }",
        );
        assert_eq!(g.nodes[0].ports[0].position, Some(Point { x: 0, y: 0 }));
        // On a container, the side carries over, the position does not, and it is counted.
        let imported = import(
            "g",
            b"node a { portConstraints: FIXED_POS port p { layout [position: 5, 0] side: NORTH } node c { layout [size: 1, 1] } }",
        )
        .unwrap();
        let port = &imported.graph.nodes[0].ports[0];
        assert_eq!((port.side, port.position), (Some(Side::North), None));
        assert_eq!(imported.notes.container_ports, 1);
    }

    #[test]
    fn option_keys_match_by_suffix_and_the_last_one_wins() {
        let g = graph("org.eclipse.elk.direction: LEFT\nnode a { layout [size: 1, 1] }");
        assert_eq!(g.direction, Some(Direction::Left));
        let g = graph("direction: LEFT\nelk.direction: \"UP\"\nnode a { layout [size: 1, 1] }");
        assert_eq!(g.direction, Some(Direction::Up));
        // Another option with a similar name is not the right one.
        let g = graph("priority.direction: UP\nnode a { layout [size: 1, 1] }");
        assert_eq!(g.direction, None);
        let g = graph(
            "node a { layout [size: 1, 1] org.eclipse.elk.portConstraints: FIXED_SIDE port p { ^port.side: SOUTH } }",
        );
        assert_eq!(g.nodes[0].ports[0].side, Some(Side::South));
    }

    #[test]
    fn words_outside_the_option_are_refused() {
        let text = "elk.direction: DIAGONAL\nnode a { layout [size: 1, 1] portConstraints: FIXED_RATIO\n port p { side: 3 } }\n\
                    edge a -> a { label \"l\" { layout [size: 1, 1] edgeLabels.placement: true } }";
        assert_eq!(
            texts(text),
            [
                "1:1: error[E010]: `elk.direction` is `DIAGONAL`: UNDEFINED, RIGHT, DOWN, LEFT, UP expected",
                "2:30: error[E010]: `portConstraints` is `FIXED_RATIO`: UNDEFINED, FREE, FIXED_SIDE, FIXED_ORDER, FIXED_POS expected",
                "4:47: error[E010]: `edgeLabels.placement` is `true`: UNDEFINED, CENTER, HEAD, TAIL expected",
            ]
        );
        // The side is only read under a constraint that fixes it.
        assert_eq!(
            refusals(
                "node a { layout [size: 1, 1] portConstraints: FIXED_SIDE port p { side: 3 } }"
            ),
            [("E010", 1, 67)]
        );
    }

    #[test]
    fn unresolved_hyper_and_root_ends_are_refused() {
        let text = "graph G\nnode a { layout [size: 1, 1] port p }\nedge a -> b\nedge a, a.p -> a\nedge a -> G\nedge a -> G.p";
        assert_eq!(
            texts(text),
            [
                "3:11: error[E007]: `b` refers to no node or port",
                "4:1: error[E008]: an edge with 3 ends: one source and one target expected",
                "5:11: error[E011]: `G` is the graph itself, not a node",
                "6:11: error[E007]: `G.p` refers to no node or port",
            ]
        );
        assert_eq!(
            refusals(text),
            [
                ("E007", 3, 11),
                ("E008", 4, 1),
                ("E011", 5, 11),
                ("E007", 6, 11)
            ]
        );
        assert_eq!(
            texts("port p\nnode a { layout [size: 1, 1] }"),
            ["1:1: error[E011]: the graph itself has ports"]
        );
    }

    #[test]
    fn a_number_that_does_not_fit_is_refused() {
        assert_eq!(
            texts("node a { layout [size: 1e30, 1] }"),
            ["1:24: error[E009]: number `1e30` cannot be carried in hundredths of a pixel"]
        );
        assert_eq!(
            refusals(
                "node a { layout [size: 1, 1] port p { layout [size: 99999999999999999999, 1] } }"
            ),
            [("E009", 1, 53)]
        );
    }

    #[test]
    fn syntax_and_validation_refusals_point_into_the_file() {
        assert_eq!(refusals("node a {"), [("E004", 1, 9)]);
        // Two `a` nodes at the same level: ELK tells them apart, the format does not.
        assert_eq!(
            refusals("node a { layout [size: 1, 1] }\nnode a { layout [size: 1, 1] }"),
            [("G001", 2, 1)]
        );
        // A zero size is refused by the validator, at the node's line.
        assert_eq!(refusals("node a { layout [size: 0, 1] }"), [("S008", 1, 1)]);
    }

    #[test]
    fn imported_graphs_are_canonical_and_identical() {
        let once = import("g", SAMPLE.as_bytes()).unwrap().graph.to_json();
        let twice = import("g", SAMPLE.as_bytes()).unwrap().graph.to_json();
        assert_eq!(once, twice);
        assert_eq!(Graph::parse(once.as_bytes()).unwrap().to_json(), once);
    }
}
