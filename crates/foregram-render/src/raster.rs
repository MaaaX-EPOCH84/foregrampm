// ForegramPM — rasterisation of a scene to greyscale, then PNG.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Everything is a polygon: a filled rectangle, the ring of a border, the
//! quadrilateral of each segment of a stroke, the octagons of its joints, the
//! dashes of a label, the triangle of an arrowhead. A polygon is filled by the
//! non-zero winding rule, with anti-aliasing: each pixel row is swept in
//! [`SUB`] sub-scanlines, and on each one the horizontal coverage is exact, to
//! the [`UNIT`]th of a pixel. Integers only, fixed rounding: the same scene
//! gives the same pixels everywhere.

use foregram_graph::{Diagnostic, Message, Point, Pos, Rect, polyline};

use crate::scene::{Class, Scene, Shape, Style, div_round, isqrt};
use crate::{PIXELS_MAX, SCALE_MAX};

/// Sub-scanlines per pixel row.
const SUB: i64 = 16;
/// Units per pixel in image coordinates.
const UNIT: i64 = 256;

/// Rasterises the scene and encodes it as PNG, at `scale` image pixels per
/// drawing pixel.
pub(crate) fn render(scene: &Scene, style: &Style, scale: i64) -> Result<Vec<u8>, Diagnostic> {
    if !(1..=SCALE_MAX).contains(&scale) {
        return Err(Diagnostic::new(
            Pos::NONE,
            Message::new("DRW001")
                .text("scale", scale)
                .text("max", SCALE_MAX),
        ));
    }
    let width = pixels(scene.view.width, scale).max(1);
    let height = pixels(scene.view.height, scale).max(1);
    if i128::from(width) * i128::from(height) > i128::from(PIXELS_MAX) {
        return Err(Diagnostic::new(
            Pos::NONE,
            Message::new("DRW002")
                .text("width", width)
                .text("height", height)
                .text("max", PIXELS_MAX),
        ));
    }
    let mut canvas = Canvas {
        width,
        height,
        scale,
        origin: Point {
            x: scene.view.x,
            y: scene.view.y,
        },
        pixels: vec![style.sheet; usize::try_from(width * height).unwrap_or(0)],
    };
    for item in &scene.items {
        canvas.paint(item.class, &item.shape, style);
    }
    encode(width, height, &canvas.pixels)
}

/// Hundredths of a pixel to image pixels, rounded to the nearest.
fn pixels(hundredths: i64, scale: i64) -> i64 {
    div_round(i128::from(hundredths) * i128::from(scale), 100)
}

/// The image under construction: greys, one row after another.
struct Canvas {
    width: i64,
    height: i64,
    scale: i64,
    /// The top-left corner of the window, in hundredths.
    origin: Point,
    pixels: Vec<u8>,
}

/// A polygon side oriented downwards, with its original direction.
struct Side {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    winding: i64,
}

impl Canvas {
    /// A drawing point in image coordinates, to the [`UNIT`]th of a pixel.
    fn device(&self, p: Point) -> (i64, i64) {
        let to = |v: i64, o: i64| {
            div_round(
                i128::from(v - o) * i128::from(self.scale) * i128::from(UNIT),
                100,
            )
        };
        (to(p.x, self.origin.x), to(p.y, self.origin.y))
    }

    fn paint(&mut self, class: Class, shape: &Shape, style: &Style) {
        match (class, shape) {
            // The sheet is already the image background.
            (Class::Sheet, _) => {}
            (Class::Leaf, Shape::Rect(rect)) => {
                self.fill(&[corners(*rect)], style.leaf_fill);
                self.fill(&ring(*rect, style.thick), style.ink);
            }
            (Class::Container, Shape::Rect(rect)) => {
                self.fill(&ring(*rect, style.thin), style.container_ink);
            }
            (Class::Label, Shape::Rect(rect)) => {
                self.fill(&dashes(*rect, style), style.ink);
            }
            // A hollow square: the sheet background, then a thin border, so
            // that the port stands out from the node border and the link end.
            (Class::Port, Shape::Rect(rect)) => {
                self.fill(&[corners(*rect)], style.sheet);
                self.fill(&ring(*rect, style.thin), style.ink);
            }
            (Class::Edge, Shape::Route(route)) => {
                self.fill(&stroke(&polyline(route), style.thick), style.ink);
            }
            (Class::Arrow, Shape::Polygon(points)) => {
                self.fill(std::slice::from_ref(points), style.ink)
            }
            // A class on a shape it does not take: nothing to draw.
            _ => {}
        }
    }

