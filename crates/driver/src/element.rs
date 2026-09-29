use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ElementRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ElementRect {
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_center() {
        let rect = ElementRect {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 30.0,
        };
        let (cx, cy) = rect.center();
        assert!((cx - 125.0).abs() < 1e-6);
        assert!((cy - 215.0).abs() < 1e-6);
    }

    #[test]
    fn test_element_rect_serde() {
        let json = r#"{"x":10.5,"y":20.5,"width":100.0,"height":50.0}"#;
        let rect: ElementRect = serde_json::from_str(json).unwrap();
        assert_eq!(rect.x, 10.5);
        assert_eq!(rect.y, 20.5);
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 50.0);
    }
}
