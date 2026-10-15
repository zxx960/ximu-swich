use serde::Deserialize;
use std::{os::windows::process::CommandExt, process::Command};
use windows::{
    core::{HSTRING, PCWSTR},
    Win32::{
        System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED},
        UI::Shell::{ApplicationActivationManager, IApplicationActivationManager, AO_NOERRORUI},
    },
};

#[derive(Deserialize)]
struct InstalledApp {
    #[serde(rename = "Name")] name: String,
    #[serde(rename = "AppID")] app_id: String,
}

fn select_client(apps: Vec<InstalledApp>) -> Result<String, String> {
    let mut candidates: Vec<_> = apps.into_iter().filter(|app| {
        let id = app.app_id.to_ascii_lowercase();
        (app.name.eq_ignore_ascii_case("ChatGPT") || app.name.eq_ignore_ascii_case("Codex"))
            && (id.starts_with("openai.codex_") || id.starts_with("openai.chatgpt"))
            && id.contains('!')
    }).collect();
    candidates.sort_by_key(|app| !app.name.eq_ignore_ascii_case("ChatGPT"));
    candidates.into_iter().next().map(|app| app.app_id)
        .ok_or_else(|| "未找到已安装的 ChatGPT 客户端，请先安装客户端后重试。".to_owned())
}

pub fn find_client() -> Result<String, String> {
    let powershell = std::env::var_os("SystemRoot").map(std::path::PathBuf::from)
        .ok_or_else(|| "无法定位 Windows 系统目录。".to_owned())?
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    // Fixed script, no user input. Keep the helper console hidden and return only app registrations.
    let output = Command::new(powershell)
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            "$ErrorActionPreference = 'Stop'; [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); ConvertTo-Json -Compress -InputObject @(Get-StartApps | Where-Object { $_.Name -in @('ChatGPT', 'Codex') } | Select-Object Name, AppID)"])
        .creation_flags(0x08000000)
        .output().map_err(|_| "无法检查 ChatGPT 客户端安装状态。".to_owned())?;
    if !output.status.success() { return Err("无法读取 Windows 应用列表，请检查 ChatGPT 客户端是否安装完整。".to_owned()); }
    let json = String::from_utf8(output.stdout).map_err(|_| "无法读取 ChatGPT 客户端信息。".to_owned())?;
    let apps = serde_json::from_str(json.trim_start_matches('\u{feff}'))
        .map_err(|_| "无法识别 ChatGPT 客户端信息。".to_owned())?;
    select_client(apps)
}

pub fn stop_client(app_id: &str) -> Result<(), String> {
    let system_root = std::env::var_os("SystemRoot").map(std::path::PathBuf::from)
        .ok_or_else(|| "无法定位 Windows 系统目录。".to_owned())?;
    // Resolve the exact package before stopping its GUI process tree. Never target unrelated CLI/IDE processes.
    let script = r#"
$ErrorActionPreference = 'Stop'
$family = $env:QUICK_CONNECT_TARGET_APP.Split('!')[0]
$package = Get-AppxPackage | Where-Object { $_.PackageFamilyName -eq $family } | Select-Object -First 1
if (!$package -or !$package.InstallLocation) { throw 'Client package not found' }
$root = $package.InstallLocation.TrimEnd('\') + '\'
$clients = @(Get-Process | Where-Object {
    $_.ProcessName -in @('ChatGPT', 'Codex') -and $_.Path -and
    $_.Path.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)
})
foreach ($clientProcess in $clients) {
    if (Get-Process -Id $clientProcess.Id -ErrorAction SilentlyContinue) {
        & "$env:SystemRoot\System32\taskkill.exe" /PID $clientProcess.Id /T /F | Out-Null
        if ($LASTEXITCODE -ne 0 -and (Get-Process -Id $clientProcess.Id -ErrorAction SilentlyContinue)) {
            throw 'Unable to close client'
        }
        if (!$clientProcess.WaitForExit(10000)) { throw 'Timed out waiting for client to exit' }
    }
}
"#;
    let output = Command::new(system_root.join("System32/WindowsPowerShell/v1.0/powershell.exe"))
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script])
        .env("QUICK_CONNECT_TARGET_APP", app_id)
        .creation_flags(0x08000000)
        .output().map_err(|_| "无法关闭 ChatGPT 客户端，请完全退出客户端后重试。".to_owned())?;
    if !output.status.success() { return Err("无法关闭 ChatGPT 客户端，请完全退出客户端后重试。".to_owned()); }
    Ok(())
}

pub fn launch_client(app_id: String) -> Result<u32, String> {
    // Use a dedicated COM apartment so activation is independent of the async worker's COM state.
    std::thread::spawn(move || unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()
            .map_err(|_| "无法初始化 Windows 应用启动服务。".to_owned())?;
        let result = (|| {
            let manager: IApplicationActivationManager = CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER)
                .map_err(|error| format!("无法连接 Windows 应用启动服务（{}）。", error.code()))?;
            manager.ActivateApplication(&HSTRING::from(app_id), PCWSTR::null(), AO_NOERRORUI)
                .map_err(|error| format!("无法启动 ChatGPT 客户端（{}），请尝试从开始菜单手动打开。", error.code()))
        })();
        CoUninitialize();
        result
    }).join().map_err(|_| "ChatGPT 客户端启动任务异常。".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_the_registered_openai_app() {
        let apps = serde_json::from_value(serde_json::json!([
            { "Name": "ChatGPT", "AppID": "other.vendor!App" },
            { "Name": "Codex", "AppID": "OpenAI.Codex_example!App" },
            { "Name": "ChatGPT", "AppID": "OpenAI.ChatGPT_example!App" }
        ])).unwrap();
        assert_eq!(select_client(apps).unwrap(), "OpenAI.ChatGPT_example!App");
    }

    #[test]
    fn reports_when_the_client_is_missing() {
        assert!(select_client(Vec::new()).is_err());
        let apps = vec![InstalledApp { name: "ChatGPT".to_owned(), app_id: "unrelated!App".to_owned() }];
        assert!(select_client(apps).is_err());
    }

    #[test]
    #[ignore = "Launches the locally installed ChatGPT client without changing any configuration"]
    fn launches_installed_client() {
        let app_id = find_client().unwrap();
        let pid = launch_client(app_id).unwrap();
        assert!(pid > 0);
        println!("ChatGPT activation succeeded, process ID: {pid}");
    }
}
