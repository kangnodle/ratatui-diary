use std::{fs::{self,
               File,
               Metadata},
          io::{self,
               Read,
               Write},
          path::Path};

pub(crate) fn is_link(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(crate) fn check_directory(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link(&metadata) || !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Storage directory must be a real directory, not a link",
        ));
    }
    Ok(())
}

pub(crate) fn create_directory(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    check_directory(path)
}

pub(crate) fn regular_file(path: &Path) -> io::Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        | Ok(metadata) => metadata,
        | Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        | Err(error) => return Err(error),
    };
    if is_link(&metadata) || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Expected a regular file, not a link or special file",
        ));
    }
    Ok(true)
}

#[cfg(unix)]
fn open_read(path: &Path) -> io::Result<File> {
    use rustix::fs::{Mode,
                     OFlags,
                     open};
    Ok(File::from(open(path, OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK, Mode::empty())?))
}

#[cfg(windows)]
fn open_read(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
    fs::OpenOptions::new().read(true).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_read(_path: &Path) -> io::Result<File> { Err(io::Error::new(io::ErrorKind::Unsupported, "Secure file reads require Unix or Windows")) }

pub(crate) fn read_text(path: &Path, limit: usize) -> io::Result<String> {
    let parent = path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    check_directory(parent)?;
    regular_file(path)?;
    let file = open_read(path)?;
    let metadata = file.metadata()?;
    if is_link(&metadata) || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Expected a regular file, not a link or special file",
        ));
    }
    if metadata.len() > limit as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "File exceeds size limit"));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "File exceeds size limit"));
    }
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub(crate) fn atomic_write(path: &Path, content: &str, limit: usize) -> io::Result<()> {
    if content.len() > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Content exceeds size limit"));
    }
    let parent = path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    check_directory(parent)?;
    if regular_file(path)? && fs::symlink_metadata(path)?.permissions().readonly() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Cannot replace a read-only file"));
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(content.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}
