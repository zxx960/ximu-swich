use objc2::rc::{autoreleasepool, Retained};
use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_foundation::NSString;
use std::{
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

// The Codex-based desktop client reads ~/.codex, including when named ChatGPT.
// The classic ChatGPT app (com.openai.chat) uses a different configuration.
const BUNDLE_ID: &str = "com.openai.codex";

pub fn find_client() -> Result<String, String> {
    autoreleasepool(|_| {
        let workspace = NSWorkspace::sharedWorkspace();
        let url = workspace
            .URLForApplicationWithBundleIdentifier(&NSString::from_str(BUNDLE_ID))
            .ok_or(
                "未找到 Codex / ChatGPT 工作客户端，请先安装 macOS 版 Codex 客户端并打开一次。",
            )?;
        let path = url
            .path()
            .ok_or("无法读取 macOS 客户端安装位置。")?
            .to_string();
        let path = Path::new(&path)
            .canonicalize()
            .map_err(|_| "客户端安装位置不可用，请重新安装后重试。")?;
        if !path.join("Contents/Info.plist").is_file() {
            return Err("客户端安装不完整，请重新安装后重试。".to_owned());
        }
        Ok(path.to_string_lossy().into_owned())
    })
}

fn matches_bundle(candidate: &Path, selected: &Path) -> bool {
    candidate == selected || candidate.starts_with(selected.join("Contents"))
}

fn running_clients(selected: &Path) -> Vec<Retained<NSRunningApplication>> {
    NSWorkspace::sharedWorkspace()
        .runningApplications()
        .iter()
        .filter(|app| {
            app.bundleURL()
                .and_then(|url| url.path())
                .map(|path| {
                    let path = PathBuf::from(path.to_string());
                    matches_bundle(&path.canonicalize().unwrap_or(path), selected)
                })
                .unwrap_or(false)
        })
        .collect()
}

pub fn stop_client(app_path: &str) -> Result<(), String> {
    autoreleasepool(|_| {
        let apps = running_clients(Path::new(app_path));
        // Native AppKit termination does not require Apple Events automation permission.
        for app in &apps {
            app.terminate();
        }
        let start = Instant::now();
        while apps.iter().any(|app| !app.isTerminated()) {
            if start.elapsed() >= Duration::from_secs(10) {
                return Err("客户端未能退出，请完全退出 ChatGPT / Codex 后重试。".to_owned());
            }
            if start.elapsed() >= Duration::from_secs(5) {
                for app in &apps {
                    if !app.isTerminated() {
                        app.forceTerminate();
                    }
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    })
}

pub fn launch_client(app_path: String) -> Result<u32, String> {
    let status = Command::new("/usr/bin/open")
        .arg("-a")
        .arg(&app_path)
        .status()
        .map_err(|_| "无法启动客户端，请从「应用程序」手动打开 ChatGPT / Codex。")?;
    if !status.success() {
        return Err("macOS 未能启动客户端，请从「应用程序」手动打开 ChatGPT / Codex。".to_owned());
    }
    autoreleasepool(|_| {
        let path = Path::new(&app_path);
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            // Exclude nested helper apps: wait for the selected main app to launch.
            if let Some(app) = running_clients(path).into_iter().find(|app| {
                app.bundleIdentifier()
                    .is_some_and(|id| id.to_string() == BUNDLE_ID)
                    && !app.isTerminated()
            }) {
                let pid = app.processIdentifier();
                if pid > 0 {
                    return Ok(pid as u32);
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("已请求打开客户端，但未检测到启动完成，请手动打开 ChatGPT / Codex 检查。".to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_is_scoped_to_the_selected_app_bundle() {
        let selected = Path::new("/Applications/Codex.app");
        assert!(matches_bundle(selected, selected));
        assert!(matches_bundle(
            Path::new("/Applications/Codex.app/Contents/Frameworks/Helper.app"),
            selected
        ));
        assert!(!matches_bundle(
            Path::new("/Applications/Codex.app.old"),
            selected
        ));
        assert!(!matches_bundle(
            Path::new("/Applications/ChatGPT.app"),
            selected
        ));
        assert!(!matches_bundle(Path::new("/usr/local/bin/codex"), selected));
    }
}
