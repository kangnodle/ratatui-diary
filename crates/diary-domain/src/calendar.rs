use chrono::{Datelike,
             Days,
             Months,
             NaiveDate};

pub struct CalendarState {
    pub current_year: i32,
    pub current_month: u32,
    pub selected_date: NaiveDate,
    pub cursor_pos: usize,
}

impl CalendarState {
    pub fn new(year: i32, month: u32) -> Self {
        let selected_date = match NaiveDate::from_ymd_opt(year, month, 1) {
            | Some(date) => date,
            | None => NaiveDate::default(),
        };
        Self {
            current_year: selected_date.year(),
            current_month: selected_date.month(),
            selected_date,
            cursor_pos: 0,
        }
    }

    pub fn next_month(&mut self) { self.select_date(self.selected_date.checked_add_months(Months::new(1))); }

    pub fn prev_month(&mut self) { self.select_date(self.selected_date.checked_sub_months(Months::new(1))); }

    pub fn next_year(&mut self) { self.select_date(self.selected_date.checked_add_months(Months::new(12))); }

    pub fn prev_year(&mut self) { self.select_date(self.selected_date.checked_sub_months(Months::new(12))); }

    pub fn move_cursor_left(&mut self) { self.select_date(self.selected_date.pred_opt()); }

    pub fn move_cursor_right(&mut self) { self.select_date(self.selected_date.succ_opt()); }

    pub fn move_cursor_up(&mut self) { self.select_date(self.selected_date.checked_sub_days(Days::new(7))); }

    pub fn move_cursor_down(&mut self) { self.select_date(self.selected_date.checked_add_days(Days::new(7))); }

    fn select_date(&mut self, date: Option<NaiveDate>) {
        self.selected_date = match date {
            | Some(date) => date,
            | None => self.selected_date,
        };
        self.current_year = self.selected_date.year();
        self.current_month = self.selected_date.month();
    }
}
