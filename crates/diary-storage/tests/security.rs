use chrono::NaiveDate;
use diary_storage::{Storage,
                    settings};
use std::{fs,
          io,
          path::Path};
use tempfile::TempDir;

#[test]
fn saving_a_hard_linked_diary_does_not_modify_the_other_file() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let outside = directory.path().join("unrelated.txt");
    match std::fs::write(&outside, "must stay unchanged") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    match std::fs::hard_link(&outside, directory.path().join(format!("entries/{date}.org"))) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the hard link: {error:?}"),
    };
    match storage.save(date, "* diary") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };
    assert_eq!(
        match std::fs::read_to_string(outside) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "must stay unchanged"
    );
    assert_eq!(
        match storage.load(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load the diary: {error:?}"),
        },
        "* diary"
    );
}

#[test]
fn saving_hard_linked_settings_does_not_modify_the_other_file() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let outside = directory.path().join("unrelated.txt");
    let config = directory.path().join("settings.conf");
    match std::fs::write(&outside, "must stay unchanged") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    match std::fs::hard_link(&outside, &config) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the hard link: {error:?}"),
    };
    match settings::save(Default::default(), &config) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save settings: {error:?}"),
    };
    assert_eq!(
        match std::fs::read_to_string(outside) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "must stay unchanged"
    );
}

#[test]
fn oversized_documents_are_rejected_without_overwriting_saved_content() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let path = directory.path().join(format!("entries/{date}.org"));
    match storage.save(date, "* Keep this") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };
    let oversized = "a".repeat(diary_storage::MAX_DIARY_BYTES + 1);
    assert_eq!(storage.save(date, &oversized).unwrap_err().kind(), io::ErrorKind::InvalidInput);
    assert_eq!(
        match storage.load(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load the diary: {error:?}"),
        },
        "* Keep this"
    );
    match (match fs::File::options().write(true).open(path) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not open the file: {error:?}"),
    })
    .set_len(oversized.len() as u64)
    {
        | Ok(_) => (),
        | Err(error) => panic!("Could not set the file length: {error:?}"),
    };
    assert_eq!(storage.load(date).unwrap_err().kind(), io::ErrorKind::InvalidData);
}

#[test]
fn oversized_settings_are_rejected_before_parsing() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let path = directory.path().join("settings.conf");
    match fs::write(&path, format!("#{}", "a".repeat(settings::MAX_SETTINGS_BYTES))) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    assert_eq!(settings::load(&path).unwrap_err().kind(), io::ErrorKind::InvalidData);
}

#[test]
fn a_failed_save_preserves_the_read_only_original_and_leaves_no_temporary_files() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let path = directory.path().join(format!("entries/{date}.org"));
    match storage.save(date, "* Original") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };
    let original = (match fs::metadata(&path) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not read file metadata: {error:?}"),
    })
    .permissions();
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    match fs::set_permissions(&path, readonly) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not set file permissions: {error:?}"),
    };
    let result = storage.save(date, "* Replacement");
    match fs::set_permissions(&path, original) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not set file permissions: {error:?}"),
    };
    assert!(result.is_err());
    assert_eq!(
        match storage.load(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load the diary: {error:?}"),
        },
        "* Original"
    );
    assert_eq!(
        (match fs::read_dir(match path.parent() {
            | Some(value) => value,
            | None => panic!("Expected a parent directory for the diary path"),
        }) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the directory: {error:?}"),
        })
        .count(),
        1
    );
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
    match result {
        | Ok(_) => (),
        | Err(error) => panic!("Search failed: {error:?}"),
    };
    true
}

#[test]
#[cfg(any(unix, windows))]
fn search_cache_rejects_linked_files_and_directories_without_touching_targets() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let outside = directory.path().join("outside.json");
    match fs::write(&outside, "private content") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    match storage.save_search_cache(date, "cache") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the embedding cache: {error:?}"),
    };
    let cache_directory = directory.path().join("entries/.search-cache");
    let path = cache_directory.join(format!("{date}.json"));
    match fs::remove_file(&path) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the file: {error:?}"),
    };
    if !symlink(&outside, &path) {
        return;
    }
    assert!(storage.load_search_cache(date).is_err());
    assert!(storage.save_search_cache(date, "replacement").is_err());
    assert!(storage.prune_search_cache(&Default::default()).is_err());
    assert_eq!(
        match fs::read_to_string(&outside) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "private content"
    );
    match fs::remove_file(path) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the file: {error:?}"),
    };
    match fs::remove_dir(&cache_directory) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the directory: {error:?}"),
    };
    let outside_directory = directory.path().join("outside");
    match fs::create_dir(&outside_directory) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the directory: {error:?}"),
    };
    assert!(symlink(&outside_directory, &cache_directory));
    assert!(storage.load_search_cache(date).is_err());
    assert!(storage.save_search_cache(date, "replacement").is_err());
    assert!(storage.prune_search_cache(&Default::default()).is_err());
    assert_eq!(
        (match fs::read_dir(outside_directory) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the directory: {error:?}"),
        })
        .count(),
        0
    );
}

