// ForegramPM — abstract graph and layout.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! The graph a layout engine receives, and the layout it returns.
//!
//! Two JSON formats, described in `docs/exchange-format.md`:
//! - [`Graph`]: rectangular nodes, edges, their labels, and what the author
//!   chose to pin;
//! - [`Layout`]: the position of every node, the route of every edge.
//!
//! [`Metrics`] measures a layout for its graph: crossings, bends, lengths,
//! overlaps, extent, honoured constraints.
//!
//! The library reads bytes and returns values or refusals. It touches no
//! file, clock or environment, and uses integers only: the same input gives
//! the same bytes on every machine.
//!
//! A refusal carries a stable code, a position and a [`Message`]: a key and
//! its arguments. Its sentence is composed at display time from the
//! [`Catalog`] of the chosen language, English or French.
//!
//! # Public surface
//!
//! This is the contract frozen for the callers of the library, from version
//! 0.1.0 on. It is held by `tests/api.rs`, which lists every exported name:
//! adding one means adding it there.
//!
//! - **Model**: [`Graph`], [`Node`], [`Port`], [`Edge`], [`Label`], and the
//!   value types [`Point`], [`Size`], [`Rect`], [`Side`], [`Anchor`],
//!   [`Direction`]; [`Layout`], [`NodePlace`], [`PortPlace`], [`EdgePlace`],
//!   [`Segment`], [`Engine`].
//! - **Reading and writing**: `Graph::parse`, `Layout::parse`, the
//!   `validate`, `check` and `to_json` methods; the format identifiers
//!   [`GRAPH_FORMAT`] and [`LAYOUT_FORMAT`], the limits [`FORMAT_VERSION`],
//!   [`COORD_MAX`], [`LAYER_MAX`] and [`IDENT_MAX_CHARS`].
//! - **Measuring**: [`Metrics`], [`CURVE_STEPS`], and the two helpers the
//!   renderer shares with the metrics, [`polyline`] and [`extent`], so that
//!   a drawing shows exactly what was measured.
//! - **Refusals**: [`Diagnostic`], [`Pos`], [`file_report`]; [`Message`],
//!   [`Arg`], [`Catalog`].
//! - **JSON**: the [`json`] module, the strict reader the formats are built
//!   on — positions kept, integers only — for a caller that re-reads what
//!   the library wrote.

#![deny(missing_docs)]

mod diag;
mod graph;
pub mod json;
mod layout;
mod metrics;
mod msg;
mod read;

pub use diag::{Diagnostic, Pos, file_report};
pub use graph::{
    Anchor, Direction, Edge, GRAPH_FORMAT, Graph, Label, Node, Point, Port, Rect, Side, Size,
};
pub use layout::{EdgePlace, Engine, LAYOUT_FORMAT, Layout, NodePlace, PortPlace, Segment};
pub use metrics::{CURVE_STEPS, Metrics, extent, polyline};
pub use msg::{Arg, Catalog, Message};
pub use read::{COORD_MAX, FORMAT_VERSION, IDENT_MAX_CHARS, LAYER_MAX};
