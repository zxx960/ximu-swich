# Codex Quick Connect

一个免安装的 Windows 桌面工具，用于登录 XimuAI 账户、选择密钥分组，并自动配置本机 Codex。基于 Tauri 2、React、TypeScript 和 Rust 开发。

## 系统要求

- Windows 10 或 Windows 11
- Microsoft Edge WebView2 Runtime（当前 Windows 通常已预装）
- 已安装 Codex CLI；本程序不负责安装 Codex
- 已安装可从 Windows 开始菜单打开的 ChatGPT 客户端（支持 OpenAI 的 ChatGPT / Codex 应用注册）
- 有可用余额和密钥分组权限的 XimuAI 账户

## 使用

1. 双击 `Codex Quick Connect.exe`。
2. 输入 XimuAI 的邮箱和密码并登录；可按需勾选“保存账号和密码”。
3. 确认显示的 API 账户余额；登录或恢复会话后，程序会自动加载可用密钥分组，下拉框仅显示平台为 OpenAI 的分组。
4. 在余额下方选择密钥分组，程序会自动拉取该分组的可用 API 密钥；确认或选择密钥后，在其下方选择模型（默认 `gpt-6.1-sol`），再点击“启动ChatGPT”。若该分组没有可用密钥，启动前会创建专用密钥。
5. 程序会先检查客户端安装状态，再保存配置，随后关闭现有 ChatGPT 客户端及其子进程，等待退出后通过 Windows 应用启动服务重新打开，让新配置生效。客户端尚未运行时则直接启动。之后也可在终端运行 `codex`。

每次点击“启动ChatGPT”都会重新启动客户端，正在进行的任务会中断。仅关闭对应 OpenAI 应用安装目录下的客户端进程树。未安装客户端时会提示安装，不会创建密钥或覆盖配置；关闭失败或等待退出超时会停止后续启动并提示手动退出后重试；配置保存成功但重启失败时，会明确提示“配置已保存，但客户端未能按新配置启动”。

如果没有可用的 OpenAI 密钥分组，请先在中转站后台配置分组权限；分组加载失败或暂无分组时，可以点击“重新加载分组”重试。密钥列表支持刷新，加载失败时可重新加载。加载期间或未选择分组、密钥时，无法启用 Codex。

## 登录与密钥

- 登录成功后，会在本机加密保存登录会话。下次打开程序时会联网验证并尝试恢复登录；验证失败时需要重新登录。
- 只有勾选“保存账号和密码”后，才会将邮箱和密码加密保存到本机，用于下次自动填充。取消勾选会删除已保存的账号密码，但不会清除登录会话。
- 点击“退出”会清除本机登录会话和已保存的账号密码，不会撤销 API Key 或恢复 Codex 配置。
- 分组、API 密钥选择和模型会按登录邮箱分别记住；重启或重新登录同一账号后，会自动回显。首次使用此功能需要先选择一次，回显本身不会自动启动客户端或写入 Codex 配置。
- 回显时会重新拉取分组和密钥。原分组不可用时提示重新选择；原密钥不可用时提示确认其他可用密钥，没有可用密钥则恢复为启动时创建。网络请求失败时可以重试，不会使用缓存密钥启动。
- 选择分组后，会分页拉取该分组的密钥，仅列出启用中、未过期且未耗尽总额度的密钥。界面显示名称和脱敏尾号，优先回显上次选择；没有已保存的可用选择时，优先选中名为 `西牧ai` 的密钥，否则选中第一个可用密钥；可在下拉框中切换。
- 启用时会重新验证所选密钥的分组和可用状态，再使用该密钥配置 Codex，不会修改其他密钥的所属分组。
- 当前分组没有可用密钥时，启用会先检查是否已有可用的 `西牧ai` 专用密钥，存在则复用，否则创建名为 `西牧ai` 的密钥。选择分组本身不会创建密钥或写入 Codex 配置。
- 登录会话、可选保存的账号密码和 API Key 本地记录，均使用当前 Windows 用户的 DPAPI 加密。

## Codex 配置

启用会整体替换以下文件，不会合并原有自定义配置：

- `%USERPROFILE%\.codex\auth.json`
- `%USERPROFILE%\.codex\config.toml`

当前版本写入的主要设置如下；模型可在界面中选择，其余设置固定：

| 设置 | 值 |
| --- | --- |
| 服务地址 | `https://codex.ximuai.com` |
| API 协议 | Responses API（`responses`） |
| 模型与审查模型 | 使用界面所选模型，默认 `gpt-6.1-sol` |
| 推理强度 | 高（`high`） |

模型下拉框提供 `gpt-6.1-sol`、`gpt-6-sol`、`gpt-6-astra`、`gpt-5.6-sol` 和 `gpt-5.6-terra`。切换模型本身不会写入配置，点击“启动ChatGPT”后生效。

此外，生成的配置会设置 `disable_response_storage = true`、`network_access = "enabled"`、`windows_wsl_setup_acknowledged = true`，并启用 `features.goals`。

启动和重置登录时都不生成配置备份。操作过程中仅在内存中暂存原文件内容，写入失败时尝试还原；操作成功后不保留历史配置副本。历史版本已经生成的备份文件保留原状。

### 重置 ChatGPT 客户端登录

登录后点击“恢复原 ChatGPT 配置”，确认后会重置客户端登录，让重新打开的客户端进入登录流程：

1. 关闭已安装的 ChatGPT 客户端及其子进程，客户端中正在进行的任务会中断。
2. 清除 `.codex\auth.json`，将 `config.toml` 重置为 `cli_auth_credentials_store = "file"`，使用默认服务连接和模型设置，避免复用系统凭据库中的旧登录。
3. 重新打开客户端，提示用户重新登录。若启动失败，会提示手动打开客户端。

文件重置失败时，会使用内存中暂存的内容尝试还原操作前的配置。

连接工具的 XimuAI 登录、平台 API 密钥和聊天历史保持原样。`.codex` 登录缓存也可能由 CLI 和 IDE 扩展共用，后续使用这些工具时可能需要重新登录；凭据存储方式见 [OpenAI 官方登录说明](https://learn.chatgpt.com/docs/auth#credential-storage)。

## 本地数据与安全

应用数据保存在 `%LOCALAPPDATA%\Codex Quick Connect`：

| 文件或目录 | 内容 |
| --- | --- |
| `session.bin` | DPAPI 加密的登录会话 |
| `credentials.bin` | 选择保存时写入的 DPAPI 加密邮箱和密码 |
| `provisioning.bin` | DPAPI 加密的 API Key 及分组记录 |

界面选择保存在本机 WebView 的 localStorage 中，按邮箱区分，仅包含分组编号、密钥编号和模型名称，不包含密码或 API 密钥原文。退出登录保留这些选择，其他账号不会回显它们。

不要把账户密码或 API Key 发给他人。Codex 的 `auth.json` 中保存的是明文 API Key，请妥善保护该文件，避免公开上传或分享。

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
  src/lib.rs              API 请求、DPAPI 加密、密钥管理、配置写入和登录重置
  src/client.rs           Windows 客户端发现与启动
  src/main.rs             桌面程序入口
  tauri.conf.json         窗口与构建配置
  capabilities/           Tauri 权限配置
```
