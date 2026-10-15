import { FormEvent, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Account = { email: string; balance: number };
type AvailableKey = { id: number; name: string; preview: string };
type EnableResult = { balance: number; key_created: boolean; backup_path: string; key: AvailableKey };
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
  const [groupsLoading, setGroupsLoading] = useState(false);
  const [groupError, setGroupError] = useState("");
  const [groupLoadAttempt, setGroupLoadAttempt] = useState(0);
  const [groupKeys, setGroupKeys] = useState<AvailableKey[]>([]);
  const [selectedKeyId, setSelectedKeyId] = useState("");
  const [loadedKeyGroupId, setLoadedKeyGroupId] = useState("");
  const [keysLoading, setKeysLoading] = useState(false);
  const [keyError, setKeyError] = useState("");
  const [keyLoadAttempt, setKeyLoadAttempt] = useState(0);
  const [restoringSession, setRestoringSession] = useState(true);
  const [rememberCredentials, setRememberCredentials] = useState(false);
  const accountEmail = account?.email;
  const keysReady = !!selectedGroupId && loadedKeyGroupId === selectedGroupId && !keysLoading && !keyError;

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

  useEffect(() => {
    if (!accountEmail) return;
    let active = true;
    setGroups([]);
    setSelectedGroupId("");
    setGroupError("");
    setGroupsLoading(true);

    async function loadGroups() {
      try {
        const availableGroups = await invoke<KeyGroup[]>("list_available_groups");
        if (!active) return;
        setGroups(availableGroups);
        if (availableGroups.length === 0) {
          setGroupError("当前账户没有可用的密钥分组，请先在中转站后台配置分组权限。");
        }
      } catch (reason) {
        if (active) setGroupError(String(reason));
      } finally {
        if (active) setGroupsLoading(false);
      }
    }

    void loadGroups();
    return () => { active = false; };
  }, [accountEmail, groupLoadAttempt]);

  useEffect(() => {
    let active = true;
    setGroupKeys([]);
    setSelectedKeyId("");
    setLoadedKeyGroupId("");
    setKeyError("");
    setKeysLoading(false);
    if (!accountEmail || !selectedGroupId) return;
    setKeysLoading(true);

    async function loadKeys() {
      try {
        const keys = await invoke<AvailableKey[]>("list_group_keys", { groupId: Number(selectedGroupId) });
        if (!active) return;
        setGroupKeys(keys);
        setLoadedKeyGroupId(selectedGroupId);
        const preferred = keys.find((key) => key.name === "Codex Quick Connect") ?? keys[0];
        setSelectedKeyId(preferred ? String(preferred.id) : "create");
      } catch (reason) {
        if (active) setKeyError(String(reason));
      } finally {
        if (active) setKeysLoading(false);
      }
    }

    void loadKeys();
    return () => { active = false; };
  }, [accountEmail, selectedGroupId, keyLoadAttempt]);

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

  async function enable() {
    if (busy || groupsLoading || !keysReady) return;
    const groupId = Number(selectedGroupId);
    if (!groups.some((group) => group.id === groupId)) { setError("请选择密钥分组。"); return; }
    const keyId = selectedKeyId === "create" ? null : Number(selectedKeyId);
    if (keyId === null ? groupKeys.length !== 0 : !groupKeys.some((key) => key.id === keyId)) {
      setError("请选择当前分组的 API 密钥。"); return;
    }
    setBusy(true); setError(""); setStatus("正在配置 Codex...");
    try {
      const result = await invoke<EnableResult>("enable_codex", { groupId, keyId });
      setAccount((current) => current ? { ...current, balance: result.balance } : current);
      setGroupKeys((current) => [...current.filter((key) => key.id !== result.key.id), result.key]);
      setSelectedKeyId(String(result.key.id));
      setStatus(result.key_created ? "已启用 Codex，已创建专用密钥。" : "已启用 Codex，已使用所选密钥。");
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

  async function signOut() { await invoke("logout"); setAccount(null); setPassword(""); setStatus(""); setError(""); setGroups([]); setSelectedGroupId(""); setGroupError(""); setGroupsLoading(false); setGroupKeys([]); setSelectedKeyId(""); setLoadedKeyGroupId(""); setKeyError(""); setKeysLoading(false); }

  return <main className="app-shell">
    <header className="topbar"><div className="brand"><span className="brand-mark">C</span><span>Codex Quick Connect</span></div>{account && <button className="quiet-button" type="button" onClick={signOut} aria-label="退出登录">退出</button>}</header>
    {restoringSession ? <section className="content login-view"><div className="eyebrow">XIMUAI CODEX</div><h1>正在恢复登录...</h1><p className="subtle">正在验证本机加密会话。</p></section> : !account ? <section className="content login-view" aria-labelledby="login-title"><div className="eyebrow">XIMUAI CODEX</div><h1 id="login-title">连接你的 Codex</h1><p className="subtle">登录后选择密钥分组，一键配置本机 Codex。</p><form onSubmit={signIn} className="form-stack"><label>邮箱<input type="email" autoComplete="username" value={email} onChange={(e) => setEmail(e.target.value)} disabled={busy} /></label><label>密码<input type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} disabled={busy} /></label><label className="remember-option"><input type="checkbox" checked={rememberCredentials} onChange={(event) => void toggleRememberCredentials(event.target.checked)} disabled={busy} />保存账号和密码</label><button className="primary-button" disabled={busy} type="submit">{busy ? "正在连接..." : "登录"}</button></form></section> : <section className="content connected-view" aria-labelledby="account-title">
      <div className="connection-chip">已连接</div>
      <h1 id="account-title">{account.email}</h1>
      <div className="balance-panel"><span>API 账户余额</span><strong>{formatBalance(account.balance)}</strong></div>
      <div className="group-field" aria-busy={groupsLoading}>
        <label htmlFor="key-group">密钥分组</label>
        <select
          id="key-group"
          value={selectedGroupId}
          onChange={(event) => { setSelectedGroupId(event.target.value); setSelectedKeyId(""); setError(""); setStatus(""); }}
          disabled={busy || groupsLoading || groups.length === 0}
          aria-describedby={groupError ? "group-error" : undefined}
        >
          <option value="" disabled>{groupsLoading ? "正在加载分组..." : groups.length === 0 ? "暂无可用分组" : "请选择分组"}</option>
          {groups.map((group) => <option key={group.id} value={group.id}>{group.name}（{group.platform}）</option>)}
        </select>
        {groupError && <div className="group-feedback">
          <p id="group-error" role="alert">{groupError}</p>
          <button className="quiet-button" type="button" onClick={() => setGroupLoadAttempt((attempt) => attempt + 1)} disabled={busy || groupsLoading}>重新加载分组</button>
        </div>}
      </div>
      {selectedGroupId && <div className="group-field" aria-busy={keysLoading}>
        <div className="field-heading">
          <label htmlFor="api-key">API 密钥</label>
          <button className="quiet-button refresh-keys" type="button" onClick={() => setKeyLoadAttempt((attempt) => attempt + 1)} disabled={busy || keysLoading} aria-label="重新加载密钥">{keyError ? "重新加载" : "刷新"}</button>
        </div>
        <select id="api-key" value={selectedKeyId} onChange={(event) => setSelectedKeyId(event.target.value)} disabled={busy || !keysReady} aria-describedby={keyError ? "key-error" : groupKeys.length === 0 && keysReady ? "key-hint" : undefined}>
          {!keysReady && <option value="" disabled>{keyError ? "密钥加载失败" : "正在加载密钥..."}</option>}
          {keysReady && groupKeys.length === 0 && <option value="create">创建专用密钥</option>}
          {keysReady && groupKeys.map((key) => <option key={key.id} value={key.id}>{key.name || "未命名密钥"}（{key.preview}）</option>)}
        </select>
        {keyError && <div className="group-feedback"><p id="key-error" role="alert">{keyError}</p></div>}
        {keysReady && groupKeys.length === 0 && <p id="key-hint" className="subtle key-hint">当前分组没有可用密钥，启用时将创建专用密钥。</p>}
      </div>}
      <button className="primary-button enable-button" onClick={enable} disabled={busy || groupsLoading || !keysReady || !selectedKeyId} type="button">{busy ? "正在处理..." : "启用 Codex"}</button>
      <p className="subtle configuration-note">启用会备份并替换当前 Codex 配置。</p>
      <button className="secondary-button restore-button" onClick={() => void restoreOriginalCodexConfig()} disabled={busy} type="button">恢复原 Codex 配置</button>
    </section>}
    <footer aria-live="polite" className={error ? "message error" : "message"}>{error || status || "https://codex.ximuai.com"}</footer>
  </main>;
}