    /// Fills the union of the contours, by the non-zero winding number. Each
    /// contour is first oriented the same way, except a hole, already
    /// oriented the opposite way by whoever built it.
    fn fill(&mut self, contours: &[Vec<Point>], gray: u8) {
        let mut sides: Vec<Side> = Vec::new();
        for contour in contours {
            let device: Vec<(i64, i64)> = contour.iter().map(|p| self.device(*p)).collect();
            for (i, &(x0, y0)) in device.iter().enumerate() {
                let (x1, y1) = device[(i + 1) % device.len()];
                if y0 == y1 {
                    continue;
                }
                sides.push(if y0 < y1 {
                    Side {
                        x0,
                        y0,
                        x1,
                        y1,
                        winding: 1,
                    }
                } else {
                    Side {
                        x0: x1,
                        y0: y1,
                        x1: x0,
                        y1: y0,
                        winding: -1,
                    }
                });
            }
        }
        if sides.is_empty() {
            return;
        }
        sides.sort_by_key(|s| s.y0);
        let top = sides[0].y0.max(0);
        let bottom = sides
            .iter()
            .map(|s| s.y1)
            .max()
            .unwrap_or(0)
            .min(self.height * UNIT);
        if bottom <= top {
            return;
        }
        let row_width = usize::try_from(self.width).unwrap_or(0);
        let mut cover = vec![0i64; row_width + 1];
        let mut active: Vec<usize> = Vec::new();
        let mut crossings: Vec<(i64, i64)> = Vec::new();
        let mut next = 0;
        for row in top.div_euclid(UNIT)..=(bottom - 1).div_euclid(UNIT) {
            cover.fill(0);
            let mut touched = false;
            for s in 0..SUB {
                let ys = row * UNIT + (2 * s + 1) * (UNIT / (2 * SUB));
                while next < sides.len() && sides[next].y0 <= ys {
                    active.push(next);
                    next += 1;
                }
                active.retain(|&i| sides[i].y1 > ys);
                crossings.clear();
                for &i in &active {
                    let side = &sides[i];
                    let x = side.x0
                        + div_round(
                            i128::from(ys - side.y0) * i128::from(side.x1 - side.x0),
                            i128::from(side.y1 - side.y0),
                        );
                    crossings.push((x, side.winding));
                }
                crossings.sort_unstable();
                let mut winding = 0;
                let mut start = 0;
                for &(x, w) in &crossings {
                    if winding == 0 {
                        start = x;
                    }
                    winding += w;
                    if winding == 0 {
                        touched |= span(&mut cover, start, x, self.width);
                    }
                }
            }
            if !touched {
                continue;
            }
            let Ok(row_start) = usize::try_from(row * self.width) else {
                continue;
            };
            for (px, &c) in cover[..row_width].iter().enumerate() {
                if c == 0 {
                    continue;
                }
                let alpha = div_round(i128::from(c) * 255, i128::from(UNIT * SUB)).min(255);
                let dst = &mut self.pixels[row_start + px];
                let d = i64::from(*dst);
                let v = d + div_round(i128::from(i64::from(gray) - d) * i128::from(alpha), 255);
                *dst = u8::try_from(v.clamp(0, 255)).unwrap_or(0);
            }
        }
    }
}

/// Adds the coverage of the interval `[xa, xb)` of a sub-scanline, in
/// [`UNIT`]ths of a pixel, to the row; true if anything was added.
fn span(cover: &mut [i64], xa: i64, xb: i64, width: i64) -> bool {
    let xa = xa.max(0);
    let xb = xb.min(width * UNIT);
    if xb <= xa {
        return false;
    }
    let (pa, pb) = (xa / UNIT, xb / UNIT);
    let at = |p: i64| usize::try_from(p).unwrap_or(0);
    if pa == pb {
        cover[at(pa)] += xb - xa;
    } else {
        cover[at(pa)] += UNIT - xa % UNIT;
        for p in pa + 1..pb {
            cover[at(p)] += UNIT;
        }
        cover[at(pb)] += xb % UNIT;
    }
    true
}

/// The four corners of a rectangle, clockwise on screen (the y axis pointing
/// down).
fn corners(rect: Rect) -> Vec<Point> {
    let (r, b) = (rect.x + rect.width, rect.y + rect.height);
    vec![
        Point {
            x: rect.x,
            y: rect.y,
        },
        Point { x: r, y: rect.y },
        Point { x: r, y: b },
        Point { x: rect.x, y: b },
    ]
}

