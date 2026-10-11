// ForegramPM — la scène : ce qui est dessiné, avant de choisir SVG ou PNG.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use foregram_graph::{Graph, Layout, Point, Rect, Segment, extent, polyline};

/// L'aspect neutre : des gris et des épaisseurs, les mêmes pour tous les
/// moteurs. Les gris vont de 0 (noir) à 255 (blanc) ; les longueurs sont en
/// centièmes de pixel. [`Style::default`] est l'aspect du banc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    /// Marge entre le dessin et le bord de la feuille.
    pub margin: i64,
    /// Fond de la feuille.
    pub sheet: u8,
    /// Fond d'une feuille du graphe (un nœud sans enfant).
    pub leaf_fill: u8,
    /// Encre des traits pleins : bord des feuilles, liens, flèches, ports.
    pub ink: u8,
    /// Encre d'un conteneur, plus claire pour que le contenu se lise.
    pub container_ink: u8,
    /// Épaisseur des traits forts : bord des feuilles, liens.
    pub thick: i64,
    /// Épaisseur des traits fins : conteneurs, étiquettes.
    pub thin: i64,
    /// Longueur de la flèche, et largeur de sa base.
    pub arrow_length: i64,
    pub arrow_width: i64,
    /// Côté du carré d'un port sans boîte.
    pub port_size: i64,
    /// Tiret et espace du contour d'une étiquette.
    pub dash: i64,
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

/// Ce qu'un élément est, donc comment il se dessine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// La feuille de fond, qui couvre la fenêtre.
    Sheet,
    /// Un nœud sans enfant : fond et bord.
    Leaf,
    /// Un conteneur : bord fin seulement.
    Container,
    /// Le tracé d'un lien.
    Edge,
    /// La pointe d'un lien, un triangle plein.
    Arrow,
    /// Une étiquette : rectangle en tirets, vide.
    Label,
    /// Un port : un carré plein, ou sa boîte.
    Port,
}

impl Class {
    /// Le nom de la classe dans le SVG.
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

/// Une forme, en centièmes de pixel absolus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    Rect(Rect),
    /// Un tracé tel que le placement le donne, courbes comprises.
    Route(Vec<Segment>),
    /// Un polygone plein.
    Polygon(Vec<Point>),
}

/// Un élément dessiné, avec l'identifiant qu'il porte dans le graphe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub class: Class,
    pub shape: Shape,
    pub title: Option<String>,
}

/// Tout ce qui est dessiné, dans l'ordre où ça se dessine : la feuille, les
/// nœuds du haut vers le bas de l'imbrication, les liens et leurs flèches,
/// les étiquettes, les ports par-dessus tout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene {
    /// Le nom du graphe et le moteur, pour le titre du dessin.
    pub title: String,
    /// La fenêtre : la boîte englobante du dessin, plus la marge.
    pub view: Rect,
    pub items: Vec<Item>,
}

impl Scene {
    /// Construit la scène d'un placement. Ce qui n'est pas placé n'est pas
    /// dessiné ; un placement qui passe `Layout::check` est dessiné en entier.
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

/// Le triangle qui termine un tracé : la pointe au bout, la base en arrière
/// dans la direction du dernier segment de la polyligne. Aucun si le tracé
/// n'a pas de direction.
fn arrow(points: &[Point], style: &Style) -> Option<Vec<Point>> {
    let tip = *points.last()?;
    let from = *points.iter().rev().find(|p| **p != tip)?;
    let (dx, dy) = (tip.x - from.x, tip.y - from.y);
    let len = isqrt(i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy));
    if len == 0 {
        return None;
    }
    // Un vecteur de longueur `l` dans la direction (dx, dy), arrondi.
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

/// Racine carrée entière, arrondie au plus près.
pub(crate) fn isqrt(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let mut x = n.isqrt();
    // `isqrt` tronque ; on arrondit au plus près.
    if (x + 1) * (x + 1) - n < n - x * x {
        x += 1;
    }
    x
}

/// Quotient arrondi au plus près, le diviseur étant positif.
pub(crate) fn div_round(a: i128, b: i128) -> i64 {
    let q = (2 * a + b).div_euclid(2 * b);
    i64::try_from(q).unwrap_or(i64::MAX)
}
