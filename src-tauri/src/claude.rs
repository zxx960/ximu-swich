use serde_json::Value;
use std::{fs, path::{Path, PathBuf}};

const MODELS: &[&str] = &["claude-sonnet-5", "claude-opus-5-5", "claude-fable-5-1"];

pub fn settings_path() -> Result<PathBuf, String> {
    let dir = match std::env::var_os("CLAUDE_CONFIG_DIR").filter(|value| !value.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => dirs::home_dir().ok_or("无法定位用户主目录。")?.join(".claude"),
    };
    Ok(dir.join("settings.json"))
}

pub fn validate_model(model: &str) -> Result<(), String> {
    if !MODELS.contains(&model) {
        return Err("请选择有效的 Claude 模型。".to_owned());
    }
    Ok(())
}

fn merge_settings(old: Option<&[u8]>, key: &str, model: &str) -> Result<Vec<u8>, String> {
    validate_model(model)?;
    let mut settings: Value = match old {
        Some(bytes) => serde_json::from_slice(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes))
            .map_err(|_| "Claude Code 配置不是有效的 JSON，请修复 settings.json 后重试。")?,
        None => serde_json::json!({}),
    };
    let root = settings.as_object_mut().ok_or("Claude Code 配置必须是 JSON 对象。")?;
    let env = root.entry("env").or_insert_with(|| serde_json::json!({}))
        .as_object_mut().ok_or("Claude Code 配置中的 env 必须是 JSON 对象。")?;
    // Avoid mixing the relay token with credentials or provider switches from an older configuration.
    for name in ["ANTHROPIC_API_KEY", "CLAUDE_CODE_OAUTH_TOKEN", "CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_USE_VERTEX", "CLAUDE_CODE_USE_FOUNDRY"] {
        env.remove(name);
    }
    env.insert("ANTHROPIC_BASE_URL".to_owned(), Value::String(super::CODEX_BASE_URL.to_owned()));
    env.insert("ANTHROPIC_AUTH_TOKEN".to_owned(), Value::String(key.to_owned()));
    env.insert("ANTHROPIC_MODEL".to_owned(), Value::String(model.to_owned()));
    root.insert("model".to_owned(), Value::String(model.to_owned()));
    serde_json::to_vec_pretty(&settings).map_err(|_| "无法生成 Claude Code 配置。".to_owned())
}

pub fn write_settings(path: &Path, key: &str, model: &str) -> Result<(), String> {
    let old = match fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("无法读取 Claude Code 配置。".to_owned()),
    };
    let updated = merge_settings(old.as_deref(), key, model)?;
    fs::create_dir_all(path.parent().ok_or("无法定位 Claude Code 配置目录。")?)
        .map_err(|_| "无法创建 Claude Code 配置目录。")?;
    let tmp = path.with_extension("ximuai.tmp");
    fs::write(&tmp, &updated).map_err(|_| "无法写入 Claude Code 临时配置。")?;
    // Replace the Windows file while preserving its permissions, without creating a backup.
    let result = if old.is_some() {
        use std::os::windows::ffi::OsStrExt;
        let target: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let replacement: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            windows_sys::Win32::Storage::FileSystem::ReplaceFileW(
                target.as_ptr(), replacement.as_ptr(), std::ptr::null(), 0, std::ptr::null(), std::ptr::null(),
            ) != 0
        }
    } else {
        fs::rename(&tmp, path).is_ok()
    };
    if !result {
        let _ = fs::remove_file(&tmp);
        return Err("无法替换 Claude Code 配置，请检查文件访问权限后重试。".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_relay_without_changing_other_settings() {
        let old = serde_json::json!({
            "theme":"dark", "permissions":{"allow":["Read"]}, "hooks":{"test":[]},
            "env":{"KEEP_ME":"yes", "ANTHROPIC_API_KEY":"old", "CLAUDE_CODE_USE_VERTEX":"1"}
        });
        let bytes = merge_settings(Some(&serde_json::to_vec(&old).unwrap()), "test-only", "claude-sonnet-5").unwrap();
        let new: Value = serde_json::from_slice(&bytes).unwrap();
        for name in ["theme", "permissions", "hooks"] { assert_eq!(new[name], old[name]); }
        assert_eq!(new["env"]["KEEP_ME"], "yes");
        assert_eq!(new["env"]["ANTHROPIC_AUTH_TOKEN"], "test-only");
        assert_eq!(new["env"]["ANTHROPIC_BASE_URL"], super::super::CODEX_BASE_URL);
        assert_eq!(new["model"], "claude-sonnet-5");
        assert_eq!(new["env"]["ANTHROPIC_MODEL"], "claude-sonnet-5");
        assert!(new["env"].get("ANTHROPIC_API_KEY").is_none());
        assert!(new["env"].get("CLAUDE_CODE_USE_VERTEX").is_none());
    }

    #[test]
    fn rejects_invalid_existing_configuration() {
        for old in [b"broken".as_slice(), b"[]", b"null", br#"{"env":[]}"#] {
            assert!(merge_settings(Some(old), "test-only", "claude-opus-5-5").is_err());
        }
        assert!(merge_settings(None, "test-only", "").is_err());
        assert!(merge_settings(None, "test-only", "model\ninvalid").is_err());
        assert!(merge_settings(None, "test-only", "claude-sonnet-custom").is_err());
    }

    #[test]
    fn supported_models_are_written_to_both_settings() {
        for model in MODELS {
            let saved: Value = serde_json::from_slice(&merge_settings(None, "test-only", model).unwrap()).unwrap();
            assert_eq!(saved["model"], *model);
            assert_eq!(saved["env"]["ANTHROPIC_MODEL"], *model);
        }
        assert!(validate_model("sonnet").is_err());
    }

    #[test]
    fn writes_settings_without_backups_and_preserves_broken_files() {
        let dir = super::super::tests::ResetTestDir::new();
        let path = dir.0.join("claude/settings.json");
        write_settings(&path, "test-only", "claude-opus-5-5").unwrap();
        write_settings(&path, "test-new", "claude-fable-5-1").unwrap();
        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["model"], "claude-fable-5-1");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        fs::write(&path, "broken").unwrap();
        assert!(write_settings(&path, "test-only", "claude-sonnet-5").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "broken");
    }
}
