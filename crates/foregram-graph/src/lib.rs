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
//!
//! Un refus porte un code stable, une position et un [`Message`] : une clé
//! et des arguments. Sa phrase est composée à l'affichage depuis le
//! [`Catalog`] de la langue choisie ; seul l'anglais existe pour l'instant.

mod diag;
mod graph;
pub mod json;
mod layout;
mod msg;
mod read;

pub use diag::{Diagnostic, Pos};
pub use graph::{
    Anchor, Direction, Edge, GRAPH_FORMAT, Graph, Label, Node, Point, Port, Rect, Side, Size,
};
pub use layout::{EdgePlace, Engine, LAYOUT_FORMAT, Layout, NodePlace, PortPlace, Segment};
pub use msg::{Arg, Catalog, Message};
pub use read::{COORD_MAX, FORMAT_VERSION, IDENT_MAX_CHARS, LAYER_MAX};
