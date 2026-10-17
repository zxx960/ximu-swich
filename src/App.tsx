import { FormEvent, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_MODEL, MODELS, CLAUDE_DEFAULT_MODEL, CLAUDE_MODELS, loadSelection, saveSelection, type Selection, type ConnectionTarget } from "./selection";
import "./App.css";

type Account = { email: string; balance: number };
type AvailableKey = { id: number; name: string; preview: string };
type EnableResult = { balance: number; key_created: boolean; key: AvailableKey; launch_error: string | null };
type ResetLoginResult = { launch_error: string | null };
type ConfigureClaudeResult = { balance: number; key_created: boolean; key: AvailableKey };
type KeyGroup = { id: number; name: string; platform: string };
type SavedCredentials = { email: string; password: string };

function formatBalance(value: number) {
  return new Intl.NumberFormat("zh-CN", { style: "currency", currency: "USD" }).format(value);
}

export default function App() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [account, setAccount] = useState<Account | null>(null);
  const [target, setTarget] = useState<ConnectionTarget>("chatgpt");
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
  const [selectedModel, setSelectedModel] = useState(DEFAULT_MODEL);
  const [groupNotice, setGroupNotice] = useState("");
  const [keyNotice, setKeyNotice] = useState("");
  const [selectionError, setSelectionError] = useState("");
  const savedSelection = useRef<Selection | null>(null);
  const [restoringSession, setRestoringSession] = useState(true);
  const [rememberCredentials, setRememberCredentials] = useState(false);
  const accountEmail = account?.email;
  const isClaude = target === "claude";
  const platform = isClaude ? "anthropic" : "openai";
  const platformLabel = isClaude ? "Claude" : "OpenAI";
  const defaultModel = isClaude ? CLAUDE_DEFAULT_MODEL : DEFAULT_MODEL;
  const keysReady = !!selectedGroupId && loadedKeyGroupId === selectedGroupId && !keysLoading && !keyError;

  function rememberSelection(selection: Selection) {
    if (!accountEmail) return;
    savedSelection.current = selection;
    try {
      saveSelection(window.localStorage, accountEmail, selection, target);
      setSelectionError("");
    } catch {
      setSelectionError("本次选择可正常使用，但无法保存，下次打开时需要重新选择。");
    }
  }

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
    setGroupNotice("");
    setSelectionError("");
    savedSelection.current = null;
    try {
      savedSelection.current = loadSelection(window.localStorage, accountEmail, target);
    } catch {
      setSelectionError("无法读取上次选择，请重新选择分组、密钥和模型。");
    }
    setSelectedModel(savedSelection.current?.model ?? defaultModel);
    setGroupsLoading(true);

    async function loadGroups() {
      try {
        const availableGroups = await invoke<KeyGroup[]>("list_available_groups");
        if (!active) return;
        const matchingGroups = availableGroups.filter((group) => group.platform.trim().toLowerCase() === platform);
        setGroups(matchingGroups);
        if (matchingGroups.length === 0) {
          setGroupError(`当前账户没有可用的 ${platformLabel} 密钥分组，请先在中转站后台配置分组权限。`);
        }
        const saved = savedSelection.current;
        if (saved?.groupId) {
          if (matchingGroups.some((group) => String(group.id) === saved.groupId)) {
            setSelectedGroupId(saved.groupId);
          } else {
            setGroupNotice("上次选择的分组已不可用，请重新选择分组。");
          }
        }
      } catch (reason) {
        if (active) setGroupError(String(reason));
      } finally {
        if (active) setGroupsLoading(false);
      }
    }

    void loadGroups();
    return () => { active = false; };
  }, [accountEmail, groupLoadAttempt, target]);

  useEffect(() => {
    let active = true;
    setGroupKeys([]);
    setSelectedKeyId("");
    setLoadedKeyGroupId("");
    setKeyError("");
    setKeyNotice("");
    setKeysLoading(false);
    if (!accountEmail || !selectedGroupId) return;
    setKeysLoading(true);

    async function loadKeys() {
      try {
        const keys = await invoke<AvailableKey[]>("list_group_keys", { groupId: Number(selectedGroupId) });
        if (!active) return;
        setGroupKeys(keys);
        setLoadedKeyGroupId(selectedGroupId);
        const saved = savedSelection.current;
        const savedKeyId = saved?.groupId === selectedGroupId ? saved.keyId : "";
        const restoredKey = keys.find((key) => String(key.id) === savedKeyId);
        const preferred = restoredKey ?? keys.find((key) => key.name === "西牧ai") ?? keys[0];
        const keyId = preferred ? String(preferred.id) : "create";
        setSelectedKeyId(keyId);
        if (savedKeyId && savedKeyId !== "create" && !restoredKey) {
          setKeyNotice(keys.length ? "上次选择的密钥已不可用，已选择其他可用密钥，请确认。" : `上次选择的密钥已不可用，${isClaude ? "保存配置" : "启动"}时将创建“西牧ai”密钥。`);
        }
        rememberSelection({ groupId: selectedGroupId, keyId, model: saved?.model ?? defaultModel });
      } catch (reason) {
        if (active) setKeyError(String(reason));
      } finally {
        if (active) setKeysLoading(false);
      }
    }

    void loadKeys();
    return () => { active = false; };
  }, [accountEmail, selectedGroupId, keyLoadAttempt, target]);

  function switchTarget(next: ConnectionTarget) {
    if (busy || next === target) return;
    setTarget(next);
    setGroups([]); setSelectedGroupId(""); setGroupKeys([]); setSelectedKeyId(""); setLoadedKeyGroupId("");
    setGroupsLoading(true); setKeysLoading(false); setGroupError(""); setKeyError("");
    setGroupNotice(""); setKeyNotice(""); setSelectionError(""); setStatus(""); setError("");
    setSelectedModel(next === "claude" ? CLAUDE_DEFAULT_MODEL : DEFAULT_MODEL);
    savedSelection.current = null;
  }

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
    const model = selectedModel.trim();
    if (isClaude && !CLAUDE_MODELS.includes(model)) { setError("请选择有效的 Claude 模型。"); return; }
    setBusy(true); setError(""); setStatus(isClaude ? "正在保存 Claude Code 配置..." : "正在保存配置并重新启动 ChatGPT...");
    try {
      const result = isClaude
        ? await invoke<ConfigureClaudeResult>("configure_claude", { groupId, keyId, model })
        : await invoke<EnableResult>("enable_codex", { groupId, keyId, model });
      setAccount((current) => current ? { ...current, balance: result.balance } : current);
      setGroupKeys((current) => [...current.filter((key) => key.id !== result.key.id), result.key]);
      setSelectedKeyId(String(result.key.id));
      setKeyNotice("");
      setSelectedModel(model);
      rememberSelection({ groupId: selectedGroupId, keyId: String(result.key.id), model });
      if (isClaude) {
        setStatus(result.key_created ? "Claude Code 配置已保存，已创建“西牧ai”密钥。请重新打开 CLI 后使用。" : "Claude Code 配置已保存，请重新打开 CLI 后使用。");
      } else if ("launch_error" in result && result.launch_error) {
        setStatus("");
        setError(`配置已保存，但客户端未能按新配置启动：${result.launch_error}`);
      } else {
        setStatus(result.key_created ? "ChatGPT 已启动，已创建“西牧ai”密钥。" : "ChatGPT 已启动，已使用所选密钥。");
      }
    } catch (reason) { setStatus(""); setError(String(reason)); }
    finally { setBusy(false); }
  }

  async function resetChatGPTLogin() {
    if (!window.confirm("将关闭并重新打开 ChatGPT，清除客户端当前登录信息并恢复默认连接设置。客户端中正在进行的任务会中断，共用该登录缓存的 CLI/IDE 后续也需要重新登录。是否继续？")) return;
    setBusy(true); setError(""); setStatus("正在重置 ChatGPT 登录状态...");
    try {
      const result = await invoke<ResetLoginResult>("reset_chatgpt_login");
      if (result.launch_error) {
        setStatus("");
        setError(`登录状态已重置，请手动打开 ChatGPT 重新登录：${result.launch_error}`);
      } else {
        setStatus("ChatGPT 登录状态已重置，客户端已重新打开，请重新登录。");
      }
    } catch (reason) { setStatus(""); setError(String(reason)); }
    finally { setBusy(false); }
  }

  async function signOut() { await invoke("logout"); setAccount(null); setPassword(""); setStatus(""); setError(""); setGroups([]); setSelectedGroupId(""); setGroupError(""); setGroupNotice(""); setGroupsLoading(false); setGroupKeys([]); setSelectedKeyId(""); setLoadedKeyGroupId(""); setKeyError(""); setKeyNotice(""); setSelectionError(""); savedSelection.current = null; setKeysLoading(false); setSelectedModel(DEFAULT_MODEL); }

  return <main className="app-shell">
    <header className="topbar"><div className="brand"><img className="brand-mark" src="/logo.jpg" alt="" /><span>西牧ai</span></div>{account && <button className="quiet-button" type="button" onClick={signOut} aria-label="退出登录">退出</button>}</header>
    {restoringSession ? <section className="content login-view"><div className="eyebrow">XIMUAI CODEX</div><h1>正在恢复登录...</h1><p className="subtle">正在验证本机加密会话。</p></section> : !account ? <section className="content login-view" aria-labelledby="login-title"><div className="eyebrow">XIMUAI CODEX</div><h1 id="login-title">连接你的 Codex</h1><p className="subtle">登录后选择密钥分组，一键配置本机 Codex。</p><form onSubmit={signIn} className="form-stack"><label>邮箱<input type="email" autoComplete="username" value={email} onChange={(e) => setEmail(e.target.value)} disabled={busy} /></label><label>密码<input type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} disabled={busy} /></label><label className="remember-option"><input type="checkbox" checked={rememberCredentials} onChange={(event) => void toggleRememberCredentials(event.target.checked)} disabled={busy} />保存账号和密码</label><button className="primary-button" disabled={busy} type="submit">{busy ? "正在连接..." : "登录"}</button></form></section> : <section className="content connected-view" aria-labelledby="account-title">
      <div className="connection-chip">已连接</div>
      <h1 id="account-title">{account.email}</h1>
      <div className="balance-panel"><span>API 账户余额</span><strong>{formatBalance(account.balance)}</strong></div>
      <div className="client-tabs" role="tablist" aria-label="客户端配置" onKeyDown={(event) => {
        if (busy || !["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
        event.preventDefault();
        const next = event.key === "Home" ? "chatgpt" : event.key === "End" ? "claude" : target === "chatgpt" ? "claude" : "chatgpt";
        switchTarget(next);
        document.getElementById(`tab-${next}`)?.focus();
      }}>
        <button id="tab-chatgpt" role="tab" type="button" aria-selected={!isClaude} aria-controls="connection-panel" tabIndex={isClaude ? -1 : 0} disabled={busy} onClick={() => switchTarget("chatgpt")}>ChatGPT</button>
        <button id="tab-claude" role="tab" type="button" aria-selected={isClaude} aria-controls="connection-panel" tabIndex={isClaude ? 0 : -1} disabled={busy} onClick={() => switchTarget("claude")}>Claude Code</button>
      </div>
      <div id="connection-panel" role="tabpanel" aria-labelledby={`tab-${target}`}>
      <div className="group-field" aria-busy={groupsLoading}>
        <label htmlFor="key-group">密钥分组</label>
        <select
          id="key-group"
          value={selectedGroupId}
          onChange={(event) => { const groupId = event.target.value; setSelectedGroupId(groupId); setSelectedKeyId(""); setGroupNotice(""); setError(""); setStatus(""); rememberSelection({ groupId, keyId: "", model: selectedModel }); }}
          disabled={busy || groupsLoading || groups.length === 0}
          aria-describedby={groupError ? "group-error" : undefined}
        >
          <option value="" disabled>{groupsLoading ? "正在加载分组..." : groups.length === 0 ? `暂无可用 ${platformLabel} 分组` : "请选择分组"}</option>
          {groups.map((group) => <option key={group.id} value={group.id}>{group.name}（{group.platform}）</option>)}
        </select>
        {groupNotice && <p className="subtle key-hint" role="status">{groupNotice}</p>}
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
        <select id="api-key" value={selectedKeyId} onChange={(event) => { const keyId = event.target.value; setSelectedKeyId(keyId); setKeyNotice(""); rememberSelection({ groupId: selectedGroupId, keyId, model: selectedModel }); }} disabled={busy || !keysReady} aria-describedby={keyError ? "key-error" : groupKeys.length === 0 && keysReady ? "key-hint" : undefined}>
          {!keysReady && <option value="" disabled>{keyError ? "密钥加载失败" : "正在加载密钥..."}</option>}
          {keysReady && groupKeys.length === 0 && <option value="create">创建“西牧ai”密钥</option>}
          {keysReady && groupKeys.map((key) => <option key={key.id} value={key.id}>{key.name || "未命名密钥"}（{key.preview}）</option>)}
        </select>
        {keyError && <div className="group-feedback"><p id="key-error" role="alert">{keyError}</p></div>}
        {keyNotice && <p className="subtle key-hint" role="status">{keyNotice}</p>}
        {keysReady && groupKeys.length === 0 && <p id="key-hint" className="subtle key-hint">当前分组没有可用密钥，{isClaude ? "保存配置" : "启动"}时将创建名为“西牧ai”的密钥。</p>}
      </div>}
      {selectedGroupId && <div className="group-field">
        <label htmlFor="codex-model">模型</label>
        <select id="codex-model" value={selectedModel} onChange={(event) => { const model = event.target.value; setSelectedModel(model); rememberSelection({ groupId: selectedGroupId, keyId: selectedKeyId || (savedSelection.current?.groupId === selectedGroupId ? savedSelection.current.keyId : ""), model }); }} disabled={busy}>
          {(isClaude ? CLAUDE_MODELS : MODELS).map((model) => <option key={model} value={model}>{model}</option>)}
        </select>
      </div>}
      {selectionError && <p className="subtle key-hint" role="alert">{selectionError}</p>}
      <button className="primary-button enable-button" onClick={enable} disabled={busy || groupsLoading || !keysReady || !selectedKeyId || !selectedModel.trim()} type="button">{busy ? "正在处理..." : isClaude ? "保存 Claude Code 配置" : "启动ChatGPT"}</button>
      <p className="subtle configuration-note">{isClaude ? "保存到 Claude Code 用户配置，重新打开 CLI 后生效。" : "保存配置后会重新启动 ChatGPT，正在进行的任务会中断。"}</p>
      {!isClaude && <button className="secondary-button restore-button" onClick={() => void resetChatGPTLogin()} disabled={busy} type="button">恢复原 ChatGPT 配置</button>}
      </div>
    </section>}
    <footer aria-live="polite" className={error ? "message error" : "message"}>{error || status || "https://codex.ximuai.com"}</footer>
  </main>;
}
