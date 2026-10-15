use chrono::Local;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::{Path, PathBuf}, ptr::null_mut, sync::Mutex};
use tauri::State;
use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN}};

const API_BASE: &str = "https://codex.ximuai.com/api/v1";
const CODEX_BASE_URL: &str = "https://codex.ximuai.com";
const CODEX_MODEL: &str = "gpt-5.5";
const KEY_NAME: &str = "Codex Quick Connect";

#[derive(Default)] struct AppState { session: Mutex<Option<Session>> }
#[derive(Clone, Deserialize, Serialize)] struct Session { access_token: String }
#[derive(Clone, Deserialize, Serialize)] struct Account { email: String, balance: f64 }
#[derive(Clone, Deserialize, Serialize)] struct SavedCredentials { email: String, password: String }
#[derive(Deserialize)] struct LoginResponse { access_token: String }
#[derive(Deserialize)] struct ApiKey { id: i64, key: String }
#[derive(Clone, Deserialize, Serialize)] struct AvailableGroup { id: i64, name: String, platform: String }
#[derive(Serialize)] struct CreateKeyRequest<'a> { name: &'a str, group_id: i64 }
#[derive(Serialize)] struct UpdateKeyGroupRequest { group_id: i64 }
#[derive(Deserialize, Serialize)] struct ProvisionedKey { id: i64, secret: String, #[serde(default)] group_id: Option<i64> }
#[derive(Serialize)] struct EnableResult { balance: f64, key_created: bool, backup_path: String }
#[derive(Serialize)] struct RestoreResult { backup_path: String, rollback_path: String }

fn err(message: impl Into<String>) -> String { message.into() }

async fn request_json<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, String> {
    let response = request.send().await.map_err(|_| err("无法连接到中转站，请检查网络后重试。"))?;
    let status = response.status();
    let body = response.text().await.map_err(|_| err("无法读取中转站响应。"))?;
    let value: Value = serde_json::from_str(&body).map_err(|_| err("中转站返回了无法识别的数据。"))?;

    let message = || value.get("message").and_then(Value::as_str).filter(|text| !text.trim().is_empty()).map(str::to_owned).unwrap_or_else(|| err("中转站请求失败。"));
    if !status.is_success() { return Err(message()); }

    // The relay's current API returns successful objects directly. Older deployments
    // wrap them as { code, message, data }, so accept both response shapes.
    if let Some(code) = value.get("code").and_then(Value::as_i64) {
        if code != 0 { return Err(message()); }
        if let Some(data) = value.get("data") {
            return serde_json::from_value(data.clone()).map_err(|_| err("中转站返回了无法识别的数据。"));
        }
    }
    serde_json::from_value(value).map_err(|_| err("中转站返回了无法识别的数据。"))
}

async fn get_account(client: &Client, token: &str) -> Result<Account, String> { request_json(client.get(format!("{API_BASE}/auth/me")).bearer_auth(token)).await }
fn data_dir() -> Result<PathBuf, String> { Ok(dirs::data_local_dir().ok_or_else(|| err("无法定位 Windows 应用数据目录。"))?.join("Codex Quick Connect")) }
fn record_path() -> Result<PathBuf, String> { Ok(data_dir()?.join("provisioning.bin")) }
fn session_path() -> Result<PathBuf, String> { Ok(data_dir()?.join("session.bin")) }
fn credentials_path() -> Result<PathBuf, String> { Ok(data_dir()?.join("credentials.bin")) }
fn backups_dir() -> Result<PathBuf, String> { Ok(data_dir()?.join("backups")) }

unsafe fn dpapi(input: &[u8], protect: bool) -> Result<Vec<u8>, String> {
    let mut source = CRYPT_INTEGER_BLOB { cbData: input.len() as u32, pbData: input.as_ptr() as *mut u8 };
    let mut target: CRYPT_INTEGER_BLOB = std::mem::zeroed();
    let success = if protect { CryptProtectData(&mut source, null_mut(), null_mut(), null_mut(), null_mut(), CRYPTPROTECT_UI_FORBIDDEN, &mut target) } else { CryptUnprotectData(&mut source, null_mut(), null_mut(), null_mut(), null_mut(), CRYPTPROTECT_UI_FORBIDDEN, &mut target) };
    if success == 0 { return Err(err("Windows 凭据保护服务不可用。")); }
    let output = std::slice::from_raw_parts(target.pbData, target.cbData as usize).to_vec();
    LocalFree(target.pbData.cast());
    Ok(output)
}

