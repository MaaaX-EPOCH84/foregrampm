// ForegramPM — placement bench: MCD import (in-house corpus).
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads an MCD written in the seed format, `dessiner_mcd.py`, and turns it into a
//! [`Graph`] in the exchange format, in memory.
//!
//! The format, as the seed reads it:
//!
//! ```text
//! # comment, in column 1
//! entite MEMBRE
//!   #id_membre            identifier
//!   nom                   property
//! association PRET
//!   date_pret             property carried by the association
//!   MEMBRE 0,n            link: entity, then cardinality
//! ```
//!
//! A block starts in column 1, its content is indented. Same refusals as
//! the seed, with a position: line not understood, indented line outside a
//! block, link in an entity, duplicate block, link to something that is not
//! an entity, association with fewer than two links.
//!
//! The graph: one node per entity and per association, in file order;
//! one link from the entity to the association for each link line, the
//! cardinality as a label against the entity; direction `right`, the seed's.
//! Sizes follow a provisional rule, for lack of an embedded font:
//! see [`text_box`].

use foregram_graph::{Anchor, Diagnostic, Direction, Edge, Graph, Label, Message, Node, Pos, Size};

/// Advance of one character, in hundredths of a pixel: 7 px, an average of Lato
/// at size 11. Provisional: the metrics of the embedded font will
/// replace it (P6).
pub const ADVANCE: i64 = 700;
/// Height of a line of text: 18 px.
pub const LINE: i64 = 1800;
/// Margin around a node's text, on each side: 8 px.
pub const MARGIN: i64 = 800;
/// Margin around a cardinality, left and right: 2 px.
pub const LABEL_MARGIN: i64 = 200;

/// The cardinalities the seed accepts.
const CARDINALITIES: [&str; 4] = ["0,1", "0,n", "1,1", "1,n"];

/// Reads an MCD. `name` becomes the graph's name.
///
/// Returns the graph, or all the refusals found, by position.
pub fn import(name: &str, bytes: &[u8]) -> Result<Graph, Vec<Diagnostic>> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let valid = bytes.get(..error.valid_up_to()).unwrap_or_default();
            let prefix = std::str::from_utf8(valid).unwrap_or_default();
            let line = prefix.matches('\n').count() + 1;
            let col = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
            let pos = Pos {
                line: u32::try_from(line).unwrap_or(u32::MAX),
                col: u32::try_from(col).unwrap_or(u32::MAX),
            };
            return Err(vec![Diagnostic::new(pos, Message::new("MCD001"))]);
        }
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);

    let mut diags = Vec::new();
    let blocks = parse(text, &mut diags);
    check(&blocks, &mut diags);
    if diags.is_empty() {
        let graph = build(name, &blocks);
        diags = graph.validate();
        if diags.is_empty() {
            return Ok(graph);
        }
    }
    diags.sort_by(|a, b| (a.pos, a.code).cmp(&(b.pos, b.code)));
    Err(diags)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Entity,
    Association,
}

struct Block<'t> {
    kind: Kind,
    name: &'t str,
    props: Vec<&'t str>,
    links: Vec<Link<'t>>,
    pos: Pos,
}

struct Link<'t> {
    target: &'t str,
    cardinality: &'t str,
    pos: Pos,
}

fn pos(line: usize, col: usize) -> Pos {
    Pos {
        line: u32::try_from(line).unwrap_or(u32::MAX),
        col: u32::try_from(col).unwrap_or(u32::MAX),
    }
}

