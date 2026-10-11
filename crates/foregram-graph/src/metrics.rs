// ForegramPM — mesures d'un placement : ce que le banc compare entre moteurs.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use crate::graph::{Direction, Edge, Graph, Node, Point, Rect, Side};
use crate::layout::{EdgePlace, Layout, Segment};
use crate::msg::Message;

/// Nombre de pas de l'aplatissement d'une courbe de Bézier : toute mesure
/// se fait sur la polyligne obtenue.
pub const CURVE_STEPS: i64 = 16;

/// Les chiffres d'un placement confronté à son graphe. Tous entiers ; les
/// longueurs et les aires en centièmes de pixel, les rapports en millièmes.
///
/// Le placement doit passer [`Layout::check`] : un élément qui manque ou qui
/// est en trop n'est pas mesuré, et ne fait pas échouer la mesure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Paires de segments de deux liens distincts qui se coupent en un point
    /// intérieur aux deux tracés.
    pub crossings: u64,
    /// Paires de segments de deux liens distincts, colinéaires, qui se
    /// recouvrent sur une longueur non nulle.
    pub shared_segments: u64,
    /// Longueur cumulée de ces recouvrements.
    pub shared_length: i64,
    /// Sommets entre deux segments droits où la direction change.
    pub bends: u64,
    /// Longueur cumulée de tous les tracés.
    pub edge_length: i64,
    /// Longueur du tracé le plus long.
    pub longest_edge: i64,
    /// Paires de nœuds dont les rectangles se recouvrent, hors parent-enfant.
    pub node_overlaps: u64,
    /// Aire cumulée de ces recouvrements.
    pub node_overlap_area: i64,
    /// Paires lien-nœud où le tracé entre dans un nœud étranger au lien.
    pub edge_node_overlaps: u64,
    /// Étiquettes qui recouvrent un nœud autre qu'un conteneur de leur lien,
    /// une autre étiquette ou un lien autre que le leur, une par paire.
    pub label_overlaps: u64,
    /// Boîte englobante de tout ce qui est dessiné.
    pub width: i64,
    pub height: i64,
    pub area: i64,
    /// Largeur sur hauteur, en millièmes ; 0 sans hauteur.
    pub proportion: i64,
    /// Aire des feuilles sur aire de la boîte, en millièmes ; 0 sans aire.
    pub density: i64,
    /// Paires de nœuds dont les `layer` imposés et les positions le long de
    /// la direction ne sont pas dans le même ordre.
    pub layer_violations: u64,
    /// Paires de nœuds de même couche dont les `order` imposés et les
    /// positions en travers de la direction ne sont pas dans le même ordre.
    pub order_violations: u64,
    /// Paires de ports d'un même côté, sous `ports_ordered`, qui ne se
    /// suivent pas dans le sens des aiguilles d'une montre.
    pub port_order_violations: u64,
}

/// Un lien tel qu'il est mesuré : son tracé aplati, et ce qui l'exempte
/// des recouvrements.
struct Measured<'a> {
    /// Les sommets, sans point répété.
    points: Vec<Point>,
    /// Pour chaque sommet, vrai s'il joint deux segments droits.
    corner: Vec<bool>,
    /// Les nœuds que le lien a le droit de toucher : ses deux bouts et
    /// leurs ancêtres.
    exempt: Vec<&'a str>,
    /// Les nœuds qui enferment le lien : les ancêtres de ses bouts, que ses
    /// étiquettes ont le droit de recouvrir.
    inside: Vec<&'a str>,
    /// Les rectangles de ses étiquettes.
    labels: Vec<Rect>,
}

impl Metrics {
    /// Mesure un placement valide pour son graphe.
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

