#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    F(u8),
    Char(char),
    Null,
    Esc,
    Unknown,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    pub const ALT: Self = Self(4);
    pub const CONTROL: Self = Self(2);
    pub const NONE: Self = Self(0);
    pub const OTHER: Self = Self(8);
    pub const SHIFT: Self = Self(1);

    pub fn is_empty(self) -> bool { self == Self::NONE }

    pub fn contains(self, other: Self) -> bool { self.0 & other.0 == other.0 }
}

impl std::ops::BitOr for KeyModifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self {
            code,
            modifiers,
        }
    }
}
