use chrono::NaiveDate;
use diary_storage::Storage;
use tempfile::TempDir;

#[test]
fn test_new_creates_entries_directory() {
    let temp = TempDir::new().unwrap();
    let _storage = Storage::with_dir(temp.path()).unwrap();

    let entries_dir = temp.path().join("entries");
    assert!(entries_dir.exists());
    assert!(entries_dir.is_dir());
}

#[test]
fn test_save_creates_org_file() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();

    let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
    let content = "* 오늘의 일기\n** 배운 것\nOrg-mode로 작성한 본문";

    storage.save(date, content).unwrap();

    let file_path = temp.path().join("entries/2026-02-14.org");
    assert!(file_path.exists());

    let saved = std::fs::read_to_string(file_path).unwrap();
    assert_eq!(saved, content);
    assert!(!storage.is_markdown(date).unwrap());
    assert!(!temp.path().join("entries/2026-02-14.md").exists());
}

#[test]
fn legacy_markdown_keeps_its_file_format_when_loaded_saved_and_deleted() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
    let path = temp.path().join("entries/2026-02-14.md");
    std::fs::write(&path, "# Original diary").unwrap();

    assert!(storage.is_markdown(date).unwrap());
    assert_eq!(storage.load(date).unwrap(), "# Original diary");
    assert!(storage.scan_entries().unwrap().contains(&date));
    storage.save(date, "# Updated diary").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# Updated diary");
    assert!(!path.with_extension("org").exists());
    storage.delete(date).unwrap();
    assert!(!path.exists());
    assert!(storage.scan_entries().unwrap().is_empty());
}

#[test]
fn org_takes_precedence_without_overwriting_legacy_markdown() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
    let org = temp.path().join("entries/2026-02-14.org");
    let markdown = org.with_extension("md");
    std::fs::write(&org, "* Org diary").unwrap();
    std::fs::write(&markdown, "# Legacy diary").unwrap();

    assert!(!storage.is_markdown(date).unwrap());
    assert_eq!(storage.load(date).unwrap(), "* Org diary");
    storage.save(date, "* Updated Org diary").unwrap();
    assert_eq!(std::fs::read_to_string(&org).unwrap(), "* Updated Org diary");
    assert_eq!(std::fs::read_to_string(&markdown).unwrap(), "# Legacy diary");
    assert_eq!(storage.scan_entries().unwrap().len(), 1);
    storage.delete(date).unwrap();
    assert!(!org.exists());
    assert!(!markdown.exists());
    assert!(storage.scan_entries().unwrap().is_empty());
}

#[test]
fn scanning_accepts_org_and_markdown_and_ignores_other_files_and_directories() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();
    let entries = temp.path().join("entries");
    for name in ["2026-02-14.org", "2026-02-14.md", "2026-02-15.md", "2026-02-30.org", "notes.org", "2026-02-16.txt", "2026-02-17.org.bak"] {
        std::fs::write(entries.join(name), "test").unwrap();
    }
    std::fs::create_dir(entries.join("2026-02-18.org")).unwrap();
    let dates = storage.scan_entries().unwrap();
    assert_eq!(dates.len(), 2);
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 2, 14).unwrap()));
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 2, 15).unwrap()));
}

#[test]
fn org_io_errors_do_not_fall_back_to_or_overwrite_markdown() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
    let org = temp.path().join("entries/2026-02-14.org");
    let markdown = org.with_extension("md");
    std::fs::write(&markdown, "# Legacy diary").unwrap();
    std::fs::write(&org, [0xff]).unwrap();
    assert_eq!(storage.load(date).unwrap_err().kind(), std::io::ErrorKind::InvalidData);

    std::fs::remove_file(&org).unwrap();
    std::fs::create_dir(&org).unwrap();
    assert!(storage.load(date).is_err());
    assert!(storage.save(date, "* Must not overwrite legacy").is_err());
    assert!(storage.delete(date).is_err());
    assert_eq!(std::fs::read_to_string(markdown).unwrap(), "# Legacy diary");
}

#[test]
fn test_load_existing_diary() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();

    let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
    let content = "Existing diary";
    storage.save(date, content).unwrap();

    let loaded = storage.load(date).unwrap();
    assert_eq!(loaded, content);
}

#[test]
fn test_load_nonexistent_diary() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();

    let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
    let result = storage.load(date);

    assert!(result.is_err());
}

#[test]
fn test_delete_diary() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();

    let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
    storage.save(date, "test").unwrap();

    storage.delete(date).unwrap();

    let result = storage.load(date);
    assert!(result.is_err());
}

#[test]
fn test_scan_entries() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();

    let date1 = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
    storage.save(date1, "test1").unwrap();
    storage.save(date2, "test2").unwrap();

    let entries = storage.scan_entries().unwrap();

    assert_eq!(entries.len(), 2);
    assert!(entries.contains(&date1));
    assert!(entries.contains(&date2));
}

#[test]
fn test_new_uses_system_data_dir() {
    // Given: 시스템 데이터 디렉토리가 존재
    // When: Storage::new() 호출
    let result = Storage::new();

    // Then: 성공적으로 생성되거나 에러 반환
    match result {
        | Ok(storage) => {
            // 생성된 storage는 유효해야 함
            assert!(storage.scan_entries().is_ok());
        },
        | Err(e) => {
            // 에러 메시지 검증
            assert!(e.to_string().contains("Cannot find local data directory") || e.kind() == std::io::ErrorKind::NotFound);
        },
    }
}

#[test]
fn test_new_with_none_dir() {
    // Given: 데이터 디렉토리를 찾을 수 없는 상황 시뮬레이션
    // When: new_with_none_dir() 호출
    let result = Storage::new_with_none_dir();

    // Then: 에러가 반환되어야 함
    assert!(result.is_err());
    if let Err(error) = result {
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(error.to_string(), "Cannot find local data directory");
    }
}
