// ForegramPM — banc de mesure : ramène sur leur bord les bouts de tracé qu'un
// moteur laisse à un cheveu de leur nœud.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Le format veut qu'un tracé parte du bord exact de son nœud, de l'ancre
// exacte de son port. Un moteur calcule en flottants et `dot` coupe ses
// courbes par approximation : le bout tombe à quelques centièmes du bord, et
// le validateur refuserait tout. Un bout à moins d'un pixel de sa cible y est
// donc projeté, et chaque déplacement est compté : le banc le dit, il ne le
// cache pas. Au-delà, le bout reste où le moteur l'a mis et le validateur le
// relève.

use foregram_graph::{Graph, Layout, Point, Rect, Segment, Side};

use super::absolute_rects;

/// Distance au-delà de laquelle un bout de tracé n'est plus ramené, en
/// centièmes de pixel : un pixel et demi. Mesuré sur les corpus : `dot`
/// laisse ses bouts à 55 centièmes au plus, et attache un point cardinal
/// exactement 1 pt hors du nœud ; le demi-pixel de plus est la marge.
pub const TOLERANCE: i64 = 150;

/// Ce que l'ajustement a déplacé.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Bouts de tracé déplacés.
    pub moved: usize,
    /// Le plus grand déplacement, en centièmes de pixel, par la plus grande
    /// des deux coordonnées.
    pub max: i64,
}

impl Stats {
    fn record(&mut self, from: Point, to: Point) {
        if from != to {
            self.moved += 1;
            self.max = self
                .max
                .max((from.x - to.x).abs().max((from.y - to.y).abs()));
        }
    }
}

/// Le point du contour de `rect` — de son côté `side`, s'il est donné — le
/// plus proche de `p`.
pub fn onto_border(rect: Rect, side: Option<Side>, p: Point) -> Point {
    Target::Border(rect, side).nearest(p)
}

/// Ce qu'un bout de tracé doit toucher.
enum Target {
    /// L'ancre d'un port, exactement.
    Anchor(Point),
    /// La boîte d'un port, centrée sur son ancre : écarts doublés admis
    /// jusqu'à la taille.
    PortBox {
        anchor: Point,
        width: i64,
        height: i64,
    },
    /// Le contour d'un nœud, ou l'un de ses côtés.
    Border(Rect, Option<Side>),
}

impl Target {
    /// Le point de la cible le plus proche de `p`.
    fn nearest(&self, p: Point) -> Point {
        match *self {
            Target::Anchor(anchor) => anchor,
            Target::PortBox {
                anchor,
                width,
                height,
            } => Point {
                x: anchor.x + (p.x - anchor.x).clamp(-(width / 2), width / 2),
                y: anchor.y + (p.y - anchor.y).clamp(-(height / 2), height / 2),
            },
            Target::Border(rect, side) => {
                let (right, bottom) = (rect.x + rect.width, rect.y + rect.height);
                let cx = p.x.clamp(rect.x, right);
                let cy = p.y.clamp(rect.y, bottom);
                match side {
                    Some(Side::West) => Point { x: rect.x, y: cy },
                    Some(Side::East) => Point { x: right, y: cy },
                    Some(Side::North) => Point { x: cx, y: rect.y },
                    Some(Side::South) => Point { x: cx, y: bottom },
                    None => {
                        // Le point ramené dans le rectangle, puis poussé vers le
                        // côté le plus proche s'il est à l'intérieur.
                        let to_sides = [
                            (cx - rect.x, Point { x: rect.x, y: cy }),
                            (right - cx, Point { x: right, y: cy }),
                            (cy - rect.y, Point { x: cx, y: rect.y }),
                            (bottom - cy, Point { x: cx, y: bottom }),
                        ];
                        to_sides
                            .into_iter()
                            .min_by_key(|(distance, _)| *distance)
                            .map_or(p, |(_, point)| point)
                    }
                }
            }
        }
    }
}

/// Ramène sur leur cible les bouts de tracé qui en sont à [`TOLERANCE`] au
/// plus, et dit combien ont bougé.
pub fn fit(graph: &Graph, layout: &mut Layout) -> Stats {
    let rects = absolute_rects(graph, layout);
    let mut stats = Stats::default();
    for place in &mut layout.edges {
        let Some(edge) = graph.edges.iter().find(|e| e.id == place.id) else {
            continue;
        };
        let ends = [
            (true, &edge.source, &edge.source_port, edge.source_side),
            (false, &edge.target, &edge.target_port, edge.target_side),
        ];
        for (at_start, node_id, port_id, side) in ends {
            let Some(rect) = rects.get(node_id) else {
                continue;
            };
            let Some(target) = target(
                graph,
                layout_ports(&layout.nodes, node_id),
                *rect,
                port_id,
                side,
            ) else {
                continue;
            };
            let segment = if at_start {
                place.route.first_mut()
            } else {
                place.route.last_mut()
            };
            let Some(segment) = segment else {
                continue;
            };
            let from = segment.end();
            let to = target.nearest(from);
            if (from.x - to.x).abs() <= TOLERANCE && (from.y - to.y).abs() <= TOLERANCE {
                stats.record(from, to);
                match segment {
                    Segment::Line(p) => *p = to,
                    Segment::Cubic { end, .. } => *end = to,
                }
            }
        }
    }
    stats
}

