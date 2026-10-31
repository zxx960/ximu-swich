//! Windows keeps the original DPAPI files; macOS keeps the records in Keychain.
use serde::{de::DeserializeOwned, Serialize};

pub fn save(name: &str, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|_| "无法编码本机凭据。")?;
    platform::save(name, &bytes)
}

pub fn load<T: DeserializeOwned>(name: &str) -> Result<Option<T>, String> {
    platform::load(name)?
        .map(|bytes| {
            serde_json::from_slice(&bytes)
                .map_err(|_| "保存的本机凭据已损坏，请重新登录。".to_owned())
        })
        .transpose()
}

pub fn delete(name: &str) -> Result<(), String> {
    platform::delete(name)
}

#[cfg(windows)]
mod platform {
    use std::{fs, path::PathBuf, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };

    fn path(name: &str) -> Result<PathBuf, String> {
        Ok(dirs::data_local_dir()
            .ok_or("无法定位应用数据目录。")?
            .join("Codex Quick Connect")
            .join(format!("{name}.bin")))
    }

    fn crypt(input: &[u8], protect: bool) -> Result<Vec<u8>, String> {
        unsafe {
            let mut source = CRYPT_INTEGER_BLOB {
                cbData: input.len() as u32,
                pbData: input.as_ptr() as *mut u8,
            };
            let mut target: CRYPT_INTEGER_BLOB = std::mem::zeroed();
            let success = if protect {
                CryptProtectData(
                    &mut source,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut target,
                )
            } else {
                CryptUnprotectData(
                    &mut source,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut target,
                )
            };
            if success == 0 {
                return Err("Windows 凭据保护服务不可用。".to_owned());
            }
            let output = std::slice::from_raw_parts(target.pbData, target.cbData as usize).to_vec();
            LocalFree(target.pbData.cast());
            Ok(output)
        }
    }

    pub(super) fn save(name: &str, bytes: &[u8]) -> Result<(), String> {
        let path = path(name)?;
        fs::create_dir_all(path.parent().unwrap()).map_err(|_| "无法创建应用数据目录。")?;
        fs::write(path, crypt(bytes, true)?).map_err(|_| "无法保存加密凭据。".to_owned())
    }

    pub(super) fn load(name: &str) -> Result<Option<Vec<u8>>, String> {
        match fs::read(path(name)?) {
            Ok(bytes) => crypt(&bytes, false).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err("无法读取本机凭据。".to_owned()),
        }
    }

    pub(super) fn delete(name: &str) -> Result<(), String> {
        match fs::remove_file(path(name)?) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("无法清除本机凭据。".to_owned()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn dpapi_preserves_existing_record_format() {
            let plain = br#"{"access_token":"test-only"}"#;
            let encrypted = crypt(plain, true).unwrap();
            assert_ne!(encrypted, plain);
            assert_eq!(crypt(&encrypted, false).unwrap(), plain);
            assert!(crypt(b"not encrypted", false).is_err());
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use security_framework::passwords::{
        delete_generic_password, generic_password, set_generic_password, PasswordOptions,
    };

    // Stable across app updates; no secrets in files, command-line arguments, or logs.
    const SERVICE: &str = "com.ximuai.codex-quick-connect";
    const ITEM_NOT_FOUND: i32 = -25300;

    pub(super) fn save(name: &str, bytes: &[u8]) -> Result<(), String> {
        set_generic_password(SERVICE, name, bytes).map_err(|error| {
            format!(
                "无法保存到 macOS 钥匙串（{}），请解锁钥匙串并允许访问。",
                error.code()
            )
        })
    }

    pub(super) fn load(name: &str) -> Result<Option<Vec<u8>>, String> {
        match generic_password(PasswordOptions::new_generic_password(SERVICE, name)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
            Err(error) => Err(format!(
                "无法读取 macOS 钥匙串（{}），请解锁钥匙串并允许访问。",
                error.code()
            )),
        }
    }

    pub(super) fn delete(name: &str) -> Result<(), String> {
        match delete_generic_password(SERVICE, name) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(()),
            Err(error) => Err(format!("无法清除 macOS 钥匙串凭据（{}）。", error.code())),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[ignore = "Uses the logged-in macOS user's Keychain with an isolated temporary record"]
        fn keychain_record_round_trip() {
            let name = format!(
                "test-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap()
            );
            assert!(load(&name).unwrap().is_none());
            save(&name, b"test-only").unwrap();
            let first = load(&name);
            let update = save(&name, b"updated-test-only");
            let second = load(&name);
            delete(&name).unwrap();
            assert_eq!(first.unwrap().unwrap(), b"test-only");
            update.unwrap();
            assert_eq!(second.unwrap().unwrap(), b"updated-test-only");
            assert!(load(&name).unwrap().is_none());
            delete(&name).unwrap();
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
compile_error!("西牧swich currently supports Windows and macOS only.");
