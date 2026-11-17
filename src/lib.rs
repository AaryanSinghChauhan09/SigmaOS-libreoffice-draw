//! Vector drawing document engine for SigmaOS.
//!
//! Shapes with styles, layers, document bounds, SVG export, and line-based
//! persistence. Dependency-free.

use std::fmt;
use std::fs;
use std::path::Path;

/// Vector shape primitives.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Rectangle { x: f64, y: f64, width: f64, height: f64 },
    Circle { cx: f64, cy: f64, radius: f64 },
    Line { x1: f64, y1: f64, x2: f64, y2: f64 },
    Text { x: f64, y: f64, content: String },
}

impl Shape {
    /// Geometric area; lines and text report zero.
    pub fn area(&self) -> f64 {
        match self {
            Shape::Rectangle { width, height, .. } => width * height,
            Shape::Circle { radius, .. } => std::f64::consts::PI * radius * radius,
            Shape::Line { .. } | Shape::Text { .. } => 0.0,
        }
    }

    /// Bounding box as (x, y, width, height).
    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        match self {
            Shape::Rectangle { x, y, width, height } => (*x, *y, *width, *height),
            Shape::Circle { cx, cy, radius } => (cx - radius, cy - radius, radius * 2.0, radius * 2.0),
            Shape::Line { x1, y1, x2, y2 } => {
                let x = x1.min(*x2);
                let y = y1.min(*y2);
                (x, y, (x2 - x1).abs(), (y2 - y1).abs())
            }
            Shape::Text { x, y, .. } => (*x, *y, 0.0, 0.0),
        }
    }
}

/// Stroke/fill style for a shape.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeStyle {
    pub stroke: String,
    pub fill: Option<String>,
    pub stroke_width: f64,
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self { stroke: "#000000".to_string(), fill: None, stroke_width: 1.0 }
    }
}

/// A shape placed on a layer with a style.
#[derive(Debug, Clone, PartialEq)]
struct PlacedShape {
    shape: Shape,
    layer: String,
    style: ShapeStyle,
}

/// Errors for drawing operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawError {
    InvalidName(String),
    InvalidIndex(usize),
    UnknownLayer(String),
    Io(String),
    Format(String),
}

impl fmt::Display for DrawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(n) => write!(f, "invalid name: {n}"),
            Self::InvalidIndex(i) => write!(f, "invalid shape index: {i}"),
            Self::UnknownLayer(l) => write!(f, "unknown layer: {l}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Format(e) => write!(f, "format error: {e}"),
        }
    }
}

impl std::error::Error for DrawError {}

const MAGIC: &str = "SIGDRAW1";
const DEFAULT_LAYER: &str = "default";

/// A drawing document with layers and styles.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawDocument {
    shapes: Vec<PlacedShape>,
    layers: Vec<String>,
    active_layer: String,
    pub title: String,
}

impl DrawDocument {
    pub fn new(title: &str) -> Self {
        Self {
            shapes: Vec::new(),
            layers: vec![DEFAULT_LAYER.to_string()],
            active_layer: DEFAULT_LAYER.to_string(),
            title: title.to_string(),
        }
    }

    pub fn add_layer(&mut self, name: &str) -> Result<(), DrawError> {
        validate_name(name)?;
        if !self.layers.iter().any(|l| l == name) {
            self.layers.push(name.to_string());
        }
        Ok(())
    }

    pub fn remove_layer(&mut self, name: &str) -> Result<(), DrawError> {
        if name == DEFAULT_LAYER {
            return Err(DrawError::InvalidName("cannot remove default layer".to_string()));
        }
        if !self.layers.iter().any(|l| l == name) {
            return Err(DrawError::UnknownLayer(name.to_string()));
        }
        self.shapes.retain(|s| s.layer != name);
        self.layers.retain(|l| l != name);
        if self.active_layer == name {
            self.active_layer = DEFAULT_LAYER.to_string();
        }
        Ok(())
    }

    pub fn layers(&self) -> &[String] {
        &self.layers
    }

    pub fn set_active_layer(&mut self, name: &str) -> Result<(), DrawError> {
        if !self.layers.iter().any(|l| l == name) {
            return Err(DrawError::UnknownLayer(name.to_string()));
        }
        self.active_layer = name.to_string();
        Ok(())
    }

