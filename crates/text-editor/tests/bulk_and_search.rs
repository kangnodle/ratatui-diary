use text_editor::TextEditor;

#[test]
fn bulk_insertion_matches_character_insertion_at_every_unicode_position() {
    for document in ["", "a한글b", "first\n한글 line\nlast", "끝\n"] {
        let mut original = TextEditor::new();
        original.load_content(document);
        for (line, source_line) in original.content.iter().enumerate() {
            for column in 0 ..= source_line.chars().count() {
                for text in ["", "plain", "한글", "\n", "\n\n", "첫\n중간\n끝", "trailing\n", "a\r\nb"] {
                    let mut bulk = TextEditor::new();
                    bulk.load_content(document);
                    bulk.cursor_line = line;
                    bulk.cursor_col = column;
                    let mut reference = TextEditor::new();
                    reference.load_content(document);
                    reference.cursor_line = line;
                    reference.cursor_col = column;
                    for ch in text.chars() {
                        if ch == '\n' {
                            reference.new_line();
                        } else {
                            reference.insert_char(ch);
                        }
                    }
                    bulk.insert_text(text);
                    assert_eq!(bulk.content, reference.content, "{document:?}, {line}:{column}, {text:?}");
                    assert_eq!((bulk.cursor_line, bulk.cursor_col), (reference.cursor_line, reference.cursor_col));
                    assert_eq!(bulk.is_modified, reference.is_modified);
                    assert_eq!(bulk.char_len(), bulk.get_content().chars().count());
                }
            }
        }
    }
}

#[test]
fn bulk_insertion_remains_one_undo_step_and_handles_an_empty_buffer() {
    let mut editor = TextEditor::new();
    editor.load_content("before after");
    editor.cursor_col = 7;
    editor.insert_text("한\n글\n");
    editor.save_snapshot();
    let pasted = editor.get_content();
    editor.undo();
    assert_eq!(editor.get_content(), "before after");
    editor.redo();
    assert_eq!(editor.get_content(), pasted);
    editor.content.clear();
    assert_eq!(editor.char_len(), 0);
    editor.insert_text("한\n글");
    assert_eq!(editor.get_content(), "한\n글");
    assert_eq!(editor.char_len(), 3);
}

#[test]
fn searching_preserves_overlaps_unicode_columns_and_order() {
    let document = "aaaa\n가한한한나\ne\u{301}e\u{301}\n한글 한글\n";
    for pattern in ["a", "aa", "aaa", "한", "한한", "e\u{301}", "한글", "없음"] {
        let mut editor = TextEditor::new();
        editor.load_content(document);
        editor.search_pattern = pattern.into();
        editor.execute_search();
        let expected: Vec<_> = editor
            .content
            .iter()
            .enumerate()
            .flat_map(|(line, text)| {
                text.char_indices()
                    .enumerate()
                    .filter_map(move |(column, (byte, _))| text[byte ..].starts_with(pattern).then_some((line, column)))
            })
            .collect();
        assert_eq!(editor.search_matches, expected, "{pattern:?}");
    }
}