    /// Les lignes du rapport, une par mesure, dans un ordre fixe.
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
        // Les paires de liens : croisements et recouvrements, chaque paire
        // une fois.
        let paths: Vec<Vec<Point>> = edges.iter().map(|e| merge_collinear(&e.points)).collect();
        for (i, p) in paths.iter().enumerate() {
            for q in &paths[i + 1..] {
                self.pair(p, q);
            }
        }
    }

    /// Les croisements et recouvrements de deux tracés.
    fn pair(&mut self, p: &[Point], q: &[Point]) {
        for a in p.windows(2) {
            for b in q.windows(2) {
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
        // Un tracé qui passe par un sommet de l'autre : la paire de segments
        // ne le voit pas, le sommet le dit. Un sommet commun aux deux est
        // jugé une fois, du côté de `q`.
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
        // Le centre d'un nœud, doublé pour rester entier : le long de la
        // direction, puis en travers.
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

/// Les rectangles des étiquettes d'un lien, en absolu, dans l'ordre du graphe.
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

/// La boîte englobante de rectangles et de points ; aucune sans rien.
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

/// La boîte englobante de tout ce qui est dessiné : nœuds, sommets des
/// tracés aplatis, étiquettes. La même que celle des mesures. Aucune pour
/// un placement vide.
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

/// Aplatit un tracé en polyligne : chaque courbe devient [`CURVE_STEPS`]
/// segments, les points répétés sont retirés. C'est la polyligne que les
/// mesures lisent, et celle que le rendu dessine.
pub fn polyline(route: &[Segment]) -> Vec<Point> {
    flatten(route).0
}

/// Aplatit un tracé en polyligne : chaque courbe devient [`CURVE_STEPS`]
/// segments, les points répétés sont retirés. Rend aussi, pour chaque
/// sommet, s'il joint deux segments droits.
fn flatten(route: &[Segment]) -> (Vec<Point>, Vec<bool>) {
    let mut points: Vec<Point> = Vec::new();
    let mut corner: Vec<bool> = Vec::new();
    let push = |p: Point, from_line: bool, points: &mut Vec<Point>, corner: &mut Vec<bool>| {
        if points.last() == Some(&p) {
            // Un point répété ne fait pas de sommet ; la jonction prend le
            // caractère du segment qui arrive.
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
    // Un sommet joint deux segments droits si celui qui arrive et celui qui
    // part le sont ; `corner[i]` ne dit encore que celui qui arrive.
    let n = points.len();
    let mut both = vec![false; n];
    for i in 1..n.saturating_sub(1) {
        both[i] = corner[i] && corner[i + 1];
    }
    (points, both)
}

/// Le point de la courbe au pas `k` sur [`CURVE_STEPS`], arrondi au plus près.
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

/// Retire les sommets où la polyligne continue tout droit : un tracé dont
/// chaque sommet est un vrai changement de direction.
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

/// Vrai si les deux vecteurs sont colinéaires et de même sens.
fn same_direction(a: Point, b: Point) -> bool {
    cross(a, b) == 0 && dot(a, b) > 0
}

/// Longueur d'un segment, arrondie au centième le plus près.
fn distance(a: Point, b: Point) -> i64 {
    let d = sub(b, a);
    let squared = dot(d, d).unsigned_abs();
    let root = squared.isqrt();
    // Au plus près : au-delà de r² + r, c'est r + 1 qui est le plus proche.
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

/// `num / den` en millièmes, arrondi au plus près ; 0 si `den` est nul.
fn ratio(num: i128, den: i128) -> i64 {
    if den <= 0 {
        return 0;
    }
    clamp((num * 1000 + den / 2).div_euclid(den))
}

/// Ce que deux segments ont en commun.
enum Meet {
    None,
    /// Un seul point, intérieur aux deux.
    Proper,
    /// Un seul point, bout de l'un au moins.
    Touch,
    /// Un morceau commun, de cette longueur.
    Shared(i64),
}

/// De quel côté de `ab` est `p` : signe du produit vectoriel.
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
        // Colinéaires : l'intervalle commun, mesuré le long de `ab` par la
        // projection de chaque bout.
        let dir = sub(b, a);
        let key = |p: Point| dot(sub(p, a), dir);
        let (kc, kd) = (key(c), key(d));
        let (lo, hi) = (kc.min(kd).max(0), kc.max(kd).min(key(b)));
        if hi > lo {
            // Les deuxième et troisième points dans l'ordre bornent le morceau.
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

/// Vrai si `p`, déjà sur la droite `se`, est entre `s` et `e`, bouts compris.
fn between(p: Point, s: Point, e: Point) -> bool {
    p.x >= s.x.min(e.x) && p.x <= s.x.max(e.x) && p.y >= s.y.min(e.y) && p.y <= s.y.max(e.y)
}

/// Vrai si le tracé `prev → at → next` traverse le tracé `path` en `at`,
/// sommet de l'un posé sur l'autre : les deux voisins sont de part et
/// d'autre. Un voisin posé sur `path` est un morceau commun, pas une
/// traversée.
fn passes_through(prev: Point, at: Point, next: Point, path: &[Point]) -> bool {
    // Les deux rayons de `path` en `at` : ceux d'un segment si `at` est à
    // l'intérieur, ceux des deux segments voisins si `at` est un sommet.
    let rays: Option<(Point, Point)> = if let Some(i) = path.iter().position(|p| *p == at) {
        if i == 0 || i + 1 == path.len() {
            // Un bout de `path` : rien ne continue de l'autre côté.
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
        // `path` va tout droit : traversée si les voisins sont strictement
        // de part et d'autre de la droite.
        let (s1, s2) = (cross(r1, q1).signum(), cross(r1, q2).signum());
        return s1 * s2 < 0;
    }
    // `path` tourne : traversée si un voisin est dans l'angle `r1 → r2`
    // et l'autre dehors, aucun sur un rayon.
    let on_ray = |q: Point| same_direction(q, r1) || same_direction(q, r2);
    if on_ray(q1) || on_ray(q2) {
        return false;
    }
    in_wedge(r1, r2, q1) != in_wedge(r1, r2, q2)
}

/// Vrai si `q` est strictement dans l'angle balayé de `r1` à `r2` dans le
/// sens trigonométrique.
fn in_wedge(r1: Point, r2: Point, q: Point) -> bool {
    if cross(r1, r2) > 0 {
        cross(r1, q) > 0 && cross(q, r2) > 0
    } else {
        !(cross(r2, q) > 0 && cross(q, r1) > 0)
    }
}

/// Vrai si un segment de la polyligne entre dans l'intérieur du rectangle.
fn enters(points: &[Point], rect: Rect) -> bool {
    points.windows(2).any(|s| segment_enters(s[0], s[1], rect))
}

fn segment_enters(a: Point, b: Point, rect: Rect) -> bool {
    let (x1, x2) = (i128::from(rect.x), i128::from(rect.x + rect.width));
    let (y1, y2) = (i128::from(rect.y), i128::from(rect.y + rect.height));
    if x2 <= x1 || y2 <= y1 {
        return false;
    }
    // L'intervalle des t de [0, 1] où le point a + t (b − a) est strictement
    // dans le rectangle, borné par des fractions `num / den` avec den > 0.
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

/// Aire du recouvrement de deux rectangles ; 0 s'ils ne font que se toucher.
fn overlap_area(a: Rect, b: Rect) -> i64 {
    let w = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let h = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    if w > 0 && h > 0 {
        clamp(i128::from(w) * i128::from(h))
    } else {
        0
    }
}

/// Position d'un port le long de son côté, dans le sens des aiguilles d'une
/// montre : par x au nord, par y à l'est, à rebours au sud et à l'ouest.
fn clockwise(side: Side, x: i64, y: i64) -> i64 {
    match side {
        Side::North => x,
        Side::East => y,
        Side::South => -x,
        Side::West => -y,
    }
}
