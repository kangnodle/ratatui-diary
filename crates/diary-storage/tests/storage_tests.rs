use chrono::NaiveDate;
use diary_storage::Storage;
use tempfile::TempDir;

#[test]
fn test_new_creates_entries_directory() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let _storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };

    let entries_dir = temp.path().join("entries");
    assert!(entries_dir.exists());
    assert!(entries_dir.is_dir());
}

#[test]
fn test_save_creates_org_file() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };

    let date = match NaiveDate::from_ymd_opt(2026, 2, 14) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let content = "* 오늘의 일기\n** 배운 것\nOrg-mode로 작성한 본문";

    match storage.save(date, content) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };

    let file_path = temp.path().join("entries/2026-02-14.org");
    assert!(file_path.exists());

    let saved = match std::fs::read_to_string(file_path) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not read the text file: {error:?}"),
    };
    assert_eq!(saved, content);
    assert!(
        !(match storage.is_markdown(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not identify the diary entry format: {error:?}"),
        })
    );
    assert!(!temp.path().join("entries/2026-02-14.md").exists());
}

#[test]
fn legacy_markdown_keeps_its_file_format_when_loaded_saved_and_deleted() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 2, 14) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let path = temp.path().join("entries/2026-02-14.md");
    match std::fs::write(&path, "# Original diary") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };

    assert!(match storage.is_markdown(date) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not identify the diary entry format: {error:?}"),
    });
    assert_eq!(
        match storage.load(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load the diary: {error:?}"),
        },
        "# Original diary"
    );
    assert!(
        (match storage.scan_entries() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the diary entries: {error:?}"),
        })
        .contains(&date)
    );
    match storage.save(date, "# Updated diary") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };
    assert_eq!(
        match std::fs::read_to_string(&path) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "# Updated diary"
    );
    assert!(!path.with_extension("org").exists());
    match storage.delete(date) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not delete the diary: {error:?}"),
    };
    assert!(!path.exists());
    assert!(
        (match storage.scan_entries() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the diary entries: {error:?}"),
        })
        .is_empty()
    );
}

#[test]
fn org_takes_precedence_without_overwriting_legacy_markdown() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 2, 14) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let org = temp.path().join("entries/2026-02-14.org");
    let markdown = org.with_extension("md");
    match std::fs::write(&org, "* Org diary") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    match std::fs::write(&markdown, "# Legacy diary") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };

    assert!(
        !(match storage.is_markdown(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not identify the diary entry format: {error:?}"),
        })
    );
    assert_eq!(
        match storage.load(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load the diary: {error:?}"),
        },
        "* Org diary"
    );
    match storage.save(date, "* Updated Org diary") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };
    assert_eq!(
        match std::fs::read_to_string(&org) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "* Updated Org diary"
    );
    assert_eq!(
        match std::fs::read_to_string(&markdown) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "# Legacy diary"
    );
    assert_eq!(
        (match storage.scan_entries() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the diary entries: {error:?}"),
        })
        .len(),
        1
    );
    match storage.delete(date) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not delete the diary: {error:?}"),
    };
    assert!(!org.exists());
    assert!(!markdown.exists());
    assert!(
        (match storage.scan_entries() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the diary entries: {error:?}"),
        })
        .is_empty()
    );
}

#[test]
fn scanning_accepts_org_and_markdown_and_ignores_other_files_and_directories() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = temp.path().join("entries");
    for name in [
        "2026-02-14.org",
        "2026-02-14.md",
        "2026-02-15.md",
        "2026-02-30.org",
        "notes.org",
        "2026-02-16.txt",
        "2026-02-17.org.bak",
    ] {
        match std::fs::write(entries.join(name), "test") {
            | Ok(_) => (),
            | Err(error) => panic!("Could not write the file: {error:?}"),
        };
    }
    match std::fs::create_dir(entries.join("2026-02-18.org")) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the directory: {error:?}"),
    };
    let dates = match storage.scan_entries() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not list the diary entries: {error:?}"),
    };
    assert_eq!(dates.len(), 2);
    assert!(dates.contains(
        &(match NaiveDate::from_ymd_opt(2026, 2, 14) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        })
    ));
    assert!(dates.contains(
        &(match NaiveDate::from_ymd_opt(2026, 2, 15) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        })
    ));
}

#[test]
fn org_io_errors_do_not_fall_back_to_or_overwrite_markdown() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 2, 14) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let org = temp.path().join("entries/2026-02-14.org");
    let markdown = org.with_extension("md");
    match std::fs::write(&markdown, "# Legacy diary") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    match std::fs::write(&org, [0xff]) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    assert_eq!(storage.load(date).unwrap_err().kind(), std::io::ErrorKind::InvalidData);

    match std::fs::remove_file(&org) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the file: {error:?}"),
    };
    match std::fs::create_dir(&org) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the directory: {error:?}"),
    };
    assert!(storage.load(date).is_err());
    assert!(storage.save(date, "* Must not overwrite legacy").is_err());
    assert!(storage.delete(date).is_err());
    assert_eq!(
        match std::fs::read_to_string(markdown) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "# Legacy diary"
    );
}

#[test]
fn test_load_existing_diary() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };

    let date = match NaiveDate::from_ymd_opt(2026, 2, 14) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let content = "Existing diary";
    match storage.save(date, content) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };

    let loaded = match storage.load(date) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not load the diary: {error:?}"),
    };
    assert_eq!(loaded, content);
}

#[test]
fn test_load_nonexistent_diary() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };

    let date = match NaiveDate::from_ymd_opt(2026, 2, 14) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let result = storage.load(date);

    assert!(result.is_err());
}

#[test]
fn test_delete_diary() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };

    let date = match NaiveDate::from_ymd_opt(2026, 2, 14) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    match storage.save(date, "test") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };

    match storage.delete(date) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not delete the diary: {error:?}"),
    };

    let result = storage.load(date);
    assert!(result.is_err());
}

#[test]
fn test_scan_entries() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };

    let date1 = match NaiveDate::from_ymd_opt(2026, 2, 14) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let date2 = match NaiveDate::from_ymd_opt(2026, 2, 15) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    match storage.save(date1, "test1") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };
    match storage.save(date2, "test2") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };

    let entries = match storage.scan_entries() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not list the diary entries: {error:?}"),
    };

    assert_eq!(entries.len(), 2);
    assert!(entries.contains(&date1));
    assert!(entries.contains(&date2));
}

#[test]
fn test_new_uses_system_data_dir() {
    let result = Storage::new();

    match result {
        | Ok(storage) => {
            assert!(storage.scan_entries().is_ok());
        },
        | Err(e) => {
            assert!(e.to_string().contains("Cannot find local data directory") || e.kind() == std::io::ErrorKind::NotFound);
        },
    }
}

#[test]
fn test_new_with_none_dir() {
    let result = Storage::new_with_none_dir();

    assert!(result.is_err());
    if let Err(error) = result {
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(error.to_string(), "Cannot find local data directory");
    }
}
