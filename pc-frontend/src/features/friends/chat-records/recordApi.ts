import { getAuthToken } from '../../../api/client'
import { resolveApiUrl } from '../../../api/runtime'

export const RECORD_PREFIX = '【一龙聊天记录】\n'
export interface RecordCard { schema: 'chat_record_bundle_v1'; record_id: string; group_id: string; title: string; summary: string; message_count: number; total_count: number }
export interface RecordRow { id: string; parent_id: string | null; sender: string; time: string; kind: string; text: string; filename: string; asset_id: string | null }
export interface RecordView { card: RecordCard; owner_id: string; document: { title: string; raw_text: string; warnings: string[]; messages: RecordRow[] } }
export function recordCard(content: string): RecordCard | null {
  if (!content.startsWith(RECORD_PREFIX)) return null
  try {
    const r = JSON.parse(content.slice(RECORD_PREFIX.length))
    return r.schema === 'chat_record_bundle_v1' && typeof r.record_id === 'string' && typeof r.group_id === 'string'
      && /^[\w-]{1,128}$/.test(r.record_id) && /^[\w-]{1,128}$/.test(r.group_id)
      && typeof r.title === 'string' && typeof r.summary === 'string' && Number.isSafeInteger(r.message_count) && r.message_count > 0 && r.message_count <= 2000 ? r : null
  } catch { return null }
}
export function recordPath(card: RecordCard): string { return `/api/me/groups/${encodeURIComponent(card.group_id)}/chat-records/${encodeURIComponent(card.record_id)}` }
export async function recordRequest(path: string, signal: AbortSignal, binary = false, method = 'GET'): Promise<unknown> {
  const token = getAuthToken()
  if (!token) throw Error('请先登录')
  // Revalidate permissions on each open; unchanged private bodies stay in the HTTP cache.
  const res = await fetch(resolveApiUrl(path), { method, signal, cache: method === 'GET' ? 'no-cache' : 'no-store', headers: { Authorization: `Bearer ${token}` } })
  if (!res.ok) throw Error([403, 404].includes(res.status) ? '记录已撤回，或你已不在此群聊中' : `读取失败（${res.status}），请重试`)
  if (Number(res.headers.get('content-length')) > 12 * 1024 * 1024) throw Error('附件过大')
  const value = binary ? await res.blob() : await res.json()
  if (getAuthToken() !== token || signal.aborted) throw Error('账号已变化或记录已关闭')
  return value
}
