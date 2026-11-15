/// Vector drawing engine for SigmaOS (LibreOffice Draw equivalent).

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Rectangle {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Circle {
        cx: f64,
        cy: f64,
        radius: f64,
    },
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Text {
        x: f64,
        y: f64,
        content: String,
    },
}

impl Shape {
    /// Returns the area of the shape.
    /// Rectangle: width * height, Circle: PI * r^2, Line/Text: 0.0
    pub fn area(&self) -> f64 {
        match self {
            Shape::Rectangle { width, height, .. } => width * height,
            Shape::Circle { radius, .. } => std::f64::consts::PI * radius * radius,
            Shape::Line { .. } => 0.0,
            Shape::Text { .. } => 0.0,
        }
    }

    /// Returns the bounding box as (x, y, width, height).
    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        match self {
            Shape::Rectangle {
                x,
                y,
                width,
                height,
            } => (*x, *y, *width, *height),
            Shape::Circle { cx, cy, radius } => {
                (cx - radius, cy - radius, radius * 2.0, radius * 2.0)
            }
            Shape::Line { x1, y1, x2, y2 } => {
                let x = x1.min(*x2);
                let y = y1.min(*y2);
                let w = (x2 - x1).abs();
                let h = (y2 - y1).abs();
                (x, y, w, h)
            }
            Shape::Text { x, y, .. } => (*x, *y, 0.0, 0.0),
        }
    }
}

pub struct DrawDocument {
    shapes: Vec<Shape>,
    pub title: String,
}

impl DrawDocument {
    pub fn new(title: &str) -> Self {
        Self {
            shapes: Vec::new(),
            title: title.to_string(),
        }
    }

    pub fn add_shape(&mut self, shape: Shape) {
        self.shapes.push(shape);
    }

    pub fn remove_shape(&mut self, index: usize) -> Result<Shape, String> {
        if index >= self.shapes.len() {
            return Err(format!(
                "Index {} out of bounds (len {})",
                index,
                self.shapes.len()
            ));
        }
        Ok(self.shapes.remove(index))
    }

    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }

    pub fn total_area(&self) -> f64 {
        self.shapes.iter().map(|s| s.area()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_shape_aaa() {
        // Arrange
        let mut doc = DrawDocument::new("test");
        let rect = Shape::Rectangle {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 5.0,
        };

        // Act
        doc.add_shape(rect);

        // Assert
        assert_eq!(doc.shape_count(), 1);
    }

    #[test]
    fn test_shape_area_aaa() {
        // Arrange
        let rect = Shape::Rectangle {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 3.0,
        };
        let circle = Shape::Circle {
            cx: 0.0,
            cy: 0.0,
            radius: 1.0,
        };

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
        doc.add_shape(Shape::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        });

        // Act
        let result = doc.remove_shape(0);

        // Assert
        assert!(result.is_ok());
        assert_eq!(doc.shape_count(), 0);
    }
}
