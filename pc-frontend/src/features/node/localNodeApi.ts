/** 本机 Node Admin API - 带 admin token 自动刷新，对应 pc_app_node.js 的 localNodeApi */

const DEFAULT_ADMIN_HEADER = 'X-Elon-Local-Admin-Token'

interface AdminState {
  token: string
  header: string
}

const adminState: AdminState = { token: '', header: DEFAULT_ADMIN_HEADER }

function buildUrl(baseUrl: string, path: string): string {
  const base = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/'
  return new URL(String(path || '').replace(/^\//, ''), base).toString()
}

function isAdminPath(path: string): boolean {
  return String(path || '').replace(/^\/+/, '') !== 'api/status'
}

function applyHeaders(init: RequestInit, needsAdmin: boolean): RequestInit {
  const headers: Record<string, string> = { ...(init.headers as Record<string, string> | undefined) }
  if (init.body && !Object.keys(headers).some((k) => k.toLowerCase() === 'content-type')) {
    headers['Content-Type'] = 'application/json'
  }
  if (needsAdmin && adminState.token) {
    headers[adminState.header || DEFAULT_ADMIN_HEADER] = adminState.token
  }
  return { ...init, headers }
}

function rememberToken(data: Record<string, unknown>): void {
  const token = String(data.local_admin_token ?? '').trim()
  const header = String(data.local_admin_token_header ?? '').trim()
  if (token) adminState.token = token
  if (header) adminState.header = header
}

async function fetchStatus(baseUrl: string, timeoutMs: number): Promise<Record<string, unknown>> {
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), timeoutMs)
  try {
    const res = await fetch(buildUrl(baseUrl, '/api/status'), {
      cache: 'no-store', signal: ctrl.signal,
    })
    const text = await res.text()
    const data = text ? JSON.parse(text) as Record<string, unknown> : {}
    if (!res.ok) throw new Error(String(data.error ?? data.message ?? `HTTP ${res.status}`))
    rememberToken(data)
    return data
  } finally {
    clearTimeout(timer)
  }
}

export async function nodeApi<T = Record<string, unknown>>(
  baseUrl: string,
  path: string,
  options: RequestInit = {},
  timeoutMs = 8000,
): Promise<T> {
  const needsAdmin = isAdminPath(path)
  if (needsAdmin && !adminState.token) {
    await fetchStatus(baseUrl, timeoutMs).catch(() => {})
  }
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), timeoutMs)
  try {
    let init = applyHeaders({ cache: 'no-store', ...options }, needsAdmin)
    init = { ...init, signal: ctrl.signal }
    let res = await fetch(buildUrl(baseUrl, path), init)
    if (needsAdmin && res.status === 403) {
      adminState.token = ''
      await fetchStatus(baseUrl, timeoutMs).catch(() => {})
      init = applyHeaders({ cache: 'no-store', ...options }, needsAdmin)
      res = await fetch(buildUrl(baseUrl, path), { ...init, signal: ctrl.signal })
    }
    const text = await res.text()
    const data = text ? JSON.parse(text) as Record<string, unknown> : {}
    rememberToken(data)
    if (!res.ok) throw new Error(String(data.error ?? data.message ?? `HTTP ${res.status}`))
    return data as T
  } finally {
    clearTimeout(timer)
  }
}

export async function probeLocalNode(baseUrl: string): Promise<Record<string, unknown>> {
  return fetchStatus(baseUrl, 2200)
}
