// ForegramPM — layout metrics: what the bench compares between engines.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use crate::graph::{Direction, Edge, Graph, Node, Point, Rect, Side};
use crate::layout::{EdgePlace, Layout, Segment};
use crate::msg::Message;

/// Number of steps used to flatten a Bézier curve: every measure is taken on
/// the resulting polyline.
pub const CURVE_STEPS: i64 = 16;

/// The figures of a layout checked against its graph. All integers; lengths and
/// areas in hundredths of a pixel, ratios in thousandths.
///
/// The layout must pass [`Layout::check`]: a missing or extra element is not
/// measured, and does not make the measure fail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Pairs of segments from two distinct edges that cross at a point interior
    /// to both routes.
    pub crossings: u64,
    /// Pairs of collinear segments from two distinct edges that share a stretch
    /// of non-zero length.
    pub shared_segments: u64,
    /// Total length of these shared stretches.
    pub shared_length: i64,
    /// Vertices between two straight segments where the direction changes.
    pub bends: u64,
    /// Total length of all routes.
    pub edge_length: i64,
    /// Length of the longest route.
    pub longest_edge: i64,
    /// Pairs of nodes whose rectangles overlap, parent-child excluded.
    pub node_overlaps: u64,
    /// Total area of these overlaps.
    pub node_overlap_area: i64,
    /// Edge-node pairs where the route enters a node foreign to the edge.
    pub edge_node_overlaps: u64,
    /// Labels that overlap a node other than a container of their edge, another
    /// label or an edge other than their own, one per pair.
    pub label_overlaps: u64,
    /// Width of the bounding box of everything drawn.
    pub width: i64,
    /// Height of that bounding box.
    pub height: i64,
    /// Area of that bounding box, brought back to `i64::MAX` if it does not
    /// fit.
    pub area: i64,
    /// Width over height, in thousandths; 0 without height.
    pub proportion: i64,
    /// Leaf area over bounding box area, in thousandths; 0 without area.
    pub density: i64,
    /// Pairs of nodes whose imposed `layer` and whose positions along the
    /// direction are not in the same order.
    pub layer_violations: u64,
    /// Pairs of nodes in the same layer whose imposed `order` and whose
    /// positions across the direction are not in the same order.
    pub order_violations: u64,
    /// Pairs of ports on the same side, under `ports_ordered`, that do not
    /// follow each other clockwise.
    pub port_order_violations: u64,
}

/// A closed box, borders included: two boxes that touch are not disjoint.
#[derive(Clone, Copy)]
struct Bounds {
    min: Point,
    max: Point,
}

impl Bounds {
    fn of(points: &[Point]) -> Bounds {
        let mut b = Bounds {
            min: Point {
                x: i64::MAX,
                y: i64::MAX,
            },
            max: Point {
                x: i64::MIN,
                y: i64::MIN,
            },
        };
        for p in points {
            b.min.x = b.min.x.min(p.x);
            b.min.y = b.min.y.min(p.y);
            b.max.x = b.max.x.max(p.x);
            b.max.y = b.max.y.max(p.y);
        }
        b
    }

    fn disjoint(&self, other: &Bounds) -> bool {
        self.max.x < other.min.x
            || other.max.x < self.min.x
            || self.max.y < other.min.y
            || other.max.y < self.min.y
    }
}

/// A route ready to be compared with the others: its vertices, its box, and the
/// box of each of its segments.
struct Path {
    points: Vec<Point>,
    bounds: Bounds,
    boxes: Vec<Bounds>,
}

impl Path {
    fn new(points: Vec<Point>) -> Path {
        let bounds = Bounds::of(&points);
        let boxes = points.windows(2).map(Bounds::of).collect();
        Path {
            points,
            bounds,
            boxes,
        }
    }
}

/// An edge as it is measured: its flattened route, and what exempts it from
/// overlaps.
struct Measured<'a> {
    /// The vertices, with no repeated point.
    points: Vec<Point>,
    /// For each vertex, true if it joins two straight segments.
    corner: Vec<bool>,
    /// The nodes the edge may touch: its two ends and their ancestors.
    exempt: Vec<&'a str>,
    /// The nodes enclosing the edge: the ancestors of its ends, which its
    /// labels may overlap.
    inside: Vec<&'a str>,
    /// The rectangles of its labels.
    labels: Vec<Rect>,
}

