//! Replace configuration files without a backup or a delete-before-write gap.
use std::{fs, io::Write, path::Path};

pub fn write(path: &Path, bytes: &[u8], extension: &str) -> Result<(), String> {
    let tmp = path.with_extension(extension);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&tmp)
        .map_err(|_| "无法创建临时配置文件，请检查文件访问权限。")?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        // Close before Windows ReplaceFileW; never remove the destination first.
        drop(file);
        replace(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
        return Err("无法替换配置文件，请检查文件访问权限后重试。".to_owned());
    }
    Ok(())
}

#[cfg(windows)]
fn replace(tmp: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    if !path.exists() {
        return fs::rename(tmp, path);
    }
    let target: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let replacement: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe {
        windows_sys::Win32::Storage::FileSystem::ReplaceFileW(
            target.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn replace(tmp: &Path, path: &Path) -> std::io::Result<()> {
    fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::ResetTestDir;

    #[test]
    fn replacement_failure_keeps_original_and_cleans_own_temp_file() {
        let dir = ResetTestDir::new();
        let path = dir.0.join("settings.json");
        fs::create_dir(&path).unwrap();
        assert!(write(&path, b"test-only", "ximuai.tmp").is_err());
        assert!(path.is_dir());
        assert!(!path.with_extension("ximuai.tmp").exists());
    }

    #[test]
    fn an_existing_temp_file_is_not_overwritten_or_deleted() {
        let dir = ResetTestDir::new();
        let path = dir.0.join("settings.json");
        fs::write(&path, "old").unwrap();
        let tmp = path.with_extension("ximuai.tmp");
        fs::write(&tmp, "another writer").unwrap();
        assert!(write(&path, b"new", "ximuai.tmp").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "old");
        assert_eq!(fs::read_to_string(&tmp).unwrap(), "another writer");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mac_configuration_files_are_private_on_create_and_replace() {
        use std::os::unix::fs::PermissionsExt;
        let dir = ResetTestDir::new();
        let path = dir.0.join("auth.json");
        write(&path, b"test-only", "ximuai.tmp").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        write(&path, b"updated", "ximuai.tmp").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"updated");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
