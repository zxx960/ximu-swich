import { FormEvent, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Account = { email: string; balance: number };
type EnableResult = { balance: number; key_created: boolean; backup_path: string };
type RestoreResult = { backup_path: string; rollback_path: string };
type KeyGroup = { id: number; name: string; platform: string };
type SavedCredentials = { email: string; password: string };

function formatBalance(value: number) {
  return new Intl.NumberFormat("zh-CN", { style: "currency", currency: "USD" }).format(value);
}

export default function App() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [account, setAccount] = useState<Account | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const [groups, setGroups] = useState<KeyGroup[]>([]);
  const [selectedGroupId, setSelectedGroupId] = useState("");
  const [groupDialogOpen, setGroupDialogOpen] = useState(false);
  const [restoringSession, setRestoringSession] = useState(true);
  const [rememberCredentials, setRememberCredentials] = useState(false);

  useEffect(() => {
    let active = true;
    async function restoreSession() {
      try {
        const [restoredAccount, savedCredentials] = await Promise.all([
          invoke<Account | null>("restore_session"),
          invoke<SavedCredentials | null>("get_saved_credentials"),
        ]);
        if (active && savedCredentials) { setEmail(savedCredentials.email); setPassword(savedCredentials.password); setRememberCredentials(true); }
        if (active && restoredAccount) { setAccount(restoredAccount); setStatus("已恢复登录会话。"); }
      } catch {
        // Leave the user on the login screen when a stored session cannot be restored.
      } finally {
        if (active) setRestoringSession(false);
      }
    }
    void restoreSession();
    return () => { active = false; };
  }, []);

  async function signIn(event: FormEvent) {
    event.preventDefault();
    if (!email.trim() || !password) { setError("请输入邮箱和密码。"); return; }
    setBusy(true); setError(""); setStatus("正在登录...");
    try {
      const result = await invoke<Account>("login", { email: email.trim(), password, rememberCredentials });
      setAccount(result); setPassword(""); setStatus("已连接到账户。");
    } catch (reason) { setPassword(""); setStatus(""); setError(String(reason)); }
    finally { setBusy(false); }
  }

  async function toggleRememberCredentials(checked: boolean) {
    setRememberCredentials(checked);
    if (!checked) {
      try { await invoke("forget_saved_credentials"); }
      catch (reason) { setError(String(reason)); }
    }
  }

  async function openGroupDialog() {
    setBusy(true); setError(""); setStatus("正在加载可用密钥分组...");
    try {
      const availableGroups = await invoke<KeyGroup[]>("list_available_groups");
      if (availableGroups.length === 0) { setStatus(""); setError("当前账户没有可用的密钥分组，请先在中转站后台配置分组权限。"); return; }
      setGroups(availableGroups); setSelectedGroupId(""); setGroupDialogOpen(true); setStatus("");
    } catch (reason) { setStatus(""); setError(String(reason)); }
    finally { setBusy(false); }
  }

  async function enable() {
    const groupId = Number(selectedGroupId);
    if (!Number.isInteger(groupId) || groupId <= 0) { setError("请选择密钥分组。"); return; }
    setBusy(true); setError(""); setStatus("正在创建或恢复专用密钥...");
    try {
      const result = await invoke<EnableResult>("enable_codex", { groupId });
      setAccount((current) => current ? { ...current, balance: result.balance } : current);
      setGroupDialogOpen(false);
      setStatus(result.key_created ? "已启用 Codex，已创建专用密钥。" : "已启用 Codex，已恢复现有专用密钥。");
    } catch (reason) { setStatus(""); setError(String(reason)); }
    finally { setBusy(false); }
  }

  async function restoreOriginalCodexConfig() {
    if (!window.confirm("将使用第一次启用前的 Codex 配置覆盖当前配置，恢复后请重启 Codex。是否继续？")) return;
    setBusy(true); setError(""); setStatus("正在恢复原 Codex 配置...");
    try {
      await invoke<RestoreResult>("restore_original_codex_config");
      setStatus("已恢复原 Codex 配置，请完全退出并重新打开 Codex。");
    } catch (reason) { setStatus(""); setError(String(reason)); }
    finally { setBusy(false); }
  }

  async function signOut() { await invoke("logout"); setAccount(null); setPassword(""); setStatus(""); setError(""); setGroupDialogOpen(false); setGroups([]); setSelectedGroupId(""); }

  return <main className="app-shell">
    <header className="topbar"><div className="brand"><span className="brand-mark">C</span><span>Codex Quick Connect</span></div>{account && <button className="quiet-button" type="button" onClick={signOut} aria-label="退出登录">退出</button>}</header>
    {restoringSession ? <section className="content login-view"><div className="eyebrow">XIMUAI CODEX</div><h1>正在恢复登录...</h1><p className="subtle">正在验证本机加密会话。</p></section> : !account ? <section className="content login-view" aria-labelledby="login-title"><div className="eyebrow">XIMUAI CODEX</div><h1 id="login-title">连接你的 Codex</h1><p className="subtle">登录后自动创建密钥并配置本机 Codex。</p><form onSubmit={signIn} className="form-stack"><label>邮箱<input type="email" autoComplete="username" value={email} onChange={(e) => setEmail(e.target.value)} disabled={busy} /></label><label>密码<input type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} disabled={busy} /></label><label className="remember-option"><input type="checkbox" checked={rememberCredentials} onChange={(event) => void toggleRememberCredentials(event.target.checked)} disabled={busy} />保存账号和密码</label><button className="primary-button" disabled={busy} type="submit">{busy ? "正在连接..." : "登录"}</button></form></section> : <section className="content connected-view" aria-labelledby="account-title"><div className="connection-chip">已连接</div><h1 id="account-title">{account.email}</h1><div className="balance-panel"><span>API 账户余额</span><strong>{formatBalance(account.balance)}</strong></div><button className="primary-button enable-button" onClick={openGroupDialog} disabled={busy} type="button">{busy ? "正在加载..." : "启用 Codex"}</button><p className="subtle configuration-note">启用会备份并替换当前 Codex 配置。</p><button className="secondary-button restore-button" onClick={() => void restoreOriginalCodexConfig()} disabled={busy} type="button">恢复原 Codex 配置</button></section>}
    {groupDialogOpen && <div className="dialog-backdrop" role="presentation"><section className="group-dialog" role="dialog" aria-modal="true" aria-labelledby="group-dialog-title"><h2 id="group-dialog-title">选择密钥分组</h2><p>请选择本次 Codex 连接使用的中转站分组。</p><label>密钥分组<select value={selectedGroupId} onChange={(event) => setSelectedGroupId(event.target.value)} disabled={busy}><option value="" disabled>请选择分组</option>{groups.map((group) => <option key={group.id} value={group.id}>{group.name}（{group.platform}）</option>)}</select></label><div className="dialog-actions"><button className="secondary-button" type="button" onClick={() => setGroupDialogOpen(false)} disabled={busy}>取消</button><button className="primary-button confirm-button" type="button" onClick={enable} disabled={busy || !selectedGroupId}>{busy ? "正在启用..." : "确认启用"}</button></div></section></div>}
    <footer aria-live="polite" className={error ? "message error" : "message"}>{error || status || "https://codex.ximuai.com"}</footer>
  </main>;
}
