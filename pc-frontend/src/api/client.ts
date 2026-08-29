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

function isApiError(error: unknown): error is ApiError {
  return Boolean(error && typeof error === 'object'
    && typeof (error as ApiError).status === 'number'
    && typeof (error as ApiError).message === 'string')
}

function normalizeTransportError(error: unknown): ApiError {
  if (isApiError(error)) return error
  if (error instanceof Error && !(error instanceof TypeError) && error.message) {
    return { status: 0, message: error.message }
  }
  return {
    status: 0,
    message: '云端连接中断，消息已保留；请检查网络后重试。',
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getAuthToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(init?.headers as Record<string, string> | undefined),
  }
  let res: Response
  try {
    res = await fetch(resolveApiUrl(path), { ...init, headers })
  } catch (error) {
    throw normalizeTransportError(error)
  }
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
  try {
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
    let terminalEventSeen = false
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
            let event: ApiStreamEvent
            try {
              event = JSON.parse(raw) as ApiStreamEvent
            } catch {
              // Ignore malformed/non-JSON SSE lines and continue the stream.
              continue
            }
            if (event.type === 'done' || event.type === 'error') terminalEventSeen = true
            // Keep callback failures visible to the caller. In particular, an
            // application-level SSE error must not be mistaken for a healthy
            // completed stream and silently swallow the retry path.
            onEvent(event)
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
      if (!terminalEventSeen) {
        throw {
          status: 0,
          message: '云端连接在回答完成前中断，消息已保留；请检查网络后重试。',
        } satisfies ApiError
      }
    } finally {
      reader.releaseLock()
    }
  } catch (error) {
    throw normalizeTransportError(error)
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
