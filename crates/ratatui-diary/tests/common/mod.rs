//! Integration harness: production keeps the pure model and effect runtime
//! separate.
#![allow(dead_code)]
use chrono::NaiveDate;
use ratatui_diary::{Model,
                    Runtime,
                    app::Command,
                    storage::Storage};
use std::{collections::HashSet,
          ops::{Deref,
                DerefMut}};

pub struct TestApp {
    pub model: Model,
    pub runtime: Runtime,
}

impl TestApp {
    pub fn new(entries: HashSet<NaiveDate>, storage: Storage) -> Self {
        let mut model = Model::new(chrono::Local::now().date_naive(), entries);
        let mut runtime = Runtime::new(storage, None);
        runtime.execute(Command::LoadPreview(model.calendar_state.selected_date), &mut model);
        Self {
            model,
            runtime,
        }
    }
}

impl Deref for TestApp {
    type Target = Model;

    fn deref(&self) -> &Self::Target { &self.model }
}
impl DerefMut for TestApp {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.model }
}

pub fn execute_command(command: Command, app: &mut TestApp) { app.runtime.execute(command, &mut app.model); }
