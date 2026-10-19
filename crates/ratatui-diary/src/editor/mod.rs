//! Editor presentation, backed by pure application state.
pub use diary_application::editor::{EditorState,
                                    EditorSubMode,
                                    Selection,
                                    helix,
                                    keymap,
                                    state,
                                    update};
pub mod view;