    pub fn active_layer(&self) -> &str {
        &self.active_layer
    }

    pub fn add_shape(&mut self, shape: Shape) {
        self.shapes.push(PlacedShape {
            shape,
            layer: self.active_layer.clone(),
            style: ShapeStyle::default(),
        });
    }

    pub fn add_styled_shape(&mut self, shape: Shape, style: ShapeStyle) {
        self.shapes.push(PlacedShape { shape, layer: self.active_layer.clone(), style });
    }

    pub fn shapes_in_layer(&self, layer: &str) -> usize {
        self.shapes.iter().filter(|s| s.layer == layer).count()
    }

    pub fn remove_shape(&mut self, index: usize) -> Result<Shape, String> {
        if index >= self.shapes.len() {
            return Err(format!("Index {} out of bounds (len {})", index, self.shapes.len()));
        }
        Ok(self.shapes.remove(index).shape)
    }

    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }

    pub fn total_area(&self) -> f64 {
        self.shapes.iter().map(|s| s.shape.area()).sum()
    }

    /// Document bounding box as (x, y, width, height), or None when empty.
    pub fn bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let mut iter = self.shapes.iter().map(|s| s.shape.bounding_box());
        let (x, y, w, h) = iter.next()?;
        let (mut min_x, mut min_y) = (x, y);
        let (mut max_x, mut max_y) = (x + w, y + h);
        for (bx, by, bw, bh) in iter {
            min_x = min_x.min(bx);
            min_y = min_y.min(by);
            max_x = max_x.max(bx + bw);
            max_y = max_y.max(by + bh);
        }
        Some((min_x, min_y, max_x - min_x, max_y - min_y))
    }

    /// Export the document as a self-contained SVG image.
    pub fn to_svg(&self, width: f64, height: f64) -> Result<String, DrawError> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(DrawError::Format("svg size must be positive".to_string()));
        }
        let mut out = String::new();
        out.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
            fmt_num(width), fmt_num(height), fmt_num(width), fmt_num(height)
        ));
        out.push_str(&format!("<title>{}</title>\n", escape(&self.title)));
        for layer in &self.layers {
            let placed: Vec<&PlacedShape> = self.shapes.iter().filter(|s| &s.layer == layer).collect();
            if placed.is_empty() {
                continue;
            }
            out.push_str(&format!("<g id=\"{}\">\n", escape(layer)));
            for p in placed {
                out.push_str(&shape_svg(&p.shape, &p.style));
            }
            out.push_str("</g>\n");
        }
        out.push_str("</svg>\n");
        Ok(out)
    }

    /// Serialize to the line-based persistence format.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), DrawError> {
        let mut out = String::new();
        out.push_str(MAGIC);
        out.push('\n');
        out.push_str(&format!("TITLE|{}\n", encode_field(&self.title)));
        out.push_str(&format!("LAYER|{}\n", encode_field(&self.active_layer)));
        for l in &self.layers {
            out.push_str(&format!("LAYERS|{}\n", encode_field(l)));
        }
        for p in &self.shapes {
            let args: String = match &p.shape {
                Shape::Rectangle { x, y, width, height } =>
                    format!("rect|{}|{}|{}|{}", fmt_num(*x), fmt_num(*y), fmt_num(*width), fmt_num(*height)),
                Shape::Circle { cx, cy, radius } =>
                    format!("circle|{}|{}|{}", fmt_num(*cx), fmt_num(*cy), fmt_num(*radius)),
                Shape::Line { x1, y1, x2, y2 } =>
                    format!("line|{}|{}|{}|{}", fmt_num(*x1), fmt_num(*y1), fmt_num(*x2), fmt_num(*y2)),
                Shape::Text { x, y, content } =>
                    format!("text|{}|{}|{}", fmt_num(*x), fmt_num(*y), encode_field(content)),
            };
            let style = &p.style;
            out.push_str(&format!(
                "SHAPE|{}|{}|{}|{}|{}\n",
                encode_field(&p.layer),
                encode_field(&style.stroke),
                style.fill.as_deref().map(encode_field).unwrap_or_default(),
                fmt_num(style.stroke_width),
                args
            ));
        }
        fs::write(path, out).map_err(|e| DrawError::Io(e.to_string()))
    }

    /// Load a document saved with [`DrawDocument::save`].
    pub fn load(path: impl AsRef<Path>) -> Result<Self, DrawError> {
        let text = fs::read_to_string(path).map_err(|e| DrawError::Io(e.to_string()))?;
        let mut lines = text.lines();
        if lines.next() != Some(MAGIC) {
            return Err(DrawError::Format("bad magic".to_string()));
        }
        let mut title = String::new();
        let mut active = DEFAULT_LAYER.to_string();
        let mut layers = vec![DEFAULT_LAYER.to_string()];
        let mut shapes = Vec::new();
        for line in lines {
            if let Some(rest) = line.strip_prefix("TITLE|") {
                title = decode_field(rest);
            } else if let Some(rest) = line.strip_prefix("LAYER|") {
                active = decode_field(rest);
            } else if let Some(rest) = line.strip_prefix("LAYERS|") {
                let l = decode_field(rest);
                if !layers.contains(&l) {
                    layers.push(l);
                }
            } else if let Some(rest) = line.strip_prefix("SHAPE|") {
                shapes.push(parse_shape_line(rest)?);
            }
        }
        if !layers.contains(&active) {
            active = DEFAULT_LAYER.to_string();
        }
        Ok(Self { shapes, layers, active_layer: active, title })
    }
}

