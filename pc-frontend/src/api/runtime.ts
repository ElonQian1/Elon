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
  const boot = safeLoopbackUrl(bootstrap().localNodeBaseUrl)
  if (boot) return boot
  const fromQuery = safeLoopbackUrl(new URLSearchParams(location.search).get('node_admin'))
  if (fromQuery) return fromQuery
  const base = isLoopbackHost(location.hostname) ? location.origin : DEFAULT_LOCAL_NODE_BASE_URL
  return trimTrailingSlash(base)
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
