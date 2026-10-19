use chrono::NaiveDate;
use diary_storage::{Storage,
                    settings};
use std::{fs,
          io,
          path::Path};
use tempfile::TempDir;

#[test]
fn saving_a_hard_linked_diary_does_not_modify_the_other_file() {
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let outside = directory.path().join("unrelated.txt");
    std::fs::write(&outside, "must stay unchanged").unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    std::fs::hard_link(&outside, directory.path().join(format!("entries/{date}.org"))).unwrap();
    storage.save(date, "* diary").unwrap();
    assert_eq!(std::fs::read_to_string(outside).unwrap(), "must stay unchanged");
    assert_eq!(storage.load(date).unwrap(), "* diary");
}

#[test]
fn saving_hard_linked_settings_does_not_modify_the_other_file() {
    let directory = TempDir::new().unwrap();
    let outside = directory.path().join("unrelated.txt");
    let config = directory.path().join("settings.conf");
    std::fs::write(&outside, "must stay unchanged").unwrap();
    std::fs::hard_link(&outside, &config).unwrap();
    settings::save(Default::default(), &config).unwrap();
    assert_eq!(std::fs::read_to_string(outside).unwrap(), "must stay unchanged");
}

#[test]
fn oversized_documents_are_rejected_without_overwriting_saved_content() {
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    let path = directory.path().join(format!("entries/{date}.org"));
    storage.save(date, "* Keep this").unwrap();
    let oversized = "a".repeat(diary_storage::MAX_DIARY_BYTES + 1);
    assert_eq!(storage.save(date, &oversized).unwrap_err().kind(), io::ErrorKind::InvalidInput);
    assert_eq!(storage.load(date).unwrap(), "* Keep this");
    fs::File::options().write(true).open(path).unwrap().set_len(oversized.len() as u64).unwrap();
    assert_eq!(storage.load(date).unwrap_err().kind(), io::ErrorKind::InvalidData);
}

#[test]
fn oversized_settings_are_rejected_before_parsing() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("settings.conf");
    fs::write(&path, format!("#{}", "a".repeat(settings::MAX_SETTINGS_BYTES))).unwrap();
    assert_eq!(settings::load(&path).unwrap_err().kind(), io::ErrorKind::InvalidData);
}

#[test]
fn a_failed_save_preserves_the_read_only_original_and_leaves_no_temporary_files() {
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    let path = directory.path().join(format!("entries/{date}.org"));
    storage.save(date, "* Original").unwrap();
    let original = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = storage.save(date, "* Replacement");
    fs::set_permissions(&path, original).unwrap();
    assert!(result.is_err());
    assert_eq!(storage.load(date).unwrap(), "* Original");
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[cfg(any(unix, windows))]
fn symlink(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let result = if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    };
    #[cfg(windows)]
    if result.as_ref().is_err_and(|error| error.raw_os_error() == Some(1314)) {
        eprintln!("Symlink test skipped: Windows requires Developer Mode or symlink privilege");
        return false;
    }
    result.unwrap();
    true
}

#[test]
#[cfg(any(unix, windows))]
fn linked_and_dangling_diaries_are_rejected_without_falling_back_to_markdown() {
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    let outside = directory.path().join("outside.txt");
    let org = directory.path().join(format!("entries/{date}.org"));
    fs::write(&outside, "private content").unwrap();
    fs::write(org.with_extension("md"), "# legacy").unwrap();
    if !symlink(&outside, &org) {
        return;
    }
    assert!(storage.load(date).is_err());
    assert!(storage.save(date, "* bad").is_err());
    assert!(storage.delete(date).is_err());
    assert!(!storage.scan_entries().unwrap().is_empty()); // The regular .md is still indexed.
    assert_eq!(fs::read_to_string(&outside).unwrap(), "private content");
    fs::remove_file(&outside).unwrap();
    assert!(storage.load(date).is_err());
    assert!(storage.save(date, "* dangling").is_err());
    assert!(!outside.exists());
    assert_eq!(fs::read_to_string(org.with_extension("md")).unwrap(), "# legacy");
}

#[test]
#[cfg(any(unix, windows))]
fn linked_settings_cannot_be_loaded_or_saved() {
    let directory = TempDir::new().unwrap();
    let outside = directory.path().join("outside.conf");
    let link = directory.path().join("settings.conf");
    fs::write(&outside, "language=en\n").unwrap();
    if !symlink(&outside, &link) {
        return;
    }
    assert!(settings::load(&link).is_err());
    assert!(settings::save(Default::default(), &link).is_err());
    assert_eq!(fs::read_to_string(outside).unwrap(), "language=en\n");
}

#[test]
#[cfg(any(unix, windows))]
fn linked_storage_directories_are_rejected_even_after_initialization() {
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let entries = directory.path().join("entries");
    fs::remove_dir(&entries).unwrap();
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).unwrap();
    if !symlink(&outside, &entries) {
        return;
    }
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    assert!(Storage::with_dir(directory.path()).is_err());
    assert!(storage.save(date, "* Must not escape").is_err());
    assert!(storage.load(date).is_err());
    assert!(storage.scan_entries().is_err());
    assert!(!outside.join(format!("{date}.org")).exists());
    let config = outside.join("settings.conf");
    fs::write(&config, "language=en\n").unwrap();
    assert!(settings::load(&entries.join("settings.conf")).is_err());
    assert!(settings::save(Default::default(), &entries.join("settings.conf")).is_err());
    assert_eq!(fs::read_to_string(config).unwrap(), "language=en\n");
}

#[test]
#[cfg(windows)]
fn failed_atomic_replacement_keeps_the_original_and_cleans_up_the_temporary_file() {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ,
                                                  FILE_SHARE_WRITE};
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    let path = directory.path().join(format!("entries/{date}.org"));
    storage.save(date, "* Original").unwrap();
    // Permit writes but deny deletion, so replacement fails after staging.
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(&path)
        .unwrap();
    let result = storage.save(date, "* Replacement");
    drop(held);
    assert!(result.is_err());
    assert_eq!(storage.load(date).unwrap(), "* Original");
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
#[cfg(unix)]
fn new_files_and_directories_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let directory = TempDir::new().unwrap();
    let base = directory.path().join("private");
    let storage = Storage::with_dir(&base).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    storage.save(date, "* Secret").unwrap();
    assert_eq!(fs::metadata(base.join("entries")).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(
        fs::metadata(base.join(format!("entries/{date}.org"))).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let config = directory.path().join("config/settings.conf");
    settings::save(Default::default(), &config).unwrap();
    assert_eq!(fs::metadata(config).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn named_pipes_are_rejected_without_blocking() {
    use rustix::fs::{CWD,
                     Mode,
                     mkfifoat};
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    let path = directory.path().join(format!("entries/{date}.org"));
    mkfifoat(CWD, &path, Mode::RUSR | Mode::WUSR).unwrap();
    assert!(storage.load(date).is_err());
    assert!(storage.save(date, "* Not a pipe").is_err());
    assert!(storage.scan_entries().unwrap().is_empty());
}
