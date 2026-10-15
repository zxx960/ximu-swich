# Codex Quick Connect

一个免安装的 Windows 小程序，用于把 XimuAI 的 API 账户直接配置到本机 Codex。

## 使用

1. 双击 `Codex Quick Connect.exe`。
2. 输入 XimuAI 的邮箱和密码并登录。
3. 确认显示的 API 账户余额。
4. 点击“启用 Codex”。
5. 之后可在终端直接使用 `codex`。

## 行为说明

- 程序不保存登录密码；关闭或退出后需重新登录。
- 第一次启用会创建名为 `Codex Quick Connect` 的 API Key，后续启用会复用它。
- API Key 的本地记录使用当前 Windows 用户的 DPAPI 加密。
- 启用会覆盖 `%USERPROFILE%\.codex\auth.json` 和 `%USERPROFILE%\.codex\config.toml`。
- 覆盖前的文件会备份到 `%LOCALAPPDATA%\Codex Quick Connect\backups`。写入失败时会尝试自动恢复旧文件。
- 本版本固定使用 `https://codex.ximuai.com`、Responses API 和 `gpt-5.5`。

## 系统要求

- Windows 10 或 Windows 11
- Microsoft Edge WebView2 Runtime（当前 Windows 通常已预装）
- 已安装 Codex CLI
- 有可用余额的 XimuAI 账户

## 安全

不要把账户密码或 API Key 发给他人。Codex 需要在其自己的 `auth.json` 中保存 API Key，因此该文件应仅由当前 Windows 用户访问。
