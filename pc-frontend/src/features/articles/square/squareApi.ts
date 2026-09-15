import { getAuthToken } from '../../../api/client'
import { cloudBaseUrl, isLocalWorkbench } from '../../../api/runtime'
export type Mode = 'article' | 'text' | 'images' | 'video'
export interface Account { bound: boolean; label: string; masked_key: string; generation: number; verified_at: number | null }
export interface Selection { article_id: string; version: number; mode: Mode; media_ids: string[]; cover_id: string | null; video_id: string | null }
export interface Preview { selection: Selection; title: string; text: string; warnings: string[]; preview_hash: string; generation: number; account_label: string; media: Record<string, string> }
export interface Job { id: string; article_id: string; revision: number; title: string; mode: Mode; status: string; scheduled_at: number; created_at: number; updated_at: number; attempts: number; message: string; post_url: string | null }
export const creatorUrl = 'https://www.binance.com/square/creator-center/home'
export const labels: Record<string, string> = { queued: '等待发布', preparing: '准备媒体', submitting: '提交中', published: '已发布', failed: '发布失败', uncertain: '结果待核实', cancelled: '已取消' }
export const modes: Record<Mode, string> = { article: '长文章', text: '文字动态', images: '图片动态', video: '视频动态' }
export function secureBase(): string {
  const url = new URL(isLocalWorkbench() ? cloudBaseUrl() : location.origin)
  if (url.hostname === '43.139.149.158' && (url.port === '8080' || url.port === '8443')) return 'https://43.139.149.158:8443'
  if (url.protocol !== 'https:') throw Error('当前服务器未配置受信任 HTTPS，暂不能绑定或发布。请联系管理员。')
  return url.origin
}
export async function square<T>(path: string, method = 'GET', body?: unknown): Promise<T> {
  const token = getAuthToken(); if (!token) throw Error('请先登录一龙账号')
  const controller = new AbortController(); const timer = window.setTimeout(() => controller.abort(), body instanceof FormData ? 120000 : 45000)
  try {
    const multipart = body instanceof FormData
    const res = await fetch(`${secureBase()}/api/me/article-channels/binance-square${path}`, { method, credentials: 'omit', redirect: 'error', signal: controller.signal, headers: { Authorization: `Bearer ${token}`, ...(!multipart ? { 'Content-Type': 'application/json' } : {}) }, ...(body === undefined ? {} : { body: multipart ? body : JSON.stringify(body) }) })
    const data = await res.json().catch(() => null)
    if (!res.ok) throw Error(data?.error || `请求失败（${res.status}）`)
    if (!data) throw Error('响应不完整，请刷新发布记录核实')
    return data as T
  } catch (e) {
    if (e instanceof TypeError || (e instanceof Error && e.name === 'AbortError')) throw Error('安全连接未完成。若刚才正在发布，请先刷新发布记录核实。')
    throw e
  } finally { clearTimeout(timer) }
}
export const stamp = (epoch: number) => new Date(epoch * 1000).toLocaleString()
export function requestId(): string { return Array.from(crypto.getRandomValues(new Uint8Array(24)), b => b.toString(16).padStart(2, '0')).join('') }
export function postIdFromLink(value: string): string {
  if (/^\d{1,64}$/.test(value.trim())) return value.trim()
  try { const u = new URL(value.trim()); if (u.protocol === 'https:' && u.hostname === 'www.binance.com') { const id = u.pathname.match(/^\/(?:[\w-]+\/)?square\/post\/(\d{1,64})\/?$/)?.[1]; if (id) return id } } catch { /* validate below */ }
  throw Error('请粘贴币安帖子链接或数字帖子 ID')
}
