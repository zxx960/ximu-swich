# 项目发布约定

当用户说“上传 Cloudflare”（包括“上传cloudflare”等同义表达）时，默认执行以下完整流程，无需再次确认：

1. 确认要发布的最新提交及 Windows x64、macOS 通用版构建产物。缺少对应最新提交的产物时，先完成构建；macOS 通用版通过本仓库 GitHub Actions 生成，支持 Apple Silicon 和 Intel。
2. 使用 Cloudflare CLI（Wrangler）上传到现有 R2 存储桶 `wangpan`。优先使用已有登录，通过 `whoami` 和实际 R2 读取命令检查连接；不要仅凭 OAuth 权限列表判断是否具有 R2 权限。不得在日志、源码或命令参数中暴露凭据。仅在 CLI 确实无法完成且故障无法修复时考虑其他方式。
3. 使用带提交短哈希的文件名区分构建，例如 `西牧ai-Windows-x64-<commit>.exe` 和 `西牧ai-macOS通用版-<commit>.dmg`。当前公开下载域名为 `https://wangpan.ximuai.com`，上传前核实存储桶和域名配置。
4. 验证两个下载链接可访问，并下载比对 SHA-256，确保远程文件与本地构建产物一致。
5. 修改教程文件 `C:\Users\Administrator\Desktop\项目\中转站使用教程\index.html` 中的 Windows 和 Mac 客户端下载链接，保持其他内容不变。
6. 在教程仓库 `https://github.com/zxx960/ximuai-jiaocheng.git` 提交本次链接更新，并推送到其当前工作分支（当前为 `main`）。提交前检查工作区，只暂存本次修改，不夹带用户的其他改动，不强制推送。
7. 返回 Windows、Mac 的下载链接，以及教程仓库的提交与推送结果。有步骤受阻时明确报告实际完成范围，不宣称已全部完成。

本约定授权上述上传、教程链接修改及教程仓库提交推送流程；不包含修改存储桶访问权限、创建新凭据或发布无关改动。
