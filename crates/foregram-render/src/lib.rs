// ForegramPM — neutral rendering of a layout.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Draws a layout for its graph from the coordinates alone: the same look
//! whatever the engine, to compare the layout and not the style.
//!
//! The drawing goes through a [`Scene`]: classified rectangles, routes and
//! polygons, which the SVG and the PNG each render their own way. Both
//! outputs therefore show the same thing. The PNG is rasterised here, in
//! integers; only its encoding comes from the `png` crate, behind the
//! feature of the same name.
//!
//! No text: the exchange format carries none. Identifiers go into the
//! `<title>` elements of the SVG, where they can be searched and show on
//! hover.
//!
//! Like the graph library, this code touches no file, clock or environment,
//! and uses integers only: the same input gives the same bytes on every
//! machine.
//!
//! # Public surface
//!
//! This is the contract frozen for the callers of the library, from version
//! 0.1.0 on. It is held by `tests/api.rs`, which lists every exported name:
//! adding one means adding it there.
//!
//! - **Drawing**: [`svg`], and `png` with the feature of the same name, bounded by
//!   [`SCALE_MAX`] and [`PIXELS_MAX`]; [`Style`] for the look.
//! - **Scene**: [`Scene`], [`Item`], [`Shape`], [`Class`] — what the two
//!   outputs draw, exposed so that a caller can render it another way
//!   (a canvas, for instance) and still show the same thing.
//!
//! The layout given to [`svg`], `png` and [`Scene::build`] is taken as
//! checked by `Layout::check`. One that is not may come back as an internal
//! error, family `INT`, which the command line reports with exit code 3: the
//! caller checks first and never draws with a hole.

#![deny(missing_docs)]

mod scene;
mod svg;

#[cfg(feature = "png")]
mod raster;

pub use scene::{Class, Item, Scene, Shape, Style};

use foregram_graph::{Diagnostic, Graph, Layout};

/// The SVG of a layout, hundredths of a pixel converted to pixels. The layout
/// is taken as checked: see [`Scene::build`] for the internal errors.
pub fn svg(graph: &Graph, layout: &Layout, style: &Style) -> Result<String, Diagnostic> {
    Ok(svg::write(&Scene::build(graph, layout, style)?, style))
}

/// The PNG of a layout, in greyscale, at `scale` pixels per pixel of the
/// drawing. Refuses a scale outside 1 to [`SCALE_MAX`] (DRW001), an image of
/// more than [`PIXELS_MAX`] pixels (DRW002); the layout is taken as checked,
/// see [`Scene::build`] for the internal errors.
#[cfg(feature = "png")]
pub fn png(
    graph: &Graph,
    layout: &Layout,
    style: &Style,
    scale: i64,
) -> Result<Vec<u8>, Diagnostic> {
    raster::render(&Scene::build(graph, layout, style)?, style, scale)
}

/// The largest scale of a PNG, in pixels per pixel of the drawing.
pub const SCALE_MAX: i64 = 8;

/// The largest number of pixels of a PNG.
pub const PIXELS_MAX: i64 = 50_000_000;
