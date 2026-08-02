/** 对 /api/* 的最小封装：自动带 token、统一错误格式 */

import { resolveApiUrl } from './runtime'

export function getAuthToken(): string | null {
  try {
    const raw = localStorage.getItem('elon_auth')
    if (!raw) return null
    const parsed = JSON.parse(raw) as { token?: string; state?: { token?: string } }
    return parsed.token ?? parsed.state?.token ?? null
  } catch {
    return null
  }
}

export function getAuthIdentityLabel(): string | null {
  try {
    const raw = localStorage.getItem('elon_auth')
    if (!raw) return null
    const parsed = JSON.parse(raw) as {
      user?: { nickname?: string; account?: string }
      state?: { user?: { nickname?: string; account?: string } }
    }
    const user = parsed.user ?? parsed.state?.user
    return user?.nickname?.trim() || user?.account?.trim() || null
  } catch {
    return null
  }
}

export interface ApiError {
  status: number
  message: string
}

export interface ApiStreamEvent {
  type: string
  [key: string]: unknown
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getAuthToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(init?.headers as Record<string, string> | undefined),
  }
  const res = await fetch(resolveApiUrl(path), { ...init, headers })
  if (!res.ok) {
    let message = res.statusText
    try {
      const body = await res.json()
      if (typeof body?.error === 'string') message = body.error
      else if (typeof body?.message === 'string') message = body.message
    } catch {
      // ignore parse error
    }
    throw { status: res.status, message } satisfies ApiError
  }
  // 204 No Content
  if (res.status === 204) return undefined as unknown as T
  return res.json() as Promise<T>
}

export async function streamPost(
  path: string,
  body: unknown,
  onEvent: (event: ApiStreamEvent) => void,
): Promise<void> {
  const token = getAuthToken()
  const res = await fetch(resolveApiUrl(path), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'text/event-stream',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body),
  })
  if (!res.ok) {
    let message = res.statusText
    try {
      const data = await res.json()
      if (typeof data?.error === 'string') message = data.error
      else if (typeof data?.message === 'string') message = data.message
    } catch {
      // ignore parse error
    }
    throw { status: res.status, message } satisfies ApiError
  }
  if (!res.body) throw { status: 0, message: '服务器没有返回流式响应' } satisfies ApiError

  const reader = res.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''
  try {
    while (true) {
      const chunk = await reader.read()
      buffer += decoder.decode(chunk.value ?? new Uint8Array(), { stream: !chunk.done })
      let separator = buffer.indexOf('\r\n\r\n')
      let separatorLength = 4
      if (separator < 0) {
        separator = buffer.indexOf('\n\n')
        separatorLength = 2
      }
      while (separator >= 0) {
        const block = buffer.slice(0, separator)
        buffer = buffer.slice(separator + separatorLength)
        for (const line of block.split(/\r?\n/)) {
          if (!line.startsWith('data:')) continue
          const raw = line.slice(5).trim()
          if (!raw || raw === '[DONE]') continue
          try {
            const event = JSON.parse(raw) as ApiStreamEvent
            onEvent(event)
          } catch {
            // Ignore incomplete/non-JSON SSE lines and continue the stream.
          }
        }
        separator = buffer.indexOf('\r\n\r\n')
        separatorLength = 4
        if (separator < 0) {
          separator = buffer.indexOf('\n\n')
          separatorLength = 2
        }
      }
      if (chunk.done) break
    }
  } finally {
    reader.releaseLock()
  }
}

export const api = {
  get: <T>(path: string) => request<T>(path),
  getWithHeaders: <T>(path: string, headers: Record<string, string>) =>
    request<T>(path, { headers }),
  post: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'POST', body: JSON.stringify(body) }),
  postWithHeaders: <T>(path: string, body: unknown, headers: Record<string, string>) =>
    request<T>(path, { method: 'POST', headers, body: JSON.stringify(body) }),
  patch: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'PATCH', body: JSON.stringify(body) }),
  put: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'PUT', body: JSON.stringify(body) }),
  delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
  streamPost,
}
