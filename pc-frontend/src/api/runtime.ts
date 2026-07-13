export interface PcWorkbenchBootstrap {
  mode?: 'cloud' | 'local'
  cloudBaseUrl?: string
  localNodeBaseUrl?: string
}

declare global {
  interface Window {
    __ELON_PC_BOOTSTRAP__?: PcWorkbenchBootstrap
  }
}

const DEFAULT_CLOUD_BASE_URL = 'http://43.139.149.158:8080'
const DEFAULT_LOCAL_NODE_BASE_URL = 'http://127.0.0.1:7799'
const LOCAL_NODE_BASE_STORAGE_KEY = 'elon_local_node_base_url'
const LOCAL_NODE_FIRST_PORT = 7799
const LOCAL_NODE_FALLBACK_LIMIT = 20
export const LOCAL_NODE_BASE_CHANGED_EVENT = 'elon:local-node-base-changed'

let validatedLocalNodeBaseUrl = ''

function bootstrap(): PcWorkbenchBootstrap {
  return window.__ELON_PC_BOOTSTRAP__ ?? {}
}

function trimTrailingSlash(value: string): string {
  return value.replace(/\/+$/, '')
}

function isLoopbackHost(hostname: string): boolean {
  const host = hostname.toLowerCase()
  return host === '127.0.0.1' || host === 'localhost' || host === '[::1]' || host === '::1'
}

function safeLoopbackUrl(raw: string | null | undefined): string {
  try {
    const url = new URL(raw || '')
    if (isLoopbackHost(url.hostname) && /^https?:$/.test(url.protocol)) {
      return trimTrailingSlash(url.toString())
    }
  } catch {
    // invalid URL
  }
  return ''
}

function rememberedLocalNodeBaseUrl(): string {
  try {
    return safeLoopbackUrl(localStorage.getItem(LOCAL_NODE_BASE_STORAGE_KEY))
  } catch {
    return ''
  }
}

export function rememberLocalNodeBaseUrl(raw: string): string {
  const safe = safeLoopbackUrl(raw)
  if (!safe) return ''
  const changed = validatedLocalNodeBaseUrl !== safe
  validatedLocalNodeBaseUrl = safe
  try {
    localStorage.setItem(LOCAL_NODE_BASE_STORAGE_KEY, safe)
  } catch {
    // Storage can be disabled; the current page can still use the discovered URL.
  }
  if (changed) window.dispatchEvent(new CustomEvent(LOCAL_NODE_BASE_CHANGED_EVENT, {
    detail: { baseUrl: safe },
  }))
  return safe
}

export function isLocalWorkbench(): boolean {
  const boot = bootstrap()
  if (boot.mode === 'local') return true
  return isLoopbackHost(location.hostname) && location.port === '7799'
}

export function cloudBaseUrl(): string {
  const base = bootstrap().cloudBaseUrl?.trim() || DEFAULT_CLOUD_BASE_URL
  return trimTrailingSlash(base)
}

export function localNodeBaseUrl(): string {
  if (validatedLocalNodeBaseUrl) return validatedLocalNodeBaseUrl
  const boot = safeLoopbackUrl(bootstrap().localNodeBaseUrl)
  if (boot) return boot
  const fromQuery = safeLoopbackUrl(new URLSearchParams(location.search).get('node_admin'))
  if (fromQuery) return fromQuery
  const base = isLoopbackHost(location.hostname) ? location.origin : DEFAULT_LOCAL_NODE_BASE_URL
  if (isLoopbackHost(location.hostname)) return trimTrailingSlash(base)
  return rememberedLocalNodeBaseUrl() || trimTrailingSlash(base)
}

export function localNodeProbeBaseUrls(): string[] {
  const fallbackBases = Array.from(
    { length: LOCAL_NODE_FALLBACK_LIMIT + 1 },
    (_, offset) => `http://127.0.0.1:${LOCAL_NODE_FIRST_PORT + offset}`,
  )
  return Array.from(new Set(
    [localNodeBaseUrl(), 'http://localhost:7799', ...fallbackBases]
      .map(trimTrailingSlash)
      .filter(Boolean),
  ))
}

export function resolveApiUrl(path: string): string {
  if (/^https?:\/\//i.test(path)) return path
  if (!isLocalWorkbench()) return path
  if (!path.startsWith('/api/') && path !== '/api') return path
  return new URL(path, cloudBaseUrl()).toString()
}

export function cloudWebSocketUrl(path: string): string {
  const base = isLocalWorkbench() ? cloudBaseUrl() : location.href
  const url = new URL(path, base)
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  return url.toString()
}

export function localNodeUrl(path: string): string {
  return new URL(path.replace(/^\//, ''), `${localNodeBaseUrl()}/`).toString()
}

export function cloudWorkbenchUrl(
  pathname = location.pathname,
  search = location.search,
  hash = location.hash,
): string {
  const url = new URL(normalizePcPath(pathname), `${cloudBaseUrl()}/`)
  const params = new URLSearchParams(search)
  params.delete('node_admin')
  params.forEach((value, key) => {
    url.searchParams.append(key, value)
  })
  url.searchParams.set('node_admin', new URL('/', `${localNodeBaseUrl()}/`).toString())
  url.hash = hash
  return url.toString()
}

function normalizePcPath(pathname: string): string {
  if (pathname === '/pc-next') return '/pc'
  if (pathname.startsWith('/pc-next/')) return `/pc${pathname.slice('/pc-next'.length)}`
  if (pathname === '/pc' || pathname.startsWith('/pc/')) return pathname
  return '/pc'
}
