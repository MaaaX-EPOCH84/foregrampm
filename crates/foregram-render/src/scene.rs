// ForegramPM — the scene: what is drawn, before choosing SVG or PNG.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use foregram_graph::{Graph, Layout, Point, Rect, Segment, extent, polyline};

/// The neutral look: greys and thicknesses, the same for all engines. Greys
/// run from 0 (black) to 255 (white); lengths are in hundredths of a pixel.
/// [`Style::default`] is the look of the bench.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    /// Margin between the drawing and the edge of the sheet.
    pub margin: i64,
    /// Background of the sheet.
    pub sheet: u8,
    /// Fill of a leaf of the graph (a node without children).
    pub leaf_fill: u8,
    /// Ink of solid strokes: border of leaves and ports, edges, arrowheads.
    pub ink: u8,
    /// Ink of a container, lighter so that the content stays readable.
    pub container_ink: u8,
    /// Width of thick strokes: border of leaves, edges.
    pub thick: i64,
    /// Width of thin strokes: containers, labels, ports.
    pub thin: i64,
    /// Length of the arrowhead.
    pub arrow_length: i64,
    /// Width of the base of the arrowhead.
    pub arrow_width: i64,
    /// Side of the square of a port without a box.
    pub port_size: i64,
    /// Length of a dash in the outline of a label.
    pub dash: i64,
    /// Length of the gap between two dashes in the outline of a label.
    pub gap: i64,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            margin: 1000,
            sheet: 255,
            leaf_fill: 224,
            ink: 51,
            container_ink: 153,
            thick: 100,
            thin: 50,
            arrow_length: 800,
            arrow_width: 500,
            port_size: 400,
            dash: 300,
            gap: 200,
        }
    }
}

/// What an element is, hence how it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// The background sheet, which covers the view.
    Sheet,
    /// A node without children: fill and border.
    Leaf,
    /// A container: thin border only.
    Container,
    /// The route of an edge.
    Edge,
    /// The head of an edge, a filled triangle.
    Arrow,
    /// A label: a dashed rectangle, empty.
    Label,
    /// A port: a hollow square, or its box, filled with the sheet color.
    Port,
}

impl Class {
    /// The name of the class in the SVG.
    pub fn name(self) -> &'static str {
        match self {
            Class::Sheet => "sheet",
            Class::Leaf => "leaf",
            Class::Container => "container",
            Class::Edge => "edge",
            Class::Arrow => "arrow",
            Class::Label => "label",
            Class::Port => "port",
        }
    }
}

/// A shape, in absolute hundredths of a pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A rectangle.
    Rect(Rect),
    /// A route as the layout gives it, curves included.
    Route(Vec<Segment>),
    /// A filled polygon.
    Polygon(Vec<Point>),
}

/// A drawn element, with the identifier it has in the graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// What the item is, hence how it is drawn.
    pub class: Class,
    /// The geometry of the item.
    pub shape: Shape,
    /// Identifier in the graph, shown as a tooltip; `None` for the sheet,
    /// arrowheads and labels.
    pub title: Option<String>,
}

/// Everything that is drawn, in drawing order: the sheet, the nodes from the
/// top of the nesting down, the edges and their arrowheads, the labels, the
/// ports on top of everything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene {
    /// The graph name and the engine, for the title of the drawing.
    pub title: String,
    /// The view: the bounding box of the drawing, plus the margin.
    pub view: Rect,
    /// The items, in drawing order.
    pub items: Vec<Item>,
}

