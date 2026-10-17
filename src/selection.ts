export const DEFAULT_MODEL = "gpt-6.1-sol";
export const MODELS = [DEFAULT_MODEL, "gpt-6-sol", "gpt-6-astra", "gpt-5.6-sol", "gpt-5.6-terra"];
export const CLAUDE_DEFAULT_MODEL = "claude-sonnet-5";
export const CLAUDE_MODELS = [CLAUDE_DEFAULT_MODEL, "claude-opus-5-5", "claude-fable-5-1"];
export type ConnectionTarget = "chatgpt" | "claude";

export type Selection = { groupId: string; keyId: string; model: string };

function storageKey(email: string, target: ConnectionTarget) {
  // Preserve the existing ChatGPT storage key when adding a separate Claude profile.
  return `quick-connect.selection.v1:${target === "claude" ? "claude:" : ""}${email.trim().toLowerCase()}`;
}

export function loadSelection(storage: Pick<Storage, "getItem">, email: string, target: ConnectionTarget = "chatgpt"): Selection | null {
  const raw = storage.getItem(storageKey(email, target));
  if (!raw) return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (!value || typeof value !== "object") return null;
    const saved = value as Partial<Selection>;
    return {
      groupId: typeof saved.groupId === "string" && /^[1-9]\d*$/.test(saved.groupId) ? saved.groupId : "",
      keyId: typeof saved.keyId === "string" && (/^[1-9]\d*$/.test(saved.keyId) || saved.keyId === "create") ? saved.keyId : "",
      model: typeof saved.model === "string" && (target === "claude" ? CLAUDE_MODELS.includes(saved.model) : MODELS.includes(saved.model))
        ? saved.model : target === "claude" ? CLAUDE_DEFAULT_MODEL : DEFAULT_MODEL,
    };
  } catch {
    return null;
  }
}

export function saveSelection(storage: Pick<Storage, "setItem">, email: string, selection: Selection, target: ConnectionTarget = "chatgpt") {
  // Store only selection IDs and the model, never credentials or API key secrets.
  storage.setItem(storageKey(email, target), JSON.stringify(selection));
}