fn parse<'t>(text: &'t str, diags: &mut Vec<Diagnostic>) -> Vec<Block<'t>> {
    let mut blocks: Vec<Block> = Vec::new();
    for (index, raw) in text.split('\n').enumerate() {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let line = index + 1;
        if raw.trim().is_empty() || raw.starts_with('#') {
            continue;
        }

        if !raw.starts_with(char::is_whitespace) {
            let (word, rest) = raw.split_once(char::is_whitespace).unwrap_or((raw, ""));
            let name = rest.trim();
            let kind = match word {
                "entite" => Kind::Entity,
                "association" => Kind::Association,
                _ => {
                    let message = Message::new("MCD002").text("word", word);
                    diags.push(Diagnostic::new(pos(line, 1), message));
                    continue;
                }
            };
            if name.is_empty() {
                let message = Message::new("MCD002.name").text("word", word);
                diags.push(Diagnostic::new(pos(line, 1), message));
                continue;
            }
            blocks.push(Block {
                kind,
                name,
                props: Vec::new(),
                links: Vec::new(),
                pos: pos(line, 1),
            });
            continue;
        }

        let item = raw.trim();
        let at = pos(
            line,
            raw.chars().take_while(|c| c.is_whitespace()).count() + 1,
        );
        let Some(block) = blocks.last_mut() else {
            diags.push(Diagnostic::new(at, Message::new("MCD003")));
            continue;
        };
        let parts: Vec<&str> = item.split_whitespace().collect();
        match parts[..] {
            [target, cardinality] if CARDINALITIES.contains(&cardinality) => {
                if block.kind == Kind::Entity {
                    let message = Message::new("MCD004").text("name", block.name);
                    diags.push(Diagnostic::new(at, message));
                } else {
                    block.links.push(Link {
                        target,
                        cardinality,
                        pos: at,
                    });
                }
            }
            [prop] => block.props.push(prop),
            _ => {
                let message = Message::new("MCD005").text("text", item);
                diags.push(Diagnostic::new(at, message));
            }
        }
    }
    blocks
}

/// Refuses what would draw wrongly without saying anything.
fn check(blocks: &[Block], diags: &mut Vec<Diagnostic>) {
    for (index, block) in blocks.iter().enumerate() {
        if let Some(first) = blocks[..index].iter().find(|b| b.name == block.name) {
            let message = Message::new("MCD006")
                .text("name", block.name)
                .text("line", first.pos.line);
            diags.push(Diagnostic::new(block.pos, message));
        }
        for link in &block.links {
            let entity = blocks
                .iter()
                .any(|b| b.kind == Kind::Entity && b.name == link.target);
            if !entity {
                let message = Message::new("MCD007").text("name", link.target);
                diags.push(Diagnostic::new(link.pos, message));
            }
        }
        if block.kind == Kind::Association && block.links.len() < 2 {
            let message = Message::new("MCD008")
                .text("name", block.name)
                .text("count", block.links.len());
            diags.push(Diagnostic::new(block.pos, message));
        }
    }
}

fn build(name: &str, blocks: &[Block]) -> Graph {
    let nodes = blocks
        .iter()
        .map(|block| {
            let mut lines = vec![block.name];
            lines.extend(block.props.iter().map(|p| p.strip_prefix('#').unwrap_or(p)));
            Node {
                id: block.name.to_string(),
                size: Some(text_box(&lines)),
                layer: None,
                order: None,
                padding: None,
                ports_ordered: false,
                ports: Vec::new(),
                children: Vec::new(),
                pos: block.pos,
            }
        })
        .collect();

    let mut edges = Vec::new();
    for block in blocks.iter().filter(|b| b.kind == Kind::Association) {
        for (index, link) in block.links.iter().enumerate() {
            // A reflexive association links the same entity twice: the
            // second occurrence takes the suffix `.2`, and so on.
            let rank = block.links[..=index]
                .iter()
                .filter(|l| l.target == link.target)
                .count();
            let id = if rank == 1 {
                format!("{}.{}", block.name, link.target)
            } else {
                format!("{}.{}.{rank}", block.name, link.target)
            };
            edges.push(Edge {
                id,
                source: link.target.to_string(),
                target: block.name.to_string(),
                source_port: None,
                target_port: None,
                source_side: None,
                target_side: None,
                labels: vec![label(link.cardinality)],
                pos: link.pos,
            });
        }
    }

    Graph {
        name: name.to_string(),
        direction: Some(Direction::Right),
        nodes,
        edges,
        pos: Pos { line: 1, col: 1 },
    }
}

/// A node's box: the longest line in code points times
/// [`ADVANCE`], the number of lines times [`LINE`], and [`MARGIN`] all
/// around. Provisional rule, to be replaced by measuring the text with the
/// embedded font.
pub fn text_box(lines: &[&str]) -> Size {
    let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let widest = i64::try_from(widest).unwrap_or(i64::MAX / ADVANCE);
    let count = i64::try_from(lines.len()).unwrap_or(i64::MAX / LINE);
    Size {
        width: widest * ADVANCE + 2 * MARGIN,
        height: count * LINE + 2 * MARGIN,
    }
}

