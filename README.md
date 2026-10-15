# Codex Quick Connect

一个免安装的 Windows 桌面工具，用于登录 XimuAI 账户、选择密钥分组，并自动配置本机 Codex。基于 Tauri 2、React、TypeScript 和 Rust 开发。

## 系统要求

- Windows 10 或 Windows 11
- Microsoft Edge WebView2 Runtime（当前 Windows 通常已预装）
- 已安装 Codex CLI；本程序不负责安装 Codex
- 有可用余额和密钥分组权限的 XimuAI 账户

## 使用

1. 双击 `Codex Quick Connect.exe`。
2. 输入 XimuAI 的邮箱和密码并登录；可按需勾选“保存账号和密码”。
3. 确认显示的 API 账户余额；登录或恢复会话后，程序会自动加载可用密钥分组。
4. 在余额下方选择密钥分组，程序会自动拉取该分组的可用 API 密钥；确认或选择密钥后，点击“启用 Codex”。若该分组没有可用密钥，启用时会创建专用密钥。
5. 等待启用成功后，在终端运行 `codex`。如果 Codex 已经打开，请完全退出后重新打开。

如果没有可用的密钥分组，请先在中转站后台配置分组权限；分组加载失败或暂无分组时，可以点击“重新加载分组”重试。密钥列表支持刷新，加载失败时可重新加载。加载期间或未选择分组、密钥时，无法启用 Codex。

## 登录与密钥

- 登录成功后，会在本机加密保存登录会话。下次打开程序时会联网验证并尝试恢复登录；验证失败时需要重新登录。
- 只有勾选“保存账号和密码”后，才会将邮箱和密码加密保存到本机，用于下次自动填充。取消勾选会删除已保存的账号密码，但不会清除登录会话。
- 点击“退出”会清除本机登录会话和已保存的账号密码，不会撤销 API Key 或恢复 Codex 配置。
- 选择分组后，会分页拉取该分组的密钥，仅列出启用中、未过期且未耗尽总额度的密钥。界面显示名称和脱敏尾号，优先选中名为 `Codex Quick Connect` 的密钥，否则选中第一个可用密钥；可在下拉框中切换。
- 启用时会重新验证所选密钥的分组和可用状态，再使用该密钥配置 Codex，不会修改其他密钥的所属分组。
- 当前分组没有可用密钥时，启用会先检查是否已有可用的 `Codex Quick Connect` 专用密钥，存在则复用，否则创建。选择分组本身不会创建密钥或写入 Codex 配置。
- 登录会话、可选保存的账号密码和 API Key 本地记录，均使用当前 Windows 用户的 DPAPI 加密。

## Codex 配置与备份

启用会整体替换以下文件，不会合并原有自定义配置：

- `%USERPROFILE%\.codex\auth.json`
- `%USERPROFILE%\.codex\config.toml`

当前版本写入的主要设置固定如下，界面暂不支持修改：

| 设置 | 值 |
| --- | --- |
| 服务地址 | `https://codex.ximuai.com` |
| API 协议 | Responses API（`responses`） |
| 模型与审查模型 | `gpt-5.5` |
| 推理强度 | `xhigh` |

此外，生成的配置会设置 `disable_response_storage = true`、`network_access = "enabled"`、`windows_wsl_setup_acknowledged = true`，并启用 `features.goals`。

每次覆盖前，已有文件会备份到 `%LOCALAPPDATA%\Codex Quick Connect\backups\YYYYMMDD-HHMMSS`。配置写入失败时，程序会尝试恢复覆盖前的文件。

### 恢复原 Codex 配置

登录后点击“恢复原 Codex 配置”，确认后会使用备份目录中**时间最早、且同时包含 `auth.json` 和 `config.toml` 的时间戳备份**覆盖当前配置。恢复完成后，请完全退出并重新打开 Codex。

- 恢复前会将当前已有配置另存到 `backups\restore-rollback-YYYYMMDD-HHMMSS`。
- 恢复写入失败时，会尝试还原恢复操作前的配置。
- 如果首次启用前缺少其中一个配置文件，该次备份不会被恢复功能选中；如果没有符合条件的备份，程序会提示无法恢复。

## 本地数据与安全

应用数据保存在 `%LOCALAPPDATA%\Codex Quick Connect`：

| 文件或目录 | 内容 |
| --- | --- |
| `session.bin` | DPAPI 加密的登录会话 |
| `credentials.bin` | 选择保存时写入的 DPAPI 加密邮箱和密码 |
| `provisioning.bin` | DPAPI 加密的 API Key 及分组记录 |
| `backups` | Codex 配置备份 |

不要把账户密码或 API Key 发给他人。Codex 的 `auth.json` 中保存的是明文 API Key；配置备份也按原文件内容保存，可能包含密钥。请妥善保护这些文件，避免公开上传或分享。

## 开发与构建

开发环境需要 Node.js、pnpm、Rust、Windows MSVC 构建工具和 WebView2 Runtime。

在项目根目录安装依赖并启动桌面开发模式：

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

构建桌面程序：

```powershell
pnpm tauri build
```

Rust release 产物位于 `src-tauri\target\release`（未自定义 Cargo 输出目录时）。当前 Tauri 配置的 `bundle.targets` 为空，不生成安装包。

其他常用命令：

- `pnpm dev`：仅启动 Vite 前端开发服务器；原生功能需要在 Tauri 环境中使用。
- `pnpm build`：运行 TypeScript 检查并构建前端。

## 项目结构

```text
src/
  App.tsx                 登录、账户、分组选择和配置恢复界面
  App.css                 界面样式
  main.tsx                React 入口
src-tauri/
  src/lib.rs              API 请求、DPAPI 加密、密钥管理和配置备份恢复
  src/main.rs             桌面程序入口
  tauri.conf.json         窗口与构建配置
  capabilities/           Tauri 权限配置
```