#[test]
#[cfg(any(unix, windows))]
fn linked_and_dangling_diaries_are_rejected_without_falling_back_to_markdown() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let outside = directory.path().join("outside.txt");
    let org = directory.path().join(format!("entries/{date}.org"));
    match fs::write(&outside, "private content") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    match fs::write(org.with_extension("md"), "# legacy") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    if !symlink(&outside, &org) {
        return;
    }
    assert!(storage.load(date).is_err());
    assert!(storage.save(date, "* bad").is_err());
    assert!(storage.delete(date).is_err());
    assert!(
        !(match storage.scan_entries() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the diary entries: {error:?}"),
        })
        .is_empty()
    );
    assert_eq!(
        match fs::read_to_string(&outside) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "private content"
    );
    match fs::remove_file(&outside) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the file: {error:?}"),
    };
    assert!(storage.load(date).is_err());
    assert!(storage.save(date, "* dangling").is_err());
    assert!(!outside.exists());
    assert_eq!(
        match fs::read_to_string(org.with_extension("md")) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "# legacy"
    );
}

#[test]
#[cfg(any(unix, windows))]
fn linked_settings_cannot_be_loaded_or_saved() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let outside = directory.path().join("outside.conf");
    let link = directory.path().join("settings.conf");
    match fs::write(&outside, "language=en\n") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    if !symlink(&outside, &link) {
        return;
    }
    assert!(settings::load(&link).is_err());
    assert!(settings::save(Default::default(), &link).is_err());
    assert_eq!(
        match fs::read_to_string(outside) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "language=en\n"
    );
}

#[test]
#[cfg(any(unix, windows))]
fn linked_storage_directories_are_rejected_even_after_initialization() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = directory.path().join("entries");
    match fs::remove_dir(&entries) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the directory: {error:?}"),
    };
    let outside = directory.path().join("outside");
    match fs::create_dir(&outside) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the directory: {error:?}"),
    };
    if !symlink(&outside, &entries) {
        return;
    }
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    assert!(Storage::with_dir(directory.path()).is_err());
    assert!(storage.save(date, "* Must not escape").is_err());
    assert!(storage.load(date).is_err());
    assert!(storage.scan_entries().is_err());
    assert!(!outside.join(format!("{date}.org")).exists());
    let config = outside.join("settings.conf");
    match fs::write(&config, "language=en\n") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    assert!(settings::load(&entries.join("settings.conf")).is_err());
    assert!(settings::save(Default::default(), &entries.join("settings.conf")).is_err());
    assert_eq!(
        match fs::read_to_string(config) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "language=en\n"
    );
}

#[test]
#[cfg(windows)]
fn failed_atomic_replacement_keeps_the_original_and_cleans_up_the_temporary_file() {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ,
                                                  FILE_SHARE_WRITE};
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let path = directory.path().join(format!("entries/{date}.org"));
    match storage.save(date, "* Original") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };
    let held = match fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).open(&path) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not open the file: {error:?}"),
    };
    let result = storage.save(date, "* Replacement");
    drop(held);
    assert!(result.is_err());
    assert_eq!(
        match storage.load(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load the diary: {error:?}"),
        },
        "* Original"
    );
    assert_eq!(
        (match fs::read_dir(match path.parent() {
            | Some(value) => value,
            | None => panic!("Expected a parent directory for the diary path"),
        }) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the directory: {error:?}"),
        })
        .count(),
        1
    );
}

#[test]
#[cfg(unix)]
fn new_files_and_directories_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let base = directory.path().join("private");
    let storage = match Storage::with_dir(&base) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    match storage.save(date, "* Secret") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };
    assert_eq!(
        (match fs::metadata(base.join("entries")) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read file metadata: {error:?}"),
        })
        .permissions()
        .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        (match fs::metadata(base.join(format!("entries/{date}.org"))) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read file metadata: {error:?}"),
        })
        .permissions()
        .mode()
            & 0o777,
        0o600
    );
    let config = directory.path().join("config/settings.conf");
    match settings::save(Default::default(), &config) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save settings: {error:?}"),
    };
    assert_eq!(
        (match fs::metadata(config) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read file metadata: {error:?}"),
        })
        .permissions()
        .mode()
            & 0o777,
        0o600
    );
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn named_pipes_are_rejected_without_blocking() {
    use rustix::fs::{CWD,
                     Mode,
                     mkfifoat};
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let path = directory.path().join(format!("entries/{date}.org"));
    match mkfifoat(CWD, &path, Mode::RUSR | Mode::WUSR) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the named pipe: {error:?}"),
    };
    assert!(storage.load(date).is_err());
    assert!(storage.save(date, "* Not a pipe").is_err());
    assert!(
        (match storage.scan_entries() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the diary entries: {error:?}"),
        })
        .is_empty()
    );
}