fn load_record() -> Result<Option<ProvisionedKey>, String> {
    let path = record_path()?;
    if !path.exists() { return Ok(None); }
    let encrypted = fs::read(path).map_err(|_| err("无法读取本机密钥记录。"))?;
    let plain = unsafe { dpapi(&encrypted, false)? };
    serde_json::from_slice(&plain).map(Some).map_err(|_| err("本机密钥记录已损坏。"))
}

fn save_record(record: &ProvisionedKey) -> Result<(), String> {
    let path = record_path()?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|_| err("无法创建应用数据目录。"))?;
    let encoded = serde_json::to_vec(record).map_err(|_| err("无法保存密钥记录。"))?;
    let encrypted = unsafe { dpapi(&encoded, true)? };
    fs::write(path, encrypted).map_err(|_| err("无法保存加密密钥记录。"))
}

fn load_session() -> Result<Option<Session>, String> {
    let path = session_path()?;
    if !path.exists() { return Ok(None); }
    let encrypted = fs::read(path).map_err(|_| err("无法读取本机登录会话。"))?;
    let plain = unsafe { dpapi(&encrypted, false)? };
    serde_json::from_slice(&plain).map(Some).map_err(|_| err("本机登录会话已损坏。"))
}

fn save_session(session: &Session) -> Result<(), String> {
    let path = session_path()?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|_| err("无法创建应用数据目录。"))?;
    let encoded = serde_json::to_vec(session).map_err(|_| err("无法保存登录会话。"))?;
    let encrypted = unsafe { dpapi(&encoded, true)? };
    fs::write(path, encrypted).map_err(|_| err("无法保存登录会话。"))
}

fn clear_session() -> Result<(), String> {
    let path = session_path()?;
    if path.exists() { fs::remove_file(path).map_err(|_| err("无法清除登录会话。"))?; }
    Ok(())
}

fn load_saved_credentials() -> Result<Option<SavedCredentials>, String> {
    let path = credentials_path()?;
    if !path.exists() { return Ok(None); }
    let encrypted = fs::read(path).map_err(|_| err("无法读取保存的登录信息。"))?;
    let plain = unsafe { dpapi(&encrypted, false)? };
    serde_json::from_slice(&plain).map(Some).map_err(|_| err("保存的登录信息已损坏。"))
}

fn save_credentials(credentials: &SavedCredentials) -> Result<(), String> {
    let path = credentials_path()?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|_| err("无法创建应用数据目录。"))?;
    let encoded = serde_json::to_vec(credentials).map_err(|_| err("无法保存登录信息。"))?;
    let encrypted = unsafe { dpapi(&encoded, true)? };
    fs::write(path, encrypted).map_err(|_| err("无法保存登录信息。"))
}

fn clear_saved_credentials() -> Result<(), String> {
    let path = credentials_path()?;
    if path.exists() { fs::remove_file(path).map_err(|_| err("无法清除保存的登录信息。"))?; }
    Ok(())
}

fn codex_paths() -> Result<(PathBuf, PathBuf), String> { let dir = dirs::home_dir().ok_or_else(|| err("无法定位用户主目录。"))?.join(".codex"); Ok((dir.join("auth.json"), dir.join("config.toml"))) }
fn snapshot(path: &Path) -> Result<Option<Vec<u8>>, String> { if path.exists() { fs::read(path).map(Some).map_err(|_| err("无法备份现有 Codex 配置。")) } else { Ok(None) } }
fn restore(path: &Path, old: &Option<Vec<u8>>) -> Result<(), String> { match old { Some(bytes) => fs::write(path, bytes).map_err(|_| err("无法恢复原 Codex 配置。")), None => { if path.exists() { fs::remove_file(path).map_err(|_| err("无法清理未完成的 Codex 配置。"))?; } Ok(()) } } }
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> { let tmp = path.with_extension("quick-connect.tmp"); fs::write(&tmp, bytes).map_err(|_| err("无法写入 Codex 临时配置。"))?; if path.exists() { fs::remove_file(path).map_err(|_| err("无法替换当前 Codex 配置。"))?; } fs::rename(tmp, path).map_err(|_| err("无法完成 Codex 配置写入。")) }

fn is_timestamped_backup_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 15 && bytes[8] == b'-' && bytes.iter().enumerate().all(|(index, byte)| index == 8 || byte.is_ascii_digit())
}