impl Metrics {
    /// Measures a layout valid for its graph.
    pub fn measure<'a>(graph: &'a Graph, layout: &Layout) -> Metrics {
        let all = graph.all_nodes();
        let rects = layout.absolute_rects(graph);
        let parents: BTreeMap<&str, &str> = all
            .iter()
            .filter_map(|(node, parent)| parent.map(|p| (node.id.as_str(), p.id.as_str())))
            .collect();
        let ancestors = |id: &'a str| -> Vec<&'a str> {
            let mut out = vec![id];
            let mut at = id;
            while let Some(parent) = parents.get(at) {
                out.push(*parent);
                at = parent;
            }
            out
        };
        let edges: Vec<Measured<'_>> = graph
            .edges
            .iter()
            .filter_map(|edge| {
                let place = layout.edges.iter().find(|e| e.id == edge.id)?;
                let (points, corner) = flatten(&place.route);
                let mut exempt = ancestors(&edge.source);
                exempt.extend(ancestors(&edge.target));
                let inside = exempt
                    .iter()
                    .copied()
                    .filter(|id| *id != edge.source && *id != edge.target)
                    .collect();
                let labels = label_rects(edge, place);
                Some(Measured {
                    points,
                    corner,
                    exempt,
                    inside,
                    labels,
                })
            })
            .collect();

        let mut m = Metrics::default();
        m.measure_edges(&edges);
        m.measure_nodes(&all, &rects, &parents);
        m.measure_edge_nodes(&edges, &rects);
        m.measure_labels(&edges, &rects);
        m.measure_extent(&all, &rects, &edges);
        m.measure_constraints(graph, &all, &rects, layout);
        m
    }

    /// The report lines, one per measure, in a fixed order.
    pub fn report(&self) -> Vec<Message> {
        let n = |key, value: u64| Message::new(key).text("n", value);
        vec![
            Message::new("metrics.crossings").text("n", self.crossings),
            Message::new("metrics.shared")
                .text("n", self.shared_segments)
                .text("length", self.shared_length),
            n("metrics.bends", self.bends),
            Message::new("metrics.edge-length")
                .text("total", self.edge_length)
                .text("longest", self.longest_edge),
            Message::new("metrics.node-overlaps")
                .text("n", self.node_overlaps)
                .text("area", self.node_overlap_area),
            n("metrics.edge-node-overlaps", self.edge_node_overlaps),
            n("metrics.label-overlaps", self.label_overlaps),
            Message::new("metrics.extent")
                .text("width", self.width)
                .text("height", self.height)
                .text("area", self.area)
                .text("proportion", self.proportion)
                .text("density", self.density),
            Message::new("metrics.constraints")
                .text("layer", self.layer_violations)
                .text("order", self.order_violations)
                .text("port", self.port_order_violations),
        ]
    }

    /// The measures under their name, in the order of the struct: what the JSON
    /// writes and what the bench aggregates. A count that does not fit in an
    /// `i64` is brought back to `i64::MAX`.
    pub fn fields(&self) -> [(&'static str, i64); 18] {
        let n = |value: u64| i64::try_from(value).unwrap_or(i64::MAX);
        [
            ("crossings", n(self.crossings)),
            ("shared_segments", n(self.shared_segments)),
            ("shared_length", self.shared_length),
            ("bends", n(self.bends)),
            ("edge_length", self.edge_length),
            ("longest_edge", self.longest_edge),
            ("node_overlaps", n(self.node_overlaps)),
            ("node_overlap_area", self.node_overlap_area),
            ("edge_node_overlaps", n(self.edge_node_overlaps)),
            ("label_overlaps", n(self.label_overlaps)),
            ("width", self.width),
            ("height", self.height),
            ("area", self.area),
            ("proportion", self.proportion),
            ("density", self.density),
            ("layer_violations", n(self.layer_violations)),
            ("order_violations", n(self.order_violations)),
            ("port_order_violations", n(self.port_order_violations)),
        ]
    }

    /// The measures as a one-line JSON object, one key per measure, in the
    /// order of [`Metrics::fields`].
    pub fn to_json(&self) -> String {
        let fields: Vec<String> = self
            .fields()
            .iter()
            .map(|(name, value)| format!("\"{name}\": {value}"))
            .collect();
        format!("{{{}}}", fields.join(", "))
    }

    fn measure_edges(&mut self, edges: &[Measured<'_>]) {
        for edge in edges {
            let mut length: i64 = 0;
            for pair in edge.points.windows(2) {
                length = length.saturating_add(distance(pair[0], pair[1]));
            }
            self.edge_length = self.edge_length.saturating_add(length);
            self.longest_edge = self.longest_edge.max(length);
            for i in 1..edge.points.len().saturating_sub(1) {
                let before = sub(edge.points[i], edge.points[i - 1]);
                let after = sub(edge.points[i + 1], edge.points[i]);
                if edge.corner[i] && !same_direction(before, after) {
                    self.bends += 1;
                }
            }
        }
        // Edge pairs: crossings and shared stretches, each pair once.
        let paths: Vec<Path> = edges
            .iter()
            .map(|e| Path::new(merge_collinear(&e.points)))
            .collect();
        for (i, p) in paths.iter().enumerate() {
            for q in &paths[i + 1..] {
                // Two routes whose boxes are disjoint touch nowhere: nothing to
                // count, and nothing to compute.
                if p.bounds.disjoint(&q.bounds) {
                    continue;
                }
                self.pair(p, q);
            }
        }
    }

    /// The crossings and shared stretches of two routes.
    fn pair(&mut self, p: &Path, q: &Path) {
        for (a, ba) in p.points.windows(2).zip(&p.boxes) {
            for (b, bb) in q.points.windows(2).zip(&q.boxes) {
                // Same reason, segment by segment: `meet` would find nothing.
                if ba.disjoint(bb) {
                    continue;
                }
                match meet(a[0], a[1], b[0], b[1]) {
                    Meet::Proper => self.crossings += 1,
                    Meet::Shared(length) => {
                        self.shared_segments += 1;
                        self.shared_length = self.shared_length.saturating_add(length);
                    }
                    Meet::None | Meet::Touch => {}
                }
            }
        }
        // A route that passes through a vertex of the other: the pair of
        // segments does not see it, the vertex tells. A vertex common to both
        // is judged once, on the side of `q`.
        let (p, q) = (&p.points, &q.points);
        for i in 1..q.len().saturating_sub(1) {
            if passes_through(q[i - 1], q[i], q[i + 1], p) {
                self.crossings += 1;
            }
        }
        for i in 1..p.len().saturating_sub(1) {
            if !q.contains(&p[i]) && passes_through(p[i - 1], p[i], p[i + 1], q) {
                self.crossings += 1;
            }
        }
    }

    fn measure_nodes(
        &mut self,
        all: &[(&Node, Option<&Node>)],
        rects: &BTreeMap<&str, Rect>,
        parents: &BTreeMap<&str, &str>,
    ) {
        let related = |a: &str, b: &str| {
            let mut at = a;
            while let Some(parent) = parents.get(at) {
                if *parent == b {
                    return true;
                }
                at = parent;
            }
            false
        };
        for (i, (a, _)) in all.iter().enumerate() {
            let Some(ra) = rects.get(a.id.as_str()) else {
                continue;
            };
            for (b, _) in &all[i + 1..] {
                let Some(rb) = rects.get(b.id.as_str()) else {
                    continue;
                };
                if related(&a.id, &b.id) || related(&b.id, &a.id) {
                    continue;
                }
                let area = overlap_area(*ra, *rb);
                if area > 0 {
                    self.node_overlaps += 1;
                    self.node_overlap_area = self.node_overlap_area.saturating_add(area);
                }
            }
        }
    }

    fn measure_edge_nodes(&mut self, edges: &[Measured<'_>], rects: &BTreeMap<&str, Rect>) {
        for edge in edges {
            for (id, rect) in rects {
                if edge.exempt.contains(id) {
                    continue;
                }
                if enters(&edge.points, *rect) {
                    self.edge_node_overlaps += 1;
                }
            }
        }
    }

    fn measure_labels(&mut self, edges: &[Measured<'_>], rects: &BTreeMap<&str, Rect>) {
        let labels: Vec<(usize, Rect)> = edges
            .iter()
            .enumerate()
            .flat_map(|(i, e)| e.labels.iter().map(move |r| (i, *r)))
            .collect();
        for (k, (owner, label)) in labels.iter().enumerate() {
            for (id, rect) in rects {
                if !edges[*owner].inside.contains(id) && overlap_area(*label, *rect) > 0 {
                    self.label_overlaps += 1;
                }
            }
            for (_, other) in &labels[k + 1..] {
                if overlap_area(*label, *other) > 0 {
                    self.label_overlaps += 1;
                }
            }
            for (i, edge) in edges.iter().enumerate() {
                if i != *owner && enters(&edge.points, *label) {
                    self.label_overlaps += 1;
                }
            }
        }
    }

    fn measure_extent(
        &mut self,
        all: &[(&Node, Option<&Node>)],
        rects: &BTreeMap<&str, Rect>,
        edges: &[Measured<'_>],
    ) {
        let mut leaves: i128 = 0;
        for (node, _) in all {
            if let Some(rect) = rects.get(node.id.as_str())
                && !node.is_container()
            {
                leaves += i128::from(rect.width) * i128::from(rect.height);
            }
        }
        let bound = bounds(
            rects.values().copied(),
            edges.iter().flat_map(|e| e.points.iter().copied()),
            edges.iter().flat_map(|e| e.labels.iter().copied()),
        );
        let Some(bound) = bound else {
            return;
        };
        self.width = bound.width;
        self.height = bound.height;
        let area = i128::from(bound.width) * i128::from(bound.height);
        self.area = clamp(area);
        self.proportion = ratio(i128::from(bound.width), i128::from(bound.height));
        self.density = ratio(leaves, area);
    }

    fn measure_constraints(
        &mut self,
        graph: &Graph,
        all: &[(&Node, Option<&Node>)],
        rects: &BTreeMap<&str, Rect>,
        layout: &Layout,
    ) {
        let direction = graph.direction.unwrap_or(Direction::Right);
        // The center of a node, doubled to stay integer: along the direction,
        // then across.
        let along = |rect: &Rect| match direction {
            Direction::Right => 2 * rect.x + rect.width,
            Direction::Left => -(2 * rect.x + rect.width),
            Direction::Down => 2 * rect.y + rect.height,
            Direction::Up => -(2 * rect.y + rect.height),
        };
        let across = |rect: &Rect| match direction {
            Direction::Right | Direction::Left => 2 * rect.y + rect.height,
            Direction::Down | Direction::Up => 2 * rect.x + rect.width,
        };
        let layered: Vec<(&Node, &Rect)> = all
            .iter()
            .filter(|(node, _)| node.layer.is_some())
            .filter_map(|(node, _)| rects.get(node.id.as_str()).map(|r| (*node, r)))
            .collect();
        for (i, (a, ra)) in layered.iter().enumerate() {
            for (b, rb) in &layered[i + 1..] {
                let (la, lb) = (a.layer.unwrap_or(0), b.layer.unwrap_or(0));
                if la != lb {
                    let placed = along(ra).cmp(&along(rb));
                    if placed != la.cmp(&lb) {
                        self.layer_violations += 1;
                    }
                } else if let (Some(oa), Some(ob)) = (a.order, b.order) {
                    let placed = across(ra).cmp(&across(rb));
                    if placed != oa.cmp(&ob) {
                        self.order_violations += 1;
                    }
                }
            }
        }
        for (node, _) in all {
            if !node.ports_ordered {
                continue;
            }
            let Some(place) = layout.nodes.iter().find(|n| n.id == node.id) else {
                continue;
            };
            let anchors: Vec<(Side, i64)> = node
                .ports
                .iter()
                .filter_map(|port| {
                    let side = port.side?;
                    let at = place.ports.iter().find(|p| p.id == port.id)?;
                    Some((side, clockwise(side, at.x, at.y)))
                })
                .collect();
            for (i, (side, key)) in anchors.iter().enumerate() {
                for (other, next) in &anchors[i + 1..] {
                    if side == other && next <= key {
                        self.port_order_violations += 1;
                    }
                }
            }
        }
    }
}

/// The rectangles of an edge's labels, absolute, in the graph's order.
pub(crate) fn label_rects(edge: &Edge, place: &EdgePlace) -> Vec<Rect> {
    edge.labels
        .iter()
        .zip(&place.labels)
        .map(|(label, at)| Rect {
            x: at.x,
            y: at.y,
            width: label.width,
            height: label.height,
        })
        .collect()
}

/// The bounding box of rectangles and points; none if there is nothing.
fn bounds(
    rects: impl Iterator<Item = Rect>,
    points: impl Iterator<Item = Point>,
    labels: impl Iterator<Item = Rect>,
) -> Option<Rect> {
    let mut bound: Option<Rect> = None;
    let dots = points.map(|p| Rect {
        x: p.x,
        y: p.y,
        width: 0,
        height: 0,
    });
    for rect in rects.chain(dots).chain(labels) {
        bound = Some(match bound {
            None => rect,
            Some(b) => {
                let x = b.x.min(rect.x);
                let y = b.y.min(rect.y);
                Rect {
                    x,
                    y,
                    width: (b.x + b.width).max(rect.x + rect.width) - x,
                    height: (b.y + b.height).max(rect.y + rect.height) - y,
                }
            }
        });
    }
    bound
}

/// The bounding box of everything drawn: nodes, vertices of the flattened
/// routes, labels. The same as the one of the measures. None for an empty
/// layout.
pub fn extent(graph: &Graph, layout: &Layout) -> Option<Rect> {
    let rects = layout.absolute_rects(graph);
    let routed: Vec<(&Edge, &EdgePlace)> = graph
        .edges
        .iter()
        .filter_map(|edge| Some((edge, layout.edges.iter().find(|e| e.id == edge.id)?)))
        .collect();
    bounds(
        rects.values().copied(),
        routed.iter().flat_map(|(_, place)| polyline(&place.route)),
        routed
            .iter()
            .flat_map(|(edge, place)| label_rects(edge, place)),
    )
}

/// Flattens a route into a polyline: each curve becomes [`CURVE_STEPS`]
/// segments, repeated points are removed. This is the polyline the measures
/// read, and the one the renderer draws.
pub fn polyline(route: &[Segment]) -> Vec<Point> {
    flatten(route).0
}

/// Flattens a route into a polyline: each curve becomes [`CURVE_STEPS`]
/// segments, repeated points are removed. Also returns, for each vertex,
/// whether it joins two straight segments.
fn flatten(route: &[Segment]) -> (Vec<Point>, Vec<bool>) {
    let mut points: Vec<Point> = Vec::new();
    let mut corner: Vec<bool> = Vec::new();
    let push = |p: Point, from_line: bool, points: &mut Vec<Point>, corner: &mut Vec<bool>| {
        if points.last() == Some(&p) {
            // A repeated point makes no vertex; the junction takes the nature
            // of the incoming segment.
            if let Some(last) = corner.last_mut() {
                *last = *last && from_line;
            }
        } else {
            points.push(p);
            corner.push(from_line);
        }
    };
    for segment in route {
        match segment {
            Segment::Line(p) => push(*p, true, &mut points, &mut corner),
            Segment::Cubic { c1, c2, end } => {
                let Some(start) = points.last().copied() else {
                    continue;
                };
                for k in 1..=CURVE_STEPS {
                    let p = bezier(start, *c1, *c2, *end, k);
                    push(p, false, &mut points, &mut corner);
                }
            }
        }
    }
    // A vertex joins two straight segments if the incoming and the outgoing one
    // both are; `corner[i]` only tells about the incoming one yet.
    let n = points.len();
    let mut both = vec![false; n];
    for i in 1..n.saturating_sub(1) {
        both[i] = corner[i] && corner[i + 1];
    }
    (points, both)
}

/// The curve point at step `k` of [`CURVE_STEPS`], rounded to nearest.
fn bezier(p0: Point, p1: Point, p2: Point, p3: Point, k: i64) -> Point {
    let t = k;
    let u = CURVE_STEPS - k;
    let scale = CURVE_STEPS * CURVE_STEPS * CURVE_STEPS;
    let at = |a: i64, b: i64, c: i64, d: i64| {
        let sum = u * u * u * a + 3 * u * u * t * b + 3 * u * t * t * c + t * t * t * d;
        (sum + scale / 2).div_euclid(scale)
    };
    Point {
        x: at(p0.x, p1.x, p2.x, p3.x),
        y: at(p0.y, p1.y, p2.y, p3.y),
    }
}

/// Removes the vertices where the polyline goes straight on: a route where
/// every vertex is a real change of direction.
fn merge_collinear(points: &[Point]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    for &p in points {
        if out.len() >= 2 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            if same_direction(sub(b, a), sub(p, b)) {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

fn sub(a: Point, b: Point) -> Point {
    Point {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn cross(a: Point, b: Point) -> i128 {
    i128::from(a.x) * i128::from(b.y) - i128::from(a.y) * i128::from(b.x)
}

fn dot(a: Point, b: Point) -> i128 {
    i128::from(a.x) * i128::from(b.x) + i128::from(a.y) * i128::from(b.y)
}

/// True if both vectors are collinear and point the same way.
fn same_direction(a: Point, b: Point) -> bool {
    cross(a, b) == 0 && dot(a, b) > 0
}

/// Length of a segment, rounded to the nearest hundredth.
fn distance(a: Point, b: Point) -> i64 {
    let d = sub(b, a);
    let squared = dot(d, d).unsigned_abs();
    let root = squared.isqrt();
    // To nearest: beyond r² + r, r + 1 is the nearest.
    let rounded = if squared - root * root > root {
        root + 1
    } else {
        root
    };
    i64::try_from(rounded).unwrap_or(i64::MAX)
}

fn clamp(value: i128) -> i64 {
    value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

/// `num / den` in thousandths, rounded to nearest; 0 if `den` is zero.
fn ratio(num: i128, den: i128) -> i64 {
    if den <= 0 {
        return 0;
    }
    clamp((num * 1000 + den / 2).div_euclid(den))
}

/// What two segments have in common.
enum Meet {
    None,
    /// A single point, interior to both.
    Proper,
    /// A single point, an end of at least one.
    Touch,
    /// A common stretch, of this length.
    Shared(i64),
}

/// Which side of `ab` `p` is on: sign of the cross product.
fn orient(a: Point, b: Point, p: Point) -> i128 {
    cross(sub(b, a), sub(p, a)).signum()
}

fn meet(a: Point, b: Point, c: Point, d: Point) -> Meet {
    let (o1, o2) = (orient(a, b, c), orient(a, b, d));
    let (o3, o4) = (orient(c, d, a), orient(c, d, b));
    if o1 * o2 < 0 && o3 * o4 < 0 {
        return Meet::Proper;
    }
    if o1 == 0 && o2 == 0 {
        // Collinear: the common interval, measured along `ab` by the projection
        // of each end.
        let dir = sub(b, a);
        let key = |p: Point| dot(sub(p, a), dir);
        let (kc, kd) = (key(c), key(d));
        let (lo, hi) = (kc.min(kd).max(0), kc.max(kd).min(key(b)));
        if hi > lo {
            // The second and third points in order bound the stretch.
            let mut keyed = [(0, a), (key(b), b), (kc, c), (kd, d)];
            keyed.sort_by_key(|(k, _)| *k);
            return Meet::Shared(distance(keyed[1].1, keyed[2].1));
        }
        return if hi == lo { Meet::Touch } else { Meet::None };
    }
    let on = |o: i128, p: Point, s: Point, e: Point| o == 0 && between(p, s, e);
    if on(o1, c, a, b) || on(o2, d, a, b) || on(o3, a, c, d) || on(o4, b, c, d) {
        return Meet::Touch;
    }
    Meet::None
}

/// True if `p`, already on the line `se`, is between `s` and `e`, ends
/// included.
fn between(p: Point, s: Point, e: Point) -> bool {
    p.x >= s.x.min(e.x) && p.x <= s.x.max(e.x) && p.y >= s.y.min(e.y) && p.y <= s.y.max(e.y)
}

/// True if the route `prev → at → next` crosses the route `path` at `at`, a
/// vertex of one resting on the other: the two neighbours are on either side. A
/// neighbour resting on `path` is a shared stretch, not a crossing.
fn passes_through(prev: Point, at: Point, next: Point, path: &[Point]) -> bool {
    // The two rays of `path` at `at`: those of a segment if `at` is inside it,
    // those of the two neighbouring segments if `at` is a vertex.
    let rays: Option<(Point, Point)> = if let Some(i) = path.iter().position(|p| *p == at) {
        if i == 0 || i + 1 == path.len() {
            // An end of `path`: nothing continues on the other side.
            return false;
        }
        Some((sub(path[i - 1], at), sub(path[i + 1], at)))
    } else {
        path.windows(2)
            .find(|s| orient(s[0], s[1], at) == 0 && between(at, s[0], s[1]))
            .map(|s| (sub(s[0], at), sub(s[1], at)))
    };
    let Some((r1, r2)) = rays else {
        return false;
    };
    let (q1, q2) = (sub(prev, at), sub(next, at));
    if cross(r1, r2) == 0 {
        // `path` goes straight: crossing if the neighbours are strictly on
        // either side of the line.
        let (s1, s2) = (cross(r1, q1).signum(), cross(r1, q2).signum());
        return s1 * s2 < 0;
    }
    // `path` turns: crossing if one neighbour is inside the angle `r1 → r2` and
    // the other outside, none on a ray.
    let on_ray = |q: Point| same_direction(q, r1) || same_direction(q, r2);
    if on_ray(q1) || on_ray(q2) {
        return false;
    }
    in_wedge(r1, r2, q1) != in_wedge(r1, r2, q2)
}

/// True if `q` is strictly inside the angle swept from `r1` to `r2`
/// counterclockwise.
fn in_wedge(r1: Point, r2: Point, q: Point) -> bool {
    if cross(r1, r2) > 0 {
        cross(r1, q) > 0 && cross(q, r2) > 0
    } else {
        !(cross(r2, q) > 0 && cross(q, r1) > 0)
    }
}

/// True if a segment of the polyline enters the interior of the rectangle.
fn enters(points: &[Point], rect: Rect) -> bool {
    points.windows(2).any(|s| segment_enters(s[0], s[1], rect))
}

fn segment_enters(a: Point, b: Point, rect: Rect) -> bool {
    let (x1, x2) = (i128::from(rect.x), i128::from(rect.x + rect.width));
    let (y1, y2) = (i128::from(rect.y), i128::from(rect.y + rect.height));
    if x2 <= x1 || y2 <= y1 {
        return false;
    }
    // The interval of t in [0, 1] where the point a + t (b − a) is strictly
    // inside the rectangle, bounded by fractions `num / den` with den > 0.
    let mut lo: (i128, i128, bool) = (0, 1, false); // (num, den, strict)
    let mut hi: (i128, i128, bool) = (1, 1, false);
    let less = |p: (i128, i128, bool), q: (i128, i128, bool)| p.0 * q.1 < q.0 * p.1;
    for (p0, delta, min, max) in [
        (i128::from(a.x), i128::from(b.x - a.x), x1, x2),
        (i128::from(a.y), i128::from(b.y - a.y), y1, y2),
    ] {
        if delta == 0 {
            if p0 <= min || p0 >= max {
                return false;
            }
            continue;
        }
        let (mut low, mut high) = ((min - p0, delta), (max - p0, delta));
        if delta < 0 {
            low = (-low.0, -low.1);
            high = (-high.0, -high.1);
            std::mem::swap(&mut low, &mut high);
        }
        let low = (low.0, low.1, true);
        let high = (high.0, high.1, true);
        if less(lo, low) {
            lo = low;
        }
        if less(high, hi) {
            hi = high;
        }
    }
    less(lo, hi)
}

/// Area of the overlap of two rectangles; 0 if they only touch.
fn overlap_area(a: Rect, b: Rect) -> i64 {
    let w = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let h = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    if w > 0 && h > 0 {
        clamp(i128::from(w) * i128::from(h))
    } else {
        0
    }
}

/// Position of a port along its side, clockwise: by x on north, by y on east,
/// reversed on south and west.
fn clockwise(side: Side, x: i64, y: i64) -> i64 {
    match side {
        Side::North => x,
        Side::East => y,
        Side::South => -x,
        Side::West => -y,
    }
}