fn validate_name(name: &str) -> Result<(), DrawError> {
    if name.trim().is_empty() || name.contains(['\n', '\r', '|']) {
        return Err(DrawError::InvalidName(name.to_string()));
    }
    Ok(())
}

fn parse_shape_line(rest: &str) -> Result<PlacedShape, DrawError> {
    // SHAPE|layer|stroke|fill|width|kind|arg1|arg2|...
    let mut parts = rest.split('|');
    let layer = decode_field(parts.next().ok_or_else(|| DrawError::Format("shape missing layer".into()))?);
    let stroke = decode_field(parts.next().unwrap_or("#000000"));
    let fill_raw = parts.next().unwrap_or("");
    let fill = if fill_raw.is_empty() { None } else { Some(decode_field(fill_raw)) };
    let width = parts.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.0);
    let kind = parts.next().ok_or_else(|| DrawError::Format("shape missing kind".into()))?;
    let args: Vec<&str> = parts.collect();
    let style = ShapeStyle { stroke, fill, stroke_width: width };
    let shape = match kind {
        "rect" if args.len() >= 4 => Shape::Rectangle {
            x: parse_num(args[0])?, y: parse_num(args[1])?, width: parse_num(args[2])?, height: parse_num(args[3])?,
        },
        "circle" if args.len() >= 3 => Shape::Circle {
            cx: parse_num(args[0])?, cy: parse_num(args[1])?, radius: parse_num(args[2])?,
        },
        "line" if args.len() >= 4 => Shape::Line {
            x1: parse_num(args[0])?, y1: parse_num(args[1])?, x2: parse_num(args[2])?, y2: parse_num(args[3])?,
        },
        "text" if args.len() >= 3 => Shape::Text {
            x: parse_num(args[0])?,
            y: parse_num(args[1])?,
            content: decode_field(args[2]),
        },
        _ => return Err(DrawError::Format(format!("unknown shape kind: {kind}"))),
    };
    Ok(PlacedShape { shape, layer, style })
}

fn parse_num(s: &str) -> Result<f64, DrawError> {
    s.parse::<f64>().map_err(|_| DrawError::Format(format!("bad number: {s}")))
}

fn shape_svg(shape: &Shape, style: &ShapeStyle) -> String {
    let stroke = escape(&style.stroke);
    let fill = style.fill.as_deref().map(|f| format!(" fill=\"{}\"", escape(f))).unwrap_or_default();
    let sw = fmt_num(style.stroke_width);
    match shape {
        Shape::Rectangle { x, y, width, height } => format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"{}\"{} stroke-width=\"{}\"/>\n",
            fmt_num(*x), fmt_num(*y), fmt_num(*width), fmt_num(*height), stroke, fill, sw
        ),
        Shape::Circle { cx, cy, radius } => format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"{}\"{} stroke-width=\"{}\"/>\n",
            fmt_num(*cx), fmt_num(*cy), fmt_num(*radius), stroke, fill, sw
        ),
        Shape::Line { x1, y1, x2, y2 } => format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>\n",
            fmt_num(*x1), fmt_num(*y1), fmt_num(*x2), fmt_num(*y2), stroke, sw
        ),
        Shape::Text { x, y, content } => format!(
            "<text x=\"{}\" y=\"{}\" stroke=\"{}\" fill=\"{}\" font-size=\"12\">{}</text>\n",
            fmt_num(*x), fmt_num(*y), stroke, stroke, escape(content)
        ),
    }
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn encode_field(s: &str) -> String {
    s.replace('\\', "\\\\").replace('|', "\\p").replace('\n', "\\n")
}

