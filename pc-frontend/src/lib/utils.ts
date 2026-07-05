/** 纯工具函数，对应旧 pc_app_utils.js 的 window.ElonPcKit */
import { localNodeBaseUrl } from '../api/runtime'

export function clean(value: unknown): string {
  return String(value == null ? '' : value).trim()
}

export function firstChar(value: unknown, fallback = '龙'): string {
  return Array.from(clean(value) || fallback)[0] || '龙'
}

export function formatTime(value: unknown): string {
  if (!value) return ''
  const date = new Date(Number(value) || String(value))
  if (Number.isNaN(date.getTime())) return String(value).slice(0, 16)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

/** 验证 node_admin URL：只允许 127.0.0.1 / localhost，默认 http://127.0.0.1:7799/ */
export function safeNodeAdminUrl(rawParam?: string | null): string {
  const raw = rawParam ?? new URLSearchParams(location.search).get('node_admin') ?? ''
  if (raw.trim()) {
    try {
      const url = new URL(raw)
      const host = url.hostname.toLowerCase()
      if ((host === '127.0.0.1' || host === 'localhost' || host === '::1') && /^https?:$/.test(url.protocol)) {
        return url.toString()
      }
    } catch {
      // invalid URL
    }
  }
  return `${localNodeBaseUrl()}/`
}
