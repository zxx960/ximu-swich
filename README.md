# 西牧ai

一个支持 Windows 和 macOS 的桌面工具，用于登录 XimuAI 账户、选择密钥分组，并配置本机 ChatGPT / Codex 和 Claude Code CLI。基于 Tauri 2、React、TypeScript 和 Rust 开发。

## 系统要求

- Windows 10 / 11，或 macOS 12 及以上；目标客户端还需满足其自身的系统与处理器要求。
- Windows 需要 Microsoft Edge WebView2 Runtime；macOS 使用系统 WebKit。
- 配置 ChatGPT / Codex 时需先安装相应桌面客户端：Windows 支持 OpenAI 的 ChatGPT / Codex 应用注册；macOS 按应用标识 `com.openai.codex` 查找客户端，支持其显示名称为 Codex 或 ChatGPT，不匹配经典版 ChatGPT（`com.openai.chat`）。安装后请先打开一次。
- 配置 Claude Code 时需自行安装 Claude Code CLI，无需安装 OpenAI 客户端；本工具不会安装 CLI。
- 有可用余额和密钥分组权限的 XimuAI 账户

## 使用

1. Windows 双击 `西牧ai.exe`；macOS 打开 `西牧ai.app`。
2. 输入 XimuAI 的邮箱和密码并登录；可按需勾选“保存账号和密码”。
3. 确认显示的 API 账户余额；在余额下方切换“ChatGPT”或“Claude Code”tab。ChatGPT 仅显示 OpenAI 分组，Claude Code 仅显示 Anthropic 分组。登录或切换 tab 后会自动加载对应分组。
4. 在余额下方选择密钥分组，程序会自动拉取该分组的可用 API 密钥；确认或选择密钥后，在其下方选择模型（默认 `gpt-6.1-sol`），再点击“启动ChatGPT”。若该分组没有可用密钥，启动前会创建专用密钥。
5. 程序会先检查客户端安装状态，再保存配置，随后关闭现有客户端并等待退出，再通过系统应用服务重新打开，让新配置生效。Windows 关闭所选应用的进程树；macOS 关闭所选应用及其应用包内的已注册辅助应用，等待正常退出，超过 5 秒尝试强制退出，10 秒后仍未退出则停止并报错。客户端尚未运行时则直接启动。

每次点击“启动ChatGPT”都会重新启动客户端，正在进行的任务会中断。仅关闭所选应用范围内的进程，不按名称批量终止其他 CLI 或 IDE。未安装客户端时会提示安装，不会创建密钥或覆盖配置；关闭失败或等待退出超时会停止后续启动并提示手动退出后重试；配置保存成功但重启失败时，会明确提示“配置已保存，但客户端未能按新配置启动”。

如果没有可用的 OpenAI 密钥分组，请先在中转站后台配置分组权限；分组加载失败或暂无分组时，可以点击“重新加载分组”重试。密钥列表支持刷新，加载失败时可重新加载。加载期间或未选择分组、密钥时，无法启用 Codex。

## 登录与密钥

- 登录成功后，会在本机加密保存登录会话。下次打开程序时会联网验证并尝试恢复登录；验证失败时需要重新登录。
- 只有勾选“保存账号和密码”后，才会将邮箱和密码加密保存到本机，用于下次自动填充。取消勾选会删除已保存的账号密码，但不会清除登录会话。
- 点击“退出”会清除本机登录会话和已保存的账号密码，不会撤销 API Key 或恢复 Codex 配置。
- 分组、API 密钥选择和模型会按登录邮箱和客户端类型分别记住；切换 tab、重启或重新登录同一账号后，会自动回显。原有 ChatGPT 选择继续可用。首次使用此功能需要先选择一次，回显本身不会自动启动客户端或写入配置。
- 回显时会重新拉取分组和密钥。原分组不可用时提示重新选择；原密钥不可用时提示确认其他可用密钥，没有可用密钥则恢复为启动时创建。网络请求失败时可以重试，不会使用缓存密钥启动。
- 选择分组后，会分页拉取该分组的密钥，仅列出启用中、未过期且未耗尽总额度的密钥。界面显示名称和脱敏尾号，优先回显上次选择；没有已保存的可用选择时，优先选中名为 `西牧ai` 的密钥，否则选中第一个可用密钥；可在下拉框中切换。
- 启用时会重新验证所选密钥的分组和可用状态，再使用该密钥配置 Codex，不会修改其他密钥的所属分组。
- 当前分组没有可用密钥时，启用会先检查是否已有可用的 `西牧ai` 专用密钥，存在则复用，否则创建名为 `西牧ai` 的密钥。选择分组本身不会创建密钥或写入 Codex 配置。
- 登录会话、可选保存的账号密码和 API Key 本地记录：Windows 继续使用当前用户的 DPAPI 加密，兼容原有记录；macOS 使用系统钥匙串，不将这些凭据另存为明文文件。钥匙串访问被拒绝时会报错，不会回退到明文存储。

