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
  const now = new Date()
  const time = `${pad2(date.getHours())}:${pad2(date.getMinutes())}`
  if (sameLocalDay(date, now)) return `今天 ${time}`

  const yesterday = new Date(now)
  yesterday.setDate(now.getDate() - 1)
  if (sameLocalDay(date, yesterday)) return `昨天 ${time}`

  const monthDay = `${date.getMonth() + 1}月${date.getDate()}日`
  if (date.getFullYear() === now.getFullYear()) return `${monthDay} ${time}`
  return `${date.getFullYear()}年${monthDay} ${time}`
}

function sameLocalDay(left: Date, right: Date): boolean {
  return left.getFullYear() === right.getFullYear()
    && left.getMonth() === right.getMonth()
    && left.getDate() === right.getDate()
}

function pad2(value: number): string {
  return String(value).padStart(2, '0')
}

/** 验证 node_admin URL：只允许回环地址；未指定时使用已探测到的本机节点。 */
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
