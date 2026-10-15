export const DEFAULT_MODEL = "gpt-6.1-sol";
export const MODELS = [DEFAULT_MODEL, "gpt-6-sol", "gpt-6-astra", "gpt-5.6-sol", "gpt-5.6-terra"];

export type Selection = { groupId: string; keyId: string; model: string };

function storageKey(email: string) {
  return `quick-connect.selection.v1:${email.trim().toLowerCase()}`;
}

export function loadSelection(storage: Pick<Storage, "getItem">, email: string): Selection | null {
  const raw = storage.getItem(storageKey(email));
  if (!raw) return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (!value || typeof value !== "object") return null;
    const saved = value as Partial<Selection>;
    return {
      groupId: typeof saved.groupId === "string" && /^[1-9]\d*$/.test(saved.groupId) ? saved.groupId : "",
      keyId: typeof saved.keyId === "string" && (/^[1-9]\d*$/.test(saved.keyId) || saved.keyId === "create") ? saved.keyId : "",
      model: typeof saved.model === "string" && MODELS.includes(saved.model) ? saved.model : DEFAULT_MODEL,
    };
  } catch {
    return null;
  }
}

export function saveSelection(storage: Pick<Storage, "setItem">, email: string, selection: Selection) {
  // Store only selection IDs and the model, never credentials or API key secrets.
  storage.setItem(storageKey(email), JSON.stringify(selection));
}
