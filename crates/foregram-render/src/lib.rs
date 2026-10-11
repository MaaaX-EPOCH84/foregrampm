// ForegramPM — rendu neutre d'un placement.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Dessine un placement pour son graphe, du seul fait des coordonnées : même
//! aspect quel que soit le moteur, pour comparer le placement et non le style.
//!
//! Le dessin passe par une [`Scene`] : des rectangles, des tracés et des
//! polygones classés, que le SVG et le PNG rendent chacun à leur façon. Les
//! deux sorties montrent donc la même chose. Le PNG est rastérisé ici, en
//! entiers ; seul son encodage vient de la crate `png`, derrière la feature
//! du même nom.
//!
//! Aucun texte : le format d'échange n'en porte pas. Les identifiants vont
//! dans les `<title>` du SVG, où ils se cherchent et s'affichent au survol.
//!
//! Comme la bibliothèque du graphe, ce code ne touche ni fichier, ni
//! horloge, ni environnement, et n'emploie que des entiers : la même entrée
//! donne les mêmes octets sur toute machine.

mod scene;
mod svg;

#[cfg(feature = "png")]
mod raster;

pub use scene::{Class, Item, Scene, Shape, Style};

use foregram_graph::{Graph, Layout};

/// Le SVG d'un placement, en centièmes de pixel convertis en pixels.
pub fn svg(graph: &Graph, layout: &Layout, style: &Style) -> String {
    svg::write(&Scene::build(graph, layout, style), style)
}

/// Le PNG d'un placement, en niveaux de gris, à `scale` pixels par pixel du
/// dessin. Refuse une échelle hors de 1 à [`SCALE_MAX`] (V001), une image de
/// plus de [`PIXELS_MAX`] pixels (V002).
#[cfg(feature = "png")]
pub fn png(
    graph: &Graph,
    layout: &Layout,
    style: &Style,
    scale: i64,
) -> Result<Vec<u8>, foregram_graph::Diagnostic> {
    raster::render(&Scene::build(graph, layout, style), style, scale)
}

/// La plus grande échelle d'un PNG, en pixels par pixel du dessin.
pub const SCALE_MAX: i64 = 8;

/// Le plus grand nombre de pixels d'un PNG.
pub const PIXELS_MAX: i64 = 50_000_000;
