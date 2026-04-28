import { invoke } from "@tauri-apps/api/core";

let baseUrl: string | null = null;

export async function getBaseUrl(): Promise<string> {
  if (!baseUrl) {
    const port = await invoke<number>("get_backend_port");
    baseUrl = `http://127.0.0.1:${port}`;
  }
  return baseUrl;
}

export async function apiFetch(path: string, init?: RequestInit): Promise<Response> {
  const url = await getBaseUrl();
  return fetch(`${url}${path}`, init);
}
