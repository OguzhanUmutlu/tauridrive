use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

impl MouseButton {
    pub fn as_str(&self) -> &'static str {
        match self {
            MouseButton::Left => "left",
            MouseButton::Middle => "middle",
            MouseButton::Right => "right",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifiers {
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
}

impl Modifiers {
    pub fn to_int(&self) -> i32 {
        let mut val = 0;
        if self.alt {
            val |= 1;
        }
        if self.ctrl {
            val |= 2;
        }
        if self.meta {
            val |= 4;
        }
        if self.shift {
            val |= 8;
        }
        val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_button_as_str() {
        assert_eq!(MouseButton::Left.as_str(), "left");
        assert_eq!(MouseButton::Middle.as_str(), "middle");
        assert_eq!(MouseButton::Right.as_str(), "right");
    }

    #[test]
    fn test_modifiers_bitmask() {
        let m = Modifiers {
            alt: true,
            ctrl: false,
            meta: true,
            shift: true,
        };
        // alt(1) + meta(4) + shift(8) = 13
        assert_eq!(m.to_int(), 13);

        let empty = Modifiers::default();
        assert_eq!(empty.to_int(), 0);
    }
}
