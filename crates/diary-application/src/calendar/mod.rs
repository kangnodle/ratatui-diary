//! Calendar state transitions and input policy.
pub mod keymap;
pub mod update;
pub use diary_domain::calendar as state;
pub use state::CalendarState;
