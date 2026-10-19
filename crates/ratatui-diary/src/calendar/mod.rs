//! Calendar presentation, backed by pure domain/application state.
pub use diary_application::calendar::{CalendarState,
                                      keymap,
                                      state,
                                      update};
pub mod view;
