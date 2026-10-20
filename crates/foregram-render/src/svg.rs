// ForegramPM — SVG output for a scene.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use foregram_graph::{Point, Rect, Segment};

use crate::scene::{Class, Scene, Shape, Style};

/// The SVG of a scene: coordinates in pixels, written from hundredths of a
/// pixel without going through a float; one stylesheet rule per class, which
/// the bench can replace; a `<title>` on each element that has an identifier.
pub(crate) fn write(scene: &Scene, style: &Style) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\">\n",
        px(scene.view.width),
        px(scene.view.height),
        px(scene.view.x),
        px(scene.view.y),
        px(scene.view.width),
        px(scene.view.height)
    ));
    out.push_str(&format!("<title>{}</title>\n", escape(&scene.title)));
    out.push_str(&stylesheet(style));
    for item in &scene.items {
        let class = item.class.name();
        let title = item
            .title
            .as_deref()
            .map(|t| format!("<title>{}</title>", escape(t)));
        let open = match &item.shape {
            Shape::Rect(rect) => rect_attrs(*rect, class),
            Shape::Route(route) => format!("<path class=\"{class}\" d=\"{}\"", path(route)),
            Shape::Polygon(points) => {
                format!("<polygon class=\"{class}\" points=\"{}\"", list(points))
            }
        };
        match title {
            Some(title) => out.push_str(&format!("{open}>{title}</{}>\n", tag(&item.shape))),
            None => out.push_str(&format!("{open}/>\n")),
        }
    }
    out.push_str("</svg>\n");
    out
}

fn tag(shape: &Shape) -> &'static str {
    match shape {
        Shape::Rect(_) => "rect",
        Shape::Route(_) => "path",
        Shape::Polygon(_) => "polygon",
    }
}

fn rect_attrs(rect: Rect, class: &str) -> String {
    format!(
        "<rect class=\"{class}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
        px(rect.x),
        px(rect.y),
        px(rect.width),
        px(rect.height)
    )
}

/// The stylesheet: one rule per class, the values coming from the style.
fn stylesheet(style: &Style) -> String {
    let rules = [
        (Class::Sheet, format!("fill:{}", gray(style.sheet))),
        (
            Class::Leaf,
            format!(
                "fill:{};stroke:{};stroke-width:{}",
                gray(style.leaf_fill),
                gray(style.ink),
                px(style.thick)
            ),
        ),
        (
            Class::Container,
            format!(
                "fill:none;stroke:{};stroke-width:{}",
                gray(style.container_ink),
                px(style.thin)
            ),
        ),
        (
            Class::Edge,
            format!(
                "fill:none;stroke:{};stroke-width:{};stroke-linejoin:round",
                gray(style.ink),
                px(style.thick)
            ),
        ),
        (Class::Arrow, format!("fill:{}", gray(style.ink))),
        (
            Class::Label,
            format!(
                "fill:none;stroke:{};stroke-width:{};stroke-dasharray:{} {}",
                gray(style.ink),
                px(style.thin),
                px(style.dash),
                px(style.gap)
            ),
        ),
        (
            Class::Port,
            format!(
                "fill:{};stroke:{};stroke-width:{}",
                gray(style.sheet),
                gray(style.ink),
                px(style.thin)
            ),
        ),
    ];
    let mut out = String::from("<style>\n");
    for (class, rule) in rules {
        out.push_str(&format!(".{}{{{rule}}}\n", class.name()));
    }
    out.push_str("</style>\n");
    out
}

/// The SVG path of a route: `M`, then `L` or `C` for each segment.
fn path(route: &[Segment]) -> String {
    let mut out = String::new();
    for (i, segment) in route.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        match segment {
            Segment::Line(p) if i == 0 => out.push_str(&format!("M{} {}", px(p.x), px(p.y))),
            Segment::Line(p) => out.push_str(&format!("L{} {}", px(p.x), px(p.y))),
            Segment::Cubic { c1, c2, end } => out.push_str(&format!(
                "C{} {} {} {} {} {}",
                px(c1.x),
                px(c1.y),
                px(c2.x),
                px(c2.y),
                px(end.x),
                px(end.y)
            )),
        }
    }
    out
}

fn list(points: &[Point]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", px(p.x), px(p.y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A gray as `#rrggbb`.
fn gray(level: u8) -> String {
    format!("#{level:02x}{level:02x}{level:02x}")
}

/// Hundredths of a pixel written as pixels: `4950` gives `49.5`, `100`
/// gives `1`, `-150` gives `-1.5`. Never a float.
pub(crate) fn px(hundredths: i64) -> String {
    let sign = if hundredths < 0 { "-" } else { "" };
    let abs = hundredths.unsigned_abs();
    let (whole, frac) = (abs / 100, abs % 100);
    if frac == 0 {
        format!("{sign}{whole}")
    } else if frac % 10 == 0 {
        format!("{sign}{whole}.{}", frac / 10)
    } else {
        format!("{sign}{whole}.{frac:02}")
    }
}

/// The text of a `<title>`, escaped for XML.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hundredths_become_pixels_without_floats() {
        assert_eq!(px(4950), "49.5");
        assert_eq!(px(100), "1");
        assert_eq!(px(0), "0");
        assert_eq!(px(-150), "-1.5");
        assert_eq!(px(-5), "-0.05");
        assert_eq!(px(1234), "12.34");
        assert_eq!(px(1230), "12.3");
        assert_eq!(px(i64::MIN), "-92233720368547758.08");
    }

    #[test]
    fn titles_are_escaped() {
        assert_eq!(escape("a<b>&c"), "a&lt;b&gt;&amp;c");
    }
}