/// A cardinality, against the entity: the source of the link.
fn label(cardinality: &str) -> Label {
    let chars = i64::try_from(cardinality.chars().count()).unwrap_or(0);
    Label {
        width: chars * ADVANCE + 2 * LABEL_MARGIN,
        height: LINE,
        at: Anchor::Source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refusals(text: &str) -> Vec<(&'static str, u32, u32)> {
        import("m", text.as_bytes())
            .unwrap_err()
            .iter()
            .map(|d| (d.code, d.pos.line, d.pos.col))
            .collect()
    }

    fn texts(text: &str) -> Vec<String> {
        import("m", text.as_bytes())
            .unwrap_err()
            .iter()
            .map(Diagnostic::to_string)
            .collect()
    }

    const GESTPRET: &str = "# MCD de GestPret\n\
        entite MEMBRE\n  #id_membre\n  nom\n\n\
        entite MATERIEL\n  #id_materiel\n\n\
        association PRET\n  date_pret\n  MEMBRE 0,n\n  MATERIEL 1,1\n";

    #[test]
    fn entities_and_associations_become_nodes_and_links_edges() {
        let graph = import("gestpret", GESTPRET.as_bytes()).unwrap();
        assert_eq!(graph.name, "gestpret");
        assert_eq!(graph.direction, Some(Direction::Right));
        let nodes: Vec<(&str, Size, Pos)> = graph
            .nodes
            .iter()
            .map(|n| (n.id.as_str(), n.size.unwrap(), n.pos))
            .collect();
        let size = |width, height| Size { width, height };
        assert_eq!(
            nodes,
            [
                // “id_membre”, 9 code points without the `#`, on 3 lines.
                (
                    "MEMBRE",
                    size(9 * 700 + 1600, 3 * 1800 + 1600),
                    Pos { line: 2, col: 1 }
                ),
                (
                    "MATERIEL",
                    size(11 * 700 + 1600, 2 * 1800 + 1600),
                    Pos { line: 6, col: 1 }
                ),
                (
                    "PRET",
                    size(9 * 700 + 1600, 2 * 1800 + 1600),
                    Pos { line: 9, col: 1 }
                ),
            ]
        );
        let edges: Vec<(&str, &str, &str, Pos)> = graph
            .edges
            .iter()
            .map(|e| (e.id.as_str(), e.source.as_str(), e.target.as_str(), e.pos))
            .collect();
        assert_eq!(
            edges,
            [
                ("PRET.MEMBRE", "MEMBRE", "PRET", Pos { line: 11, col: 3 }),
                (
                    "PRET.MATERIEL",
                    "MATERIEL",
                    "PRET",
                    Pos { line: 12, col: 3 }
                ),
            ]
        );
        let label = &graph.edges[0].labels[0];
        assert_eq!(
            (label.width, label.height, label.at),
            (2500, 1800, Anchor::Source)
        );
    }

    #[test]
    fn a_reflexive_association_links_the_same_entity_twice() {
        let text = "entite EMPLOYE\n  #matricule\n\
            association ENCADRER\n  EMPLOYE 0,n\n  EMPLOYE 0,1\n";
        let graph = import("m", text.as_bytes()).unwrap();
        let edges: Vec<(&str, &str)> = graph
            .edges
            .iter()
            .map(|e| (e.id.as_str(), e.source.as_str()))
            .collect();
        assert_eq!(
            edges,
            [
                ("ENCADRER.EMPLOYE", "EMPLOYE"),
                ("ENCADRER.EMPLOYE.2", "EMPLOYE")
            ]
        );
    }

    #[test]
    fn comments_blank_lines_carriage_returns_and_tabs_are_accepted() {
        let text = "\u{feff}# c\r\nentite A  \r\n\t#id\r\n\r\nentite B\r\n\
            association R\r\n\tA 0,n\r\n\tB 1,n\r\n";
        let graph = import("m", text.as_bytes()).unwrap();
        assert_eq!(graph.nodes[0].id, "A");
        assert_eq!(graph.edges[1].pos, Pos { line: 8, col: 2 });
    }

    #[test]
    fn imported_graphs_come_out_canonical_and_identical() {
        let once = import("g", GESTPRET.as_bytes()).unwrap().to_json();
        let twice = import("g", GESTPRET.as_bytes()).unwrap().to_json();
        assert_eq!(once, twice);
        assert_eq!(Graph::parse(once.as_bytes()).unwrap().to_json(), once);
    }

    #[test]
    fn text_that_is_not_utf8_is_refused_where_it_breaks() {
        let diags = import("m", b"entite A\n  n\xffom\n").unwrap_err();
        assert_eq!(
            (diags[0].code, diags[0].pos),
            ("MCD001", Pos { line: 2, col: 4 })
        );
    }

    #[test]
    fn a_block_starts_with_entite_or_association_and_a_name() {
        assert_eq!(
            refusals("entity A\nentite\nentite B\n"),
            [("MCD002", 1, 1), ("MCD002", 2, 1)]
        );
        assert_eq!(
            texts("entity A\nentite\n"),
            [
                "1:1: error[MCD002]: `entity` starts no block: `entite NAME` or `association NAME` expected",
                "2:1: error[MCD002]: `entite` without a name",
            ]
        );
    }

    #[test]
    fn an_indented_line_needs_a_block() {
        assert_eq!(refusals("  nom\nentite A\n"), [("MCD003", 1, 3)]);
    }

    #[test]
    fn only_an_association_carries_links() {
        assert_eq!(
            refusals("entite A\nentite B\n  A 0,n\n"),
            [("MCD004", 3, 3)]
        );
        assert_eq!(
            texts("entite A\nentite B\n  A 0,n\n"),
            ["3:3: error[MCD004]: only an association carries links; `B` is an entity"]
        );
    }

    #[test]
    fn a_line_is_a_property_or_a_link() {
        let text = "entite A\nassociation R\n  A 0,n\n  A 2,n\n  A 0,n x\n  A 0,1\n";
        assert_eq!(refusals(text), [("MCD005", 4, 3), ("MCD005", 5, 3)]);
        assert_eq!(
            texts(text)[0],
            "4:3: error[MCD005]: `A 2,n`: a property, or an entity followed by a cardinality \
             (`0,1`, `0,n`, `1,1` or `1,n`) expected"
        );
    }

    #[test]
    fn a_block_is_declared_once() {
        let text = "entite A\nentite B\nassociation A\n  A 0,n\n  B 0,n\n";
        assert_eq!(refusals(text), [("MCD006", 3, 1)]);
        assert_eq!(
            texts(text),
            ["3:1: error[MCD006]: `A` already declared (line 1)"]
        );
    }

    #[test]
    fn a_link_goes_to_a_declared_entity() {
        let text = "entite A\nassociation R\n  A 0,n\n  Z 1,1\nassociation S\n  R 0,n\n  A 0,n\n";
        assert_eq!(refusals(text), [("MCD007", 4, 3), ("MCD007", 6, 3)]);
        assert_eq!(
            texts(text)[0],
            "4:3: error[MCD007]: `Z` is not a declared entity"
        );
    }

    #[test]
    fn an_association_has_two_links_at_least() {
        // With no link at all, or with only one.
        assert_eq!(
            refusals("entite A\nassociation R\nassociation S\n  A 0,n\n"),
            [("MCD008", 2, 1), ("MCD008", 3, 1)]
        );
        assert_eq!(
            texts("entite A\nassociation R\n  date\n  A 0,n\n"),
            ["2:1: error[MCD008]: association `R` needs two links at least, it has 1"]
        );
    }

    #[test]
    fn the_validator_refuses_the_rest_with_mcd_positions() {
        let long = "x".repeat(300);
        let text = format!("entite {long}\nentite B\nassociation R\n  {long} 0,n\n  B 0,n\n");
        let diags = import("m", text.as_bytes()).unwrap_err();
        assert!(diags.iter().all(|d| d.code.starts_with("SCH")), "{diags:?}");
        assert_eq!(diags[0].pos, Pos { line: 1, col: 1 });
    }
}
