import type { AuthState, HistoryItem, ReelResponse, RepositoryDetail, SavedRepository, SettingsSummary } from "../types";

// API 呼び出しをここに集約し、画面側はレスポンス型と操作名だけを意識する。
async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
    ...init
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.status}`);
  }

  return response.json() as Promise<T>;
}

export const api = {
  authState: () => request<AuthState>("/api/auth/state"),
  current: () => request<ReelResponse>("/api/reel/current"),
  next: () => request<ReelResponse>("/api/reel/next", { method: "POST" }),
  previous: () => request<ReelResponse>("/api/reel/previous", { method: "POST" }),
  save: (id: number) => request<{ ok: boolean }>(`/api/reel/${id}/save`, { method: "POST" }),
  skip: (id: number) => request<{ ok: boolean }>(`/api/reel/${id}/skip`, { method: "POST" }),
  detail: (id: number) => request<RepositoryDetail>(`/api/reel/${id}/detail`),
  saved: (query = "") => request<SavedRepository[]>(`/api/saved?query=${encodeURIComponent(query)}`),
  note: (id: number, body: string) =>
    request<{ ok: boolean }>(`/api/saved/${id}/note`, { method: "PATCH", body: JSON.stringify({ body }) }),
  tags: (id: number, tags: string[]) =>
    request<{ ok: boolean }>(`/api/saved/${id}/tags`, { method: "PUT", body: JSON.stringify({ tags }) }),
  history: () => request<HistoryItem[]>("/api/history"),
  settings: () => request<SettingsSummary>("/api/settings")
};
