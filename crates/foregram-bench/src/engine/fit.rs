// ForegramPM — measurement bench: pulls route ends back onto their edge when an
// engine leaves them a hair away from their node.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The format requires a route to start at the exact edge of its node, at the
// exact anchor of its port. An engine computes in floats and `dot` cuts its
// curves by approximation: the end lands a few hundredths from the edge, and
// the validator would refuse everything. An end within one pixel of its target is
// therefore projected onto it, and every move is counted: the bench reports it, it does not
// hide it. Beyond that, the end stays where the engine put it and the validator
// flags it.

use foregram_graph::{Graph, Layout, Point, Rect, Segment, Side};

use super::absolute_rects;

/// Distance beyond which a route end is no longer pulled back, in
/// hundredths of a pixel: one and a half pixels. Measured on the corpora: `dot`
/// leaves its ends at 55 hundredths at most, and attaches a compass point
/// exactly 1 pt outside the node; the extra half pixel is the margin.
pub const TOLERANCE: i64 = 150;

/// What the fit moved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Route ends moved.
    pub moved: usize,
    /// The largest move, in hundredths of a pixel, by the larger
    /// of the two coordinates.
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

/// The point on the outline of `rect` — on its side `side`, if given — closest
/// to `p`.
pub fn onto_border(rect: Rect, side: Option<Side>, p: Point) -> Point {
    Target::Border(rect, side).nearest(p)
}

/// What a route end must touch.
enum Target {
    /// The anchor of a port, exactly.
    Anchor(Point),
    /// The box of a port, centred on its anchor: doubled gaps allowed
    /// up to its size.
    PortBox {
        anchor: Point,
        width: i64,
        height: i64,
    },
    /// The outline of a node, or one of its sides.
    Border(Rect, Option<Side>),
}

impl Target {
    /// The point of the target closest to `p`.
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
                        // The point pulled into the rectangle, then pushed towards the
                        // nearest side if it is inside.
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

/// Pulls route ends back onto their target when they are within [`TOLERANCE`] at
/// most, and reports how many moved.
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

/// The placed anchors of a node's ports, relative to the node.
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
        // e1 leaves from port `req`, a point: its start slides by 7 hundredths.
        // e2 arrives on the west side of Store: its arrival sinks by 100.
        // e3 arrives on the edge of System (curve): 151 hundredths, too far.
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
        // e2 leaves from port `out`, box 600 × 600: a point inside does not move,
        // a point beside it is pulled into the box.
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