/// The border of a rectangle, of thickness `width` centred on the contour: the
/// enlarged rectangle, with the shrunk rectangle cut out when one remains.
fn ring(rect: Rect, width: i64) -> Vec<Vec<Point>> {
    let h = width / 2;
    let outer = corners(Rect {
        x: rect.x - h,
        y: rect.y - h,
        width: rect.width + 2 * h,
        height: rect.height + 2 * h,
    });
    let inner = Rect {
        x: rect.x + h,
        y: rect.y + h,
        width: rect.width - 2 * h,
        height: rect.height - 2 * h,
    };
    if inner.width <= 0 || inner.height <= 0 {
        return vec![outer];
    }
    let mut hole = corners(inner);
    hole.reverse();
    vec![outer, hole]
}

/// A stroke along a polyline: one quadrilateral per segment, one octagon at
/// each interior vertex to close the joints.
fn stroke(points: &[Point], width: i64) -> Vec<Vec<Point>> {
    let mut out = Vec::new();
    for pair in points.windows(2) {
        if let Some(quad) = quad(pair[0], pair[1], width) {
            out.push(quad);
        }
    }
    for p in &points[1..points.len().saturating_sub(1)] {
        out.push(octagon(*p, width / 2));
    }
    out
}

/// The quadrilateral of a thick segment, oriented like [`corners`]; none for
/// a zero-length segment.
fn quad(a: Point, b: Point, width: i64) -> Option<Vec<Point>> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = isqrt(i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy));
    if len == 0 {
        return None;
    }
    let h = i128::from(width / 2);
    let nx = div_round(-i128::from(dy) * h, len);
    let ny = div_round(i128::from(dx) * h, len);
    Some(oriented(vec![
        Point {
            x: a.x + nx,
            y: a.y + ny,
        },
        Point {
            x: b.x + nx,
            y: b.y + ny,
        },
        Point {
            x: b.x - nx,
            y: b.y - ny,
        },
        Point {
            x: a.x - nx,
            y: a.y - ny,
        },
    ]))
}

/// An octagon of radius `r` around a point, oriented like [`corners`].
fn octagon(c: Point, r: i64) -> Vec<Point> {
    // r · cos 45°, rounded.
    let d = div_round(i128::from(r) * 7071, 10000);
    [
        (r, 0),
        (d, d),
        (0, r),
        (-d, d),
        (-r, 0),
        (-d, -d),
        (0, -r),
        (d, -d),
    ]
    .into_iter()
    .map(|(x, y)| Point {
        x: c.x + x,
        y: c.y + y,
    })
    .collect()
}

