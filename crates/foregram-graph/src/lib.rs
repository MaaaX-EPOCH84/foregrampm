// ForegramPM — graphe abstrait et placement.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Le graphe qu'un moteur de placement reçoit, et le placement qu'il rend.
//!
//! Deux formats JSON, décrits dans `docs/exchange-format.md` :
//! - [`Graph`] : des nœuds rectangulaires, des liens, leurs étiquettes, et
//!   ce que l'auteur a voulu fixer ;
//! - [`Layout`] : la position de chaque nœud, le tracé de chaque lien.
//!
//! La bibliothèque lit des octets et rend des valeurs ou des refus. Elle ne
//! touche ni fichier, ni horloge, ni environnement, et n'emploie que des
//! entiers : la même entrée donne les mêmes octets sur toute machine.

mod diag;
mod graph;
pub mod json;
mod layout;
mod read;

pub use diag::{Diagnostic, Pos};
pub use graph::{Anchor, Direction, Edge, GRAPH_FORMAT, Graph, Label, Node, Side};
pub use layout::{EdgePlace, Engine, LAYOUT_FORMAT, Layout, NodePlace, Point};
pub use read::{COORD_MAX, FORMAT_VERSION, IDENT_MAX_CHARS, LAYER_MAX};