fn find_original_codex_backup() -> Result<PathBuf, String> {
    let root = backups_dir()?;
    if !root.exists() { return Err(err("没有找到 Codex 配置备份，无法恢复。")); }
    let mut candidates: Vec<(String, PathBuf)> = Vec::new();
    for entry in fs::read_dir(&root).map_err(|_| err("无法读取 Codex 备份目录。"))? {
        let entry = entry.map_err(|_| err("无法读取 Codex 备份目录。"))?;
        let path = entry.path();
        if !path.is_dir() { continue; }
        let name = entry.file_name().to_string_lossy().to_string();
        if is_timestamped_backup_name(&name) && path.join("auth.json").is_file() && path.join("config.toml").is_file() {
            candidates.push((name, path));
        }
    }
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    candidates.into_iter().next().map(|(_, path)| path).ok_or_else(|| err("没有找到同时包含 auth.json 和 config.toml 的原 Codex 配置备份。"))
}

fn backup_current_codex(prefix: &str, old_auth: &Option<Vec<u8>>, old_config: &Option<Vec<u8>>) -> Result<PathBuf, String> {
    let backup = backups_dir()?.join(format!("{}-{}", prefix, Local::now().format("%Y%m%d-%H%M%S")));
    fs::create_dir_all(&backup).map_err(|_| err("无法创建 Codex 备份目录。"))?;
    if let Some(bytes) = old_auth { fs::write(backup.join("auth.json"), bytes).map_err(|_| err("无法保存 Codex 备份。"))?; }
    if let Some(bytes) = old_config { fs::write(backup.join("config.toml"), bytes).map_err(|_| err("无法保存 Codex 备份。"))?; }
    Ok(backup)
}

fn write_codex(key: &str) -> Result<String, String> {
    let (auth_path, config_path) = codex_paths()?;
    fs::create_dir_all(auth_path.parent().unwrap()).map_err(|_| err("无法创建 Codex 配置目录。"))?;
    let old_auth = snapshot(&auth_path)?; let old_config = snapshot(&config_path)?;
    let backup = backups_dir()?.join(Local::now().format("%Y%m%d-%H%M%S").to_string());
    fs::create_dir_all(&backup).map_err(|_| err("无法创建 Codex 备份目录。"))?;
    if let Some(bytes) = &old_auth { fs::write(backup.join("auth.json"), bytes).map_err(|_| err("无法保存 Codex 备份。"))?; }
    if let Some(bytes) = &old_config { fs::write(backup.join("config.toml"), bytes).map_err(|_| err("无法保存 Codex 备份。"))?; }
    let auth = serde_json::json!({ "OPENAI_API_KEY": key });
    let config = format!("model_provider = \"OpenAI\"\nmodel = \"{CODEX_MODEL}\"\nreview_model = \"{CODEX_MODEL}\"\nmodel_reasoning_effort = \"xhigh\"\ndisable_response_storage = true\nnetwork_access = \"enabled\"\nwindows_wsl_setup_acknowledged = true\n\n[model_providers.OpenAI]\nname = \"OpenAI\"\nbase_url = \"{CODEX_BASE_URL}\"\nwire_api = \"responses\"\nrequires_openai_auth = true\n\n[features]\ngoals = true\n");
    toml::from_str::<toml::Table>(&config).map_err(|_| err("生成的 Codex 配置无效。"))?;
    let result = write_file(&auth_path, serde_json::to_string_pretty(&auth).map_err(|_| err("无法生成认证配置。"))?.as_bytes()).and_then(|_| write_file(&config_path, config.as_bytes()));
    if let Err(error) = result { let _ = restore(&auth_path, &old_auth); let _ = restore(&config_path, &old_config); return Err(error); }
    Ok(backup.display().to_string())
}

#[tauri::command]
async fn login(state: State<'_, AppState>, email: String, password: String, remember_credentials: bool) -> Result<Account, String> {
    let credentials = SavedCredentials { email, password };
    let client = Client::new();
    let login: LoginResponse = request_json(client.post(format!("{API_BASE}/auth/login")).json(&serde_json::json!({ "email": &credentials.email, "password": &credentials.password }))).await?;
    let account = get_account(&client, &login.access_token).await?;
    let session = Session { access_token: login.access_token };
    save_session(&session)?;
    if remember_credentials { save_credentials(&credentials)?; } else { clear_saved_credentials()?; }
    *state.session.lock().map_err(|_| err("应用会话不可用。"))? = Some(session);
    Ok(account)
}