## Claude Code CLI 配置

在“Claude Code”tab 选择 Anthropic 分组及其 API 密钥，再选择模型，点击“保存 Claude Code 配置”。模型下拉框依次提供 `claude-sonnet-5`、`claude-opus-5-5`、`claude-fable-5-1`，默认选择第一个 `claude-sonnet-5`；已保存的有效选择会继续回显，旧版模型别名或自定义选择会回退到默认模型。

程序重新验证分组平台和密钥可用状态后，将配置合并写入用户主目录下的 `.claude/settings.json`（Windows 为 `%USERPROFILE%\.claude\settings.json`，macOS 为 `~/.claude/settings.json`）；设置了 `CLAUDE_CONFIG_DIR` 时使用该目录下的 `settings.json`，位置规则见 [Claude Code 配置说明](https://code.claude.com/docs/en/settings)。

- 写入 `env.ANTHROPIC_BASE_URL = "https://codex.ximuai.com"`、`env.ANTHROPIC_AUTH_TOKEN`，以及 `model` 和 `env.ANTHROPIC_MODEL`。
- 保留主题、权限、hooks、其他环境变量等现有设置；移除该文件 `env` 中冲突的 `ANTHROPIC_API_KEY`、`CLAUDE_CODE_OAUTH_TOKEN` 及 Bedrock / Vertex / Foundry 提供商开关。
- 没有可用密钥时，保存前复用或创建同分组下名为“西牧ai”的密钥。界面只显示密钥名称和脱敏尾号。
- 无效 JSON 或无效 `env` 结构会报错并保留原文件；保存时通过临时文件替换，不创建历史备份。
- 保存成功后重新打开 Claude Code CLI，再运行 `claude`。本操作不关闭 CLI 进程、不启动 ChatGPT，也不修改 `.codex` 配置或 Claude 登录缓存。

设置文件中的密钥为明文，请妥善保护。已有终端的环境变量、项目配置或组织托管设置可能覆盖用户配置；如果未生效，可用 Claude Code 的 `/status` 检查配置来源，优先级见[官方说明](https://code.claude.com/docs/en/settings#settings-precedence)。

## Codex 配置

启用会整体替换以下文件，不会合并原有自定义配置：

- Windows：`%USERPROFILE%\.codex\auth.json` 和 `%USERPROFILE%\.codex\config.toml`
- macOS：`~/.codex/auth.json` 和 `~/.codex/config.toml`

当前版本写入的主要设置如下；模型可在界面中选择，其余设置固定：

| 设置 | 值 |
| --- | --- |
| 服务地址 | `https://codex.ximuai.com` |
| API 协议 | Responses API（`responses`） |
| 模型与审查模型 | 使用界面所选模型，默认 `gpt-6.1-sol` |
| 推理强度 | 高（`high`） |

模型下拉框提供 `gpt-6.1-sol`、`gpt-6-sol`、`gpt-6-astra`、`gpt-5.6-sol` 和 `gpt-5.6-terra`。切换模型本身不会写入配置，点击“启动ChatGPT”后生效。

此外，生成的配置会设置 `disable_response_storage = true`、`network_access = "enabled"`，并启用 `features.goals`。只有 Windows 写入 `windows_wsl_setup_acknowledged = true`；macOS 指定 `cli_auth_credentials_store = "file"`，确保客户端读取本次写入的 API 密钥，而非复用原有系统凭据。

启动和重置登录时都不生成配置备份。操作过程中仅在内存中暂存原文件内容，写入失败时尝试还原；操作成功后不保留历史配置副本。历史版本已经生成的备份文件保留原状。

### 重置 ChatGPT 客户端登录

登录后点击“恢复原 ChatGPT 配置”，确认后会重置客户端登录，让重新打开的客户端进入登录流程：

1. 关闭已安装的 ChatGPT 客户端及其子进程，客户端中正在进行的任务会中断。
2. 清除 `.codex\auth.json`，将 `config.toml` 重置为 `cli_auth_credentials_store = "file"`，使用默认服务连接和模型设置，避免复用系统凭据库中的旧登录。
3. 重新打开客户端，提示用户重新登录。若启动失败，会提示手动打开客户端。

文件重置失败时，会使用内存中暂存的内容尝试还原操作前的配置。

连接工具的 XimuAI 登录、平台 API 密钥和聊天历史保持原样。`.codex` 登录缓存也可能由 CLI 和 IDE 扩展共用，后续使用这些工具时可能需要重新登录；凭据存储方式见 [OpenAI 官方登录说明](https://learn.chatgpt.com/docs/auth#credential-storage)。

## 本地数据与安全

Windows 应用数据继续保存在 `%LOCALAPPDATA%\Codex Quick Connect`：

| 文件或目录 | 内容 |
| --- | --- |
| `session.bin` | DPAPI 加密的登录会话 |
| `credentials.bin` | 选择保存时写入的 DPAPI 加密邮箱和密码 |
| `provisioning.bin` | DPAPI 加密的 API Key 及分组记录 |

macOS 将上述三类凭据保存在系统钥匙串中，服务名为 `com.ximuai.codex-quick-connect`，条目名分别为 `session`、`credentials` 和 `provisioning`。首次访问时系统可能请求授权；请允许本应用访问。拒绝或锁定钥匙串不会自动删除原有凭据。

macOS 写入的 Codex 和 Claude 配置文件权限为 `0600`，仅当前用户可读写；通过同目录临时文件原子替换已有配置，不创建历史备份。

界面选择保存在本机 WebView 的 localStorage 中，按邮箱区分，仅包含分组编号、密钥编号和模型名称，不包含密码或 API 密钥原文。退出登录保留这些选择，其他账号不会回显它们。

不要把账户密码或 API Key 发给他人。Codex 的 `auth.json` 中保存的是明文 API Key，请妥善保护该文件，避免公开上传或分享。

## 开发与构建

开发环境需要 Node.js 22、pnpm 10 和 Rust。Windows 另需 MSVC 构建工具和 WebView2 Runtime；macOS 另需 Xcode Command Line Tools，可运行 `xcode-select --install` 安装。构建 macOS 应用需在 Mac 上执行，参见 [Tauri macOS 构建说明](https://v2.tauri.app/distribute/macos-application-bundle/)。

在项目根目录安装依赖并启动桌面开发模式：

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

构建桌面程序：

```powershell
pnpm tauri build
```

Windows release 产物为 `src-tauri\target\release\西牧ai.exe`（未自定义 Cargo 输出目录时），默认不生成安装包。macOS 自动合并 `src-tauri/tauri.macos.conf.json`，默认生成 `.app` 和 `.dmg`，产物位于 `src-tauri/target/release/bundle/`。

macOS 应用显示名称仍为“西牧ai”，包内可执行文件使用 ASCII 名称 `ximuai`，避免中文可执行文件名触发 WebKit 初始化时的 `NSBundle` 查询崩溃。构建后可运行 `python3 scripts/check-macos-startup.py src-tauri/target/release/bundle/macos/西牧ai.app`，检查真实应用能否启动并持续运行 30 秒。CI 在 macOS 15（Apple Silicon / Intel）和 macOS 26 上执行这一检查；它会关闭自己启动的测试进程。

Mac 上的开发与打包命令相同：

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
# 退出开发模式后构建
pnpm tauri build
```

默认按当前 Mac 的架构构建。需要同时支持 Apple Silicon 和 Intel 的通用包时，在 Mac 上执行：

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm tauri build --target universal-apple-darwin
```

通用包位于 `src-tauri/target/universal-apple-darwin/release/bundle/`。面向其他用户分发时，请配置自己的 Apple 签名与公证；本项目未内置签名凭据。

`.github/workflows/desktop-check.yml` 提供 Windows、Apple Silicon Mac 和 Intel Mac 的测试与构建检查。默认测试不启动真实客户端、不调用收费 API，也不访问真实钥匙串；钥匙串读写测试需在 Mac 上单独运行 `cargo test --manifest-path src-tauri/Cargo.toml keychain_record_round_trip -- --ignored`，仅创建并清理独立的测试条目。

其他常用命令：

- `pnpm dev`：仅启动 Vite 前端开发服务器；原生功能需要在 Tauri 环境中使用。
- `pnpm build`：运行 TypeScript 检查并构建前端。

## 项目结构

```text
src/
  App.tsx                 登录、账户、客户端 tab、分组选择和配置界面
  App.css                 界面样式
  main.tsx                React 入口
src-tauri/
  src/lib.rs              API 请求、密钥管理、配置写入和登录重置
  src/storage.rs          Windows DPAPI / macOS 钥匙串凭据存储
  src/files.rs            跨平台配置替换与 macOS 文件权限
  src/client.rs           按系统选择客户端控制实现
  src/client_windows.rs   Windows 客户端发现与重启
  src/client_macos.rs     macOS 客户端发现与重启
  src/claude.rs           Claude Code CLI 用户配置合并和文件写入
  src/main.rs             桌面程序入口
  tauri.conf.json         窗口与构建配置
  tauri.macos.conf.json   macOS 应用包与最低系统版本
  capabilities/           Tauri 权限配置
```