/// Les ancres placées des ports d'un nœud, relatives au nœud.
fn layout_ports<'a>(
    nodes: &'a [foregram_graph::NodePlace],
    node_id: &str,
) -> &'a [foregram_graph::PortPlace] {
    nodes
        .iter()
        .find(|n| n.id == node_id)
        .map_or(&[], |n| n.ports.as_slice())
}

fn target(
    graph: &Graph,
    ports: &[foregram_graph::PortPlace],
    rect: Rect,
    port_id: &Option<String>,
    side: Option<Side>,
) -> Option<Target> {
    let Some(port_id) = port_id else {
        return Some(Target::Border(rect, side));
    };
    let placed = ports.iter().find(|p| p.id == *port_id)?;
    let anchor = Point {
        x: rect.x + placed.x,
        y: rect.y + placed.y,
    };
    let (_, port) = graph.port(port_id)?;
    Some(match port.size {
        None => Target::Anchor(anchor),
        Some(size) => Target::PortBox {
            anchor,
            width: size.width,
            height: size.height,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use foregram_graph::Layout;

    fn components() -> (Graph, Layout) {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../foregram-graph/tests/data/");
        let graph = std::fs::read(format!("{dir}components.graph.json")).unwrap();
        let layout = std::fs::read(format!("{dir}components.layout.json")).unwrap();
        (
            Graph::parse(&graph).unwrap(),
            Layout::parse(&layout).unwrap(),
        )
    }

    fn start(layout: &Layout, id: &str) -> Point {
        let edge = layout.edges.iter().find(|e| e.id == id).unwrap();
        edge.route.first().unwrap().end()
    }

    fn end(layout: &Layout, id: &str) -> Point {
        let edge = layout.edges.iter().find(|e| e.id == id).unwrap();
        edge.route.last().unwrap().end()
    }

    #[test]
    fn an_exact_layout_is_left_alone() {
        let (graph, mut layout) = components();
        let before = layout.to_json();
        assert_eq!(fit(&graph, &mut layout), Stats::default());
        assert_eq!(layout.to_json(), before);
    }

    #[test]
    fn ends_within_a_pixel_are_brought_back_and_counted() {
        let (graph, mut layout) = components();
        // e1 part du port `req`, un point : son départ glisse de 7 centièmes.
        // e2 arrive sur le côté ouest de Store : son arrivée s'enfonce de 100.
        // e3 arrive sur le bord de System (courbe) : 151 centièmes, trop loin.
        for edge in &mut layout.edges {
            match edge.id.as_str() {
                "e1" => edge.route[0] = Segment::Line(Point { x: 10007, y: 4995 }),
                "e2" => edge.route[1] = Segment::Line(Point { x: 35100, y: 4100 }),
                "e3" => {
                    edge.route[1] = Segment::Cubic {
                        c1: Point { x: 39000, y: 8500 },
                        c2: Point { x: 40000, y: 9000 },
                        end: Point { x: 41000, y: 10151 },
                    }
                }
                _ => {}
            }
        }
        let stats = fit(&graph, &mut layout);
        assert_eq!(stats, Stats { moved: 2, max: 100 });
        assert_eq!(start(&layout, "e1"), Point { x: 10000, y: 5000 });
        assert_eq!(end(&layout, "e2"), Point { x: 35000, y: 4100 });
        assert_eq!(end(&layout, "e3"), Point { x: 41000, y: 10151 });
        assert!(!layout.check(&graph).is_empty());
    }

    #[test]
    fn a_port_box_keeps_an_end_that_is_inside_it() {
        let (graph, mut layout) = components();
        // e2 part du port `out`, boîte 600 × 600 : un point dedans ne bouge pas,
        // un point à côté est ramené dans la boîte.
        let out = layout.edges.iter_mut().find(|e| e.id == "e2").unwrap();
        out.route[0] = Segment::Line(Point { x: 29200, y: 4200 });
        assert_eq!(fit(&graph, &mut layout), Stats::default());
        let out = layout.edges.iter_mut().find(|e| e.id == "e2").unwrap();
        out.route[0] = Segment::Line(Point { x: 29350, y: 4000 });
        assert_eq!(fit(&graph, &mut layout), Stats { moved: 1, max: 50 });
        assert_eq!(start(&layout, "e2"), Point { x: 29300, y: 4000 });
    }

    #[test]
    fn a_free_end_goes_to_the_nearest_side() {
        let rect = Rect {
            x: 0,
            y: 0,
            width: 1000,
            height: 400,
        };
        let border = Target::Border(rect, None);
        assert_eq!(
            border.nearest(Point { x: 500, y: 30 }),
            Point { x: 500, y: 0 }
        );
        assert_eq!(
            border.nearest(Point { x: 990, y: 200 }),
            Point { x: 1000, y: 200 }
        );
        assert_eq!(border.nearest(Point { x: -5, y: -5 }), Point { x: 0, y: 0 });
        let south = Target::Border(rect, Some(Side::South));
        assert_eq!(
            south.nearest(Point { x: 1200, y: 390 }),
            Point { x: 1000, y: 400 }
        );
    }
}
