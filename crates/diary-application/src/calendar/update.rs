use crate::{app::{Command,
                  Model,
                  entry},
            message::Msg};

pub fn update(model: &mut Model, msg: Msg) -> Option<Command> {
    match msg {
        | Msg::CalendarMoveLeft => model.calendar_state.move_cursor_left(),
        | Msg::CalendarMoveRight => model.calendar_state.move_cursor_right(),
        | Msg::CalendarMoveUp => model.calendar_state.move_cursor_up(),
        | Msg::CalendarMoveDown => model.calendar_state.move_cursor_down(),
        | Msg::CalendarSelectDate => {
            let date = model.calendar_state.selected_date;
            return Some(entry::open(model, date, None));
        },
        | Msg::CalendarNextMonth => model.calendar_state.next_month(),
        | Msg::CalendarPrevMonth => model.calendar_state.prev_month(),
        | Msg::CalendarNextYear => model.calendar_state.next_year(),
        | Msg::CalendarPrevYear => model.calendar_state.prev_year(),
        | _ => return None,
    }

    Some(Command::LoadPreview(model.calendar_state.selected_date))
}