fn decode_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('p') => out.push('|'),
                Some('\\') => out.push('\\'),
                Some(other) => { out.push('\\'); out.push(other); }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_shape_aaa() {
        // Arrange
        let mut doc = DrawDocument::new("test");
        let rect = Shape::Rectangle { x: 0.0, y: 0.0, width: 10.0, height: 5.0 };

        // Act
        doc.add_shape(rect);

        // Assert
        assert_eq!(doc.shape_count(), 1);
    }

    #[test]
    fn test_shape_area_aaa() {
        // Arrange
        let rect = Shape::Rectangle { x: 0.0, y: 0.0, width: 4.0, height: 3.0 };
        let circle = Shape::Circle { cx: 0.0, cy: 0.0, radius: 1.0 };

        // Act
        let rect_area = rect.area();
        let circle_area = circle.area();

        // Assert
        assert!((rect_area - 12.0).abs() < 1e-10);
        assert!((circle_area - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_remove_shape_aaa() {
        // Arrange
        let mut doc = DrawDocument::new("test");
        doc.add_shape(Shape::Line { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0 });

        // Act
        let result = doc.remove_shape(0);

        // Assert
        assert!(result.is_ok());
        assert_eq!(doc.shape_count(), 0);
    }

    #[test]
    fn test_layers_aaa() {
        // Arrange
        let mut doc = DrawDocument::new("test");
        doc.add_layer("background").unwrap();
        doc.set_active_layer("background").unwrap();
        doc.add_shape(Shape::Rectangle { x: 0.0, y: 0.0, width: 1.0, height: 1.0 });
        doc.set_active_layer("default").unwrap();
        doc.add_shape(Shape::Circle { cx: 0.0, cy: 0.0, radius: 1.0 });

        // Act
        let bg = doc.shapes_in_layer("background");
        let def = doc.shapes_in_layer("default");

        // Assert
        assert_eq!(bg, 1);
        assert_eq!(def, 1);
        assert_eq!(doc.layers().len(), 2);
        assert!(doc.set_active_layer("nope").is_err());
        assert!(doc.remove_layer("default").is_err());
    }

    #[test]
    fn test_svg_export_aaa() {
        // Arrange
        let mut doc = DrawDocument::new("<Doc>");
        doc.add_shape(Shape::Rectangle { x: 1.0, y: 2.0, width: 3.0, height: 4.0 });
        doc.add_shape(Shape::Text { x: 5.0, y: 6.0, content: "hi & <b>".to_string() });

        // Act
        let svg = doc.to_svg(100.0, 100.0).unwrap();

        // Assert
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("hi &amp; &lt;b&gt;"));
        assert!(svg.contains("&lt;Doc&gt;"));
        assert!(DrawDocument::new("x").to_svg(0.0, 10.0).is_err());
    }

    #[test]
    fn test_save_load_roundtrip_aaa() {
        // Arrange
        let mut doc = DrawDocument::new("round");
        doc.add_layer("fg").unwrap();
        doc.set_active_layer("fg").unwrap();
        doc.add_styled_shape(
            Shape::Circle { cx: 1.5, cy: 2.5, radius: 3.0 },
            ShapeStyle { stroke: "#ff0000".into(), fill: Some("#00ff00".into()), stroke_width: 2.0 },
        );
        doc.add_shape(Shape::Text { x: 1.0, y: 2.0, content: "a|b\nc".to_string() });
        let dir = std::env::temp_dir().join(format!("draw-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("doc.sigdraw");

        // Act
        doc.save(&path).unwrap();
        let loaded = DrawDocument::load(&path).unwrap();

        // Assert
        assert_eq!(loaded.title, "round");
        assert_eq!(loaded.shape_count(), 2);
        assert_eq!(loaded.active_layer(), "fg");
        assert_eq!(loaded.shapes_in_layer("fg"), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