#[tauri::command]
fn get_saved_credentials() -> Result<Option<SavedCredentials>, String> { load_saved_credentials() }

#[tauri::command]
fn forget_saved_credentials() -> Result<(), String> { clear_saved_credentials() }

#[tauri::command]
fn restore_original_codex_config() -> Result<RestoreResult, String> {
    let backup = find_original_codex_backup()?;
    let auth = fs::read(backup.join("auth.json")).map_err(|_| err("无法读取原 Codex auth.json 备份。"))?;
    let config = fs::read(backup.join("config.toml")).map_err(|_| err("无法读取原 Codex config.toml 备份。"))?;
    let (auth_path, config_path) = codex_paths()?;
    fs::create_dir_all(auth_path.parent().unwrap()).map_err(|_| err("无法创建 Codex 配置目录。"))?;
    let old_auth = snapshot(&auth_path)?;
    let old_config = snapshot(&config_path)?;
    let rollback = backup_current_codex("restore-rollback", &old_auth, &old_config)?;
    let result = write_file(&auth_path, &auth).and_then(|_| write_file(&config_path, &config));
    if let Err(error) = result {
        let _ = restore(&auth_path, &old_auth);
        let _ = restore(&config_path, &old_config);
        return Err(error);
    }
    Ok(RestoreResult { backup_path: backup.display().to_string(), rollback_path: rollback.display().to_string() })
}

#[tauri::command]
async fn restore_session(state: State<'_, AppState>) -> Result<Option<Account>, String> {
    let session = match load_session() {
        Ok(Some(session)) => session,
        Ok(None) => return Ok(None),
        Err(_) => { let _ = clear_session(); return Ok(None); }
    };
    let client = Client::new();
    match get_account(&client, &session.access_token).await {
        Ok(account) => {
            *state.session.lock().map_err(|_| err("应用会话不可用。"))? = Some(session);
            Ok(Some(account))
        }
        Err(_) => { clear_session()?; Ok(None) }
    }
}

#[tauri::command]
async fn list_available_groups(state: State<'_, AppState>) -> Result<Vec<AvailableGroup>, String> {
    let session = state.session.lock().map_err(|_| err("应用会话不可用。"))?.clone().ok_or_else(|| err("请先登录。"))?;
    let client = Client::new();
    request_json(client.get(format!("{API_BASE}/groups/available")).bearer_auth(&session.access_token)).await
}

#[tauri::command]
async fn enable_codex(state: State<'_, AppState>, group_id: i64) -> Result<EnableResult, String> {
    if group_id <= 0 { return Err(err("请选择有效的密钥分组。")); }
    let session = state.session.lock().map_err(|_| err("应用会话不可用。"))?.clone().ok_or_else(|| err("请先登录。"))?;
    let client = Client::new(); let account = get_account(&client, &session.access_token).await?;
    let (record, key_created) = match load_record()? {
        Some(mut record) => {
            let _: Value = request_json(client.put(format!("{API_BASE}/keys/{}", record.id)).bearer_auth(&session.access_token).json(&UpdateKeyGroupRequest { group_id })).await?;
            record.group_id = Some(group_id);
            (record, false)
        }
        None => {
            let api_key: ApiKey = request_json(client.post(format!("{API_BASE}/keys")).bearer_auth(&session.access_token).json(&CreateKeyRequest { name: KEY_NAME, group_id })).await?;
            (ProvisionedKey { id: api_key.id, secret: api_key.key, group_id: Some(group_id) }, true)
        }
    };
    let backup_path = write_codex(&record.secret)?;
    save_record(&record)?;
    Ok(EnableResult { balance: account.balance, key_created, backup_path })
}

#[tauri::command]
fn logout(state: State<'_, AppState>) -> Result<(), String> { clear_session()?; clear_saved_credentials()?; *state.session.lock().map_err(|_| err("应用会话不可用。"))? = None; Ok(()) }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() { tauri::Builder::default().manage(AppState::default()).invoke_handler(tauri::generate_handler![login, get_saved_credentials, forget_saved_credentials, restore_original_codex_config, restore_session, list_available_groups, enable_codex, logout]).run(tauri::generate_context!()).expect("Tauri application failed"); }