/// The dashes of a label's contour, following the perimeter from the top-left
/// corner, with the style's dash and gap lengths.
fn dashes(rect: Rect, style: &Style) -> Vec<Vec<Point>> {
    let c = corners(rect);
    let sides = [(c[0], c[1]), (c[1], c[2]), (c[2], c[3]), (c[3], c[0])];
    let mut out = Vec::new();
    let (dash, gap) = (style.dash.max(1), style.gap.max(1));
    let mut on = true;
    let mut left = dash;
    for (a, b) in sides {
        let length = (b.x - a.x).abs() + (b.y - a.y).abs();
        let (ux, uy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
        let mut pos = 0;
        while pos < length {
            let take = left.min(length - pos);
            if on {
                let from = Point {
                    x: a.x + ux * pos,
                    y: a.y + uy * pos,
                };
                let to = Point {
                    x: a.x + ux * (pos + take),
                    y: a.y + uy * (pos + take),
                };
                if let Some(q) = quad(from, to, style.thin) {
                    out.push(q);
                }
            }
            pos += take;
            left -= take;
            if left == 0 {
                on = !on;
                left = if on { dash } else { gap };
            }
        }
    }
    out
}

/// The polygon oriented like [`corners`]: positive signed area on screen.
fn oriented(mut points: Vec<Point>) -> Vec<Point> {
    let mut area: i128 = 0;
    for (i, p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        area += i128::from(p.x) * i128::from(q.y) - i128::from(q.x) * i128::from(p.y);
    }
    if area < 0 {
        points.reverse();
    }
    points
}

/// Encodes the image as PNG, 8-bit grey.
fn encode(width: i64, height: i64, pixels: &[u8]) -> Result<Vec<u8>, Diagnostic> {
    let fail = |error: png::EncodingError| {
        Diagnostic::new(Pos::NONE, Message::new("DRW003").text("error", error))
    };
    let (w, h) = (
        u32::try_from(width).unwrap_or(0),
        u32::try_from(height).unwrap_or(0),
    );
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(fail)?;
    writer.write_image_data(pixels).map_err(fail)?;
    writer.finish().map_err(fail)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(width: i64, height: i64) -> Canvas {
        Canvas {
            width,
            height,
            scale: 1,
            origin: Point { x: 0, y: 0 },
            pixels: vec![255; usize::try_from(width * height).unwrap_or(0)],
        }
    }

    fn rect(x: i64, y: i64, width: i64, height: i64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn a_pixel_aligned_rectangle_is_full_inside_and_empty_outside() {
        let mut c = canvas(4, 4);
        c.fill(&[corners(rect(100, 100, 200, 200))], 0);
        let expected = [
            255, 255, 255, 255, //
            255, 0, 0, 255, //
            255, 0, 0, 255, //
            255, 255, 255, 255,
        ];
        assert_eq!(c.pixels, expected);
    }

    #[test]
    fn a_half_pixel_edge_is_half_covered() {
        let mut c = canvas(3, 1);
        // From 0.5 to 2.5: the middle pixel full, the other two half covered.
        c.fill(&[corners(rect(50, 0, 200, 100))], 0);
        assert_eq!(c.pixels, [127, 0, 127]);
        // A quarter of the height: quarter coverage, over the sixteen
        // sub-scanlines.
        let mut c = canvas(1, 1);
        c.fill(&[corners(rect(0, 0, 100, 25))], 0);
        assert_eq!(c.pixels, [191]);
    }

    #[test]
    fn a_diagonal_is_symmetric_and_a_ring_has_a_hole() {
        let mut c = canvas(4, 4);
        c.fill(
            &[vec![
                Point { x: 0, y: 0 },
                Point { x: 400, y: 0 },
                Point { x: 0, y: 400 },
            ]],
            0,
        );
        for i in 0..4 {
            for j in 0..4 {
                assert_eq!(c.pixels[i * 4 + j], c.pixels[j * 4 + i], "({i}, {j})");
            }
        }
        assert_eq!(c.pixels[0], 0);
        assert_eq!(c.pixels[15], 255);
        assert_eq!(c.pixels[3], c.pixels[12]);
        assert!(c.pixels[3] > 0 && c.pixels[3] < 255);

        // A 2 px border centred on the contour: it covers pixels 1 to 5, the
        // hole is pixel 3.
        let mut c = canvas(7, 7);
        c.fill(&ring(rect(200, 200, 300, 300), 200), 0);
        assert_eq!(c.pixels[3 * 7 + 3], 255, "le centre est un trou");
        assert_eq!(c.pixels[7 + 1], 0, "le bord est plein");
        assert_eq!(c.pixels[2 * 7 + 5], 0, "le bord est plein");
        assert_eq!(c.pixels[0], 255, "rien hors du bord");
        assert_eq!(c.pixels[6 * 7 + 6], 255, "rien hors du bord");
    }

    #[test]
    fn a_too_thin_rectangle_has_no_hole() {
        assert_eq!(ring(rect(0, 0, 1, 1), 100).len(), 1);
    }

    #[test]
    fn overlapping_quads_do_not_cancel() {
        // Two opposite segments: the quadrilaterals overlap, both oriented the
        // same way, so the winding stays non-zero.
        let mut c = canvas(4, 1);
        let a = Point { x: 0, y: 50 };
        let b = Point { x: 400, y: 50 };
        c.fill(&stroke(&[a, b, a], 100), 0);
        assert_eq!(c.pixels, [0, 0, 0, 0]);
    }

    #[test]
    fn dashes_follow_the_perimeter() {
        let style = Style {
            dash: 100,
            gap: 100,
            thin: 100,
            ..Style::default()
        };
        // Perimeter of 800: four dashes.
        assert_eq!(dashes(rect(0, 0, 200, 200), &style).len(), 4);
    }

    #[test]
    fn the_encoder_refuses_a_short_buffer() {
        let diag = encode(2, 2, &[0; 3]).unwrap_err();
        assert_eq!(diag.code, "DRW003");
        assert!(encode(2, 2, &[0; 4]).is_ok());
    }
}
