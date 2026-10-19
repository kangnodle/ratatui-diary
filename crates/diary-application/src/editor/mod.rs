//! Diary editing state and Emacs/Helix input policy.
pub mod helix;
pub mod keymap;
pub mod state;
pub mod update;
pub use state::{EditorState,
                EditorSubMode,
                Selection};
