use chrono::{Datelike,
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

    pub fn next_month(&mut self) {
        if self.current_month == 12 {
            self.current_month = 1;
            self.current_year += 1;
        } else {
            self.current_month += 1;
        }
        self.adjust_selected_date();
    }

    pub fn prev_month(&mut self) {
        if self.current_month == 1 {
            self.current_month = 12;
            self.current_year -= 1;
        } else {
            self.current_month -= 1;
        }
        self.adjust_selected_date();
    }

    pub fn next_year(&mut self) {
        self.current_year += 1;
        self.adjust_selected_date();
    }

    pub fn prev_year(&mut self) {
        self.current_year -= 1;
        self.adjust_selected_date();
    }

    pub fn move_cursor_left(&mut self) {
        self.selected_date = match self.selected_date.pred_opt() {
            | Some(date) => date,
            | None => self.selected_date,
        };
    }

    pub fn move_cursor_right(&mut self) {
        self.selected_date = match self.selected_date.succ_opt() {
            | Some(date) => date,
            | None => self.selected_date,
        };
    }

    pub fn move_cursor_up(&mut self) {
        self.selected_date = match self.selected_date.checked_sub_days(chrono::Days::new(7)) {
            | Some(date) => date,
            | None => self.selected_date,
        };
    }

    pub fn move_cursor_down(&mut self) {
        self.selected_date = match self.selected_date.checked_add_days(chrono::Days::new(7)) {
            | Some(date) => date,
            | None => self.selected_date,
        };
    }

    fn adjust_selected_date(&mut self) {
        let day = self.selected_date.day();
        let target_day = day.min(days_in_month(self.current_year, self.current_month));
        self.selected_date = match NaiveDate::from_ymd_opt(self.current_year, self.current_month, target_day) {
            | Some(date) => date,
            | None => self.selected_date,
        };
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month_first = match NaiveDate::from_ymd_opt(year, month + 1, 1) {
        | Some(date) => date,
        | None => match NaiveDate::from_ymd_opt(year + 1, 1, 1) {
            | Some(date) => date,
            | None => return 31,
        },
    };
    match next_month_first.pred_opt() {
        | Some(last_day) => last_day.day(),
        | None => 31,
    }
}