impl Scene {
    /// Builds the scene of a layout. What is not placed is not drawn; a layout
    /// that passes `Layout::check` is drawn in full.
    pub fn build(graph: &Graph, layout: &Layout, style: &Style) -> Scene {
        let drawn = extent(graph, layout).unwrap_or(Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        });
        let view = Rect {
            x: drawn.x - style.margin,
            y: drawn.y - style.margin,
            width: drawn.width + 2 * style.margin,
            height: drawn.height + 2 * style.margin,
        };
        let mut items = vec![Item {
            class: Class::Sheet,
            shape: Shape::Rect(view),
            title: None,
        }];
        let rects = layout.absolute_rects(graph);
        let mut ports = Vec::new();
        for (node, _) in graph.all_nodes() {
            let Some(rect) = rects.get(node.id.as_str()) else {
                continue;
            };
            let class = if node.is_container() {
                Class::Container
            } else {
                Class::Leaf
            };
            items.push(Item {
                class,
                shape: Shape::Rect(*rect),
                title: Some(node.id.clone()),
            });
            let Some(place) = layout.nodes.iter().find(|n| n.id == node.id) else {
                continue;
            };
            for port in &node.ports {
                let Some(at) = place.ports.iter().find(|p| p.id == port.id) else {
                    continue;
                };
                let (width, height) = match port.size {
                    Some(size) => (size.width, size.height),
                    None => (style.port_size, style.port_size),
                };
                ports.push(Item {
                    class: Class::Port,
                    shape: Shape::Rect(Rect {
                        x: rect.x + at.x - width / 2,
                        y: rect.y + at.y - height / 2,
                        width,
                        height,
                    }),
                    title: Some(port.id.clone()),
                });
            }
        }
        let mut labels = Vec::new();
        for edge in &graph.edges {
            let Some(place) = layout.edges.iter().find(|e| e.id == edge.id) else {
                continue;
            };
            items.push(Item {
                class: Class::Edge,
                shape: Shape::Route(place.route.clone()),
                title: Some(edge.id.clone()),
            });
            if let Some(head) = arrow(&polyline(&place.route), style) {
                items.push(Item {
                    class: Class::Arrow,
                    shape: Shape::Polygon(head),
                    title: None,
                });
            }
            for (label, at) in edge.labels.iter().zip(&place.labels) {
                labels.push(Item {
                    class: Class::Label,
                    shape: Shape::Rect(Rect {
                        x: at.x,
                        y: at.y,
                        width: label.width,
                        height: label.height,
                    }),
                    title: None,
                });
            }
        }
        items.extend(labels);
        items.extend(ports);
        Scene {
            title: format!(
                "{} · {} {}",
                graph.name, layout.engine.name, layout.engine.version
            ),
            view,
            items,
        }
    }
}

/// The triangle that ends a route: the tip at the end, the base set back in
/// the direction of the last segment of the polyline. None if the route has
/// no direction.
fn arrow(points: &[Point], style: &Style) -> Option<Vec<Point>> {
    let tip = *points.last()?;
    let from = *points.iter().rev().find(|p| **p != tip)?;
    let (dx, dy) = (tip.x - from.x, tip.y - from.y);
    let len = isqrt(i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy));
    if len == 0 {
        return None;
    }
    // A vector of length `l` in the direction (dx, dy), rounded.
    let along = |l: i64| {
        (
            div_round(i128::from(dx) * i128::from(l), len),
            div_round(i128::from(dy) * i128::from(l), len),
        )
    };
    let (bx, by) = along(style.arrow_length);
    let (px, py) = along(style.arrow_width / 2);
    let base = Point {
        x: tip.x - bx,
        y: tip.y - by,
    };
    Some(vec![
        tip,
        Point {
            x: base.x - py,
            y: base.y + px,
        },
        Point {
            x: base.x + py,
            y: base.y - px,
        },
    ])
}

/// Integer square root, rounded to nearest.
pub(crate) fn isqrt(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let mut x = n.isqrt();
    // `isqrt` truncates; we round to nearest.
    if (x + 1) * (x + 1) - n < n - x * x {
        x += 1;
    }
    x
}

/// Quotient rounded to nearest, the divisor being positive.
pub(crate) fn div_round(a: i128, b: i128) -> i64 {
    let q = (2 * a + b).div_euclid(2 * b);
    i64::try_from(q).unwrap_or(i64::MAX)
}
