/** 向本机 Node Admin API（默认 http://127.0.0.1:7799）发送请求 */

export function buildLocalUrl(baseUrl: string, path: string): string {
  const base = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/'
  return new URL(String(path || '').replace(/^\//, ''), base).toString()
}

export async function localJson<T>(
  baseUrl: string,
  path: string,
  options?: RequestInit,
): Promise<T> {
  const opts: RequestInit = { mode: 'cors', ...options }
  if (opts.body && !opts.headers) {
    opts.headers = { 'Content-Type': 'application/json' }
  }
  const res = await fetch(buildLocalUrl(baseUrl, path), opts)
  const text = await res.text()
  const data = text ? (JSON.parse(text) as Record<string, unknown>) : {}
  if (!res.ok || data.ok === false) {
    const err = new Error(
      String(data.error ?? data.message ?? `HTTP ${res.status}`),
    ) as Error & { data: unknown }
    err.data = data
    throw err
  }
  return data as T
}
