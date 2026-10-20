//! 최상위 메시지 디스패처.
//!
//! 앱 전역 메시지(종료, 에러, 파일 I/O 결과)는 여기서 직접 처리하고,
//! 그 외 화면별 메시지는 현재 [`Screen`]에 따라 기능 모듈로 위임한다.
//! 위임 대상 메시지는 모두 해당 화면에서만 의미가 있으므로(원래 각 처리부가
//! 화면 가드를 갖고 있었다) 화면 기준 라우팅이 동작을 보존한다.

use crate::{app::{Model,
                  Screen},
            calendar,
            editor,
            message::Msg};
use chrono::NaiveDate;

pub enum Command {
    LoadPreview(NaiveDate),
    SaveSettings,
    LoadDiary(NaiveDate),
    SaveDiary(NaiveDate, String),
    DeleteDiary(NaiveDate),
    Search(u64, String),
}

pub fn update(model: &mut Model, msg: Msg) -> Option<Command> {
    match msg {
        | Msg::OpenSearch if model.screen == Screen::Calendar => {
            model.screen = Screen::Search;
            model.search.editing = true;
            model.editor_state.submode = None;
            model.editor_state.helix.prefix = None;
            None
        },
        | Msg::SearchFinished(id, result) => {
            if model.screen == Screen::Search && id == model.search.revision {
                model.search.busy = false;
                match result {
                    | Ok(output) => {
                        model.search.results = output.results;
                        model.search.notice = output.notice;
                    },
                    | Err(error) => model.search.notice = Some(error),
                }
            }
            None
        },
        | Msg::ToggleSettings => {
            model.show_settings = !model.show_settings;
            None
        },
        | Msg::SettingsMove => {
            model.settings_row = (model.settings_row + 1) % 3;
            None
        },
        | Msg::SettingsMoveBack => {
            model.settings_row = (model.settings_row + 2) % 3;
            None
        },
        | Msg::SettingsChange(forward) => {
            use crate::settings::{Language,
                                  Theme};
            if model.settings_row == 0 {
                model.settings.language = match model.settings.language {
                    | Language::Korean => Language::English,
                    | Language::English => Language::Korean,
                };
            } else if model.settings_row == 1 {
                model.settings.theme = match (model.settings.theme, forward) {
                    | (Theme::System, true) | (Theme::Light, false) => Theme::Dark,
                    | (Theme::Dark, true) | (Theme::System, false) => Theme::Light,
                    | _ => Theme::System,
                };
            } else {
                use crate::settings::Keybindings;
                model.settings.keybindings = match model.settings.keybindings {
                    | Keybindings::Emacs => Keybindings::Helix,
                    | Keybindings::Helix => Keybindings::Emacs,
                };
                editor::helix::reset(&mut model.editor_state, model.settings.keybindings == Keybindings::Helix);
            }
            Some(Command::SaveSettings)
        },
        | Msg::SettingsSaveFailed(error) => {
            model.error_message = Some(format!("{}: {}", model.settings.language.tr(crate::i18n::Text::FailedToSaveSettings), error));
            model.show_error_popup = true;
            None
        },
        | Msg::Quit => {
            model.editor_state.submode = None;
            super::navigate(model, super::Navigation::Quit, false)
        },
        | Msg::EditorEnterCtrlXMode => {
            model.editor_state.submode = Some(editor::state::EditorSubMode::CtrlX);
            None
        },
        | Msg::EditorExitSubMode => {
            model.editor_state.submode = None;
            None
        },
        | Msg::EditorKeyboardQuit => {
            let state = &mut model.editor_state;
            if let Some((line, col)) = state.search_origin.take() {
                state.cursor_line = line;
                state.cursor_col = col;
            }
            state.submode = None;
            state.selection = None;
            state.search_matches.clear();
            state.search_pattern.clear();
            None
        },
        | Msg::EditorCtrlXBack if model.screen == Screen::Editor => {
            model.editor_state.submode = None;
            super::navigate(model, super::Navigation::Calendar, false)
        },
        | Msg::EditorUndo if model.editor_state.submode == Some(editor::state::EditorSubMode::CtrlX) => {
            model.editor_state.submode = None;
            model.editor_state.undo();
            None
        },

        | Msg::DismissError => {
            model.show_error_popup = false;
            model.error_message = None;
            None
        },

        // ===== 파일 I/O 결과 =====
        | Msg::LoadDiarySuccess(date, content, format) => {
            if model.screen == Screen::Editor {
                let target = match model.search.target.take() {
                    | Some(hit) => match crate::search::locate(&hit, &content) {
                        | Some(position) => Some(position),
                        | None =>
                            return update(
                                model,
                                Msg::LoadDiaryFailed(
                                    model
                                        .settings
                                        .language
                                        .text(
                                            "검색한 구절이 변경되었거나 삭제되었습니다. 다시 검색하세요.",
                                            "The passage changed or was deleted. Please search again.",
                                        )
                                        .to_string(),
                                ),
                            ),
                    },
                    | None => None,
                };
                model.editor_state.date = date;
                model.editor_state.format = format;
                model.editor_state.load_content(&content);
                if let Some((line, col)) = target {
                    model.editor_state.cursor_line = line.min(model.editor_state.content.len().saturating_sub(1));
                    model.editor_state.cursor_col = col.min(model.editor_state.content[model.editor_state.cursor_line].chars().count());
                }
                model.editor_state.submode = None;
                model.editor_state.search_origin = None;
                if model.settings.keybindings == crate::settings::Keybindings::Helix {
                    editor::helix::reset(&mut model.editor_state, true);
                }
            }
            None
        },
        | Msg::LoadDiaryFailed(error) => {
            model.search.target = None;
            model.screen = Screen::Calendar;
            model.error_message = Some(format!("{}: {}", model.settings.language.tr(crate::i18n::Text::LoadFailed), error));
            model.show_error_popup = true;
            None
        },
        | Msg::SaveDiarySuccess(date) => {
            model.diary_entries.entries.insert(date);
            if date == model.editor_state.date {
                model.editor_state.is_modified = false;
                if let Some(navigation) = model.pending_navigation.take() {
                    return super::finish_navigation(model, navigation);
                }
            }
            None
        },
        | Msg::SaveDiaryFailed(error) => {
            model.pending_navigation = None;
            model.error_message = Some(format!("{}: {}", model.settings.language.tr(crate::i18n::Text::SaveFailed), error));
            model.show_error_popup = true;
            None
        },
        | Msg::DeleteDiarySuccess(date) => {
            model.diary_entries.entries.remove(&date);
            model.screen = Screen::Calendar;
            Some(Command::LoadPreview(model.calendar_state.selected_date))
        },
        | Msg::RefreshIndex(entries) => {
            model.diary_entries.entries = entries;
            None
        },

        | Msg::PreviewLoaded(date, entry) => {
            if date == model.calendar_state.selected_date {
                model.preview = entry.map(|entry| (date, entry));
            }
            None
        },

        | Msg::HelixKey(key) => editor::helix::update(key, model),

        // ===== 화면별 메시지 위임 =====
        | other => match model.screen {
            | Screen::Calendar => calendar::update::update(model, other),
            | Screen::Editor => editor::update::update(model, other),
            | Screen::Search => crate::search::update(model, other),
        },
    }
}
