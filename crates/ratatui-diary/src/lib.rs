pub mod app;
pub mod calendar;
pub mod editor;
pub mod input;
mod key_help;
mod search;
pub mod settings;
pub mod theme;

pub use diary_application::{Model,
                            Msg,
                            message,
                            model,
                            update};
pub use diary_domain::i18n;
pub use diary_runtime::Runtime;
pub use diary_storage as storage;
pub use markdown_render as markdown;
pub use org_render as org;

pub mod view {
    pub use crate::{app::view::view,
                    calendar::view::build_calendar_keybindings,
                    editor::view::build_editor_keybindings};
}
