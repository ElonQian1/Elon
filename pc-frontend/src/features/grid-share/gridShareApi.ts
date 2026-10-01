import { getAuthToken } from '../../api/client'
import { resolveApiUrl } from '../../api/runtime'
import type { SocialMessage } from '../friends/socialMessageTypes'
import type { GridAttachment, GridSource } from '../grid-chat/gridChatSnapshot'
import { gridCard, shareDocument, type GridShare, type GridShareView } from './gridShareModel'
import { gridShareDigest } from './gridShareIdentity'

export const sharePath = (group: string, id = '') => {
  if (![group, ...(id ? [id] : [])].every(v => /^[\w-]{1,160}$/.test(v))) throw Error('分享地址无效')
  return `/api/me/groups/${group}/ai-snapshots${id ? `/${id}` : ''}`
}
export async function shareRequest<T>(path: string, signal: AbortSignal, body?: unknown, method?: string): Promise<T> {
  const token = getAuthToken()
  if (!token) throw Error('请先登录一龙账号')
  const timed = AbortSignal.any([signal, AbortSignal.timeout(25_000)])
  const response = await fetch(resolveApiUrl(path), { method: method ?? (body ? 'POST' : 'GET'), signal: timed, cache: 'no-store', redirect: 'error',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined })
  if (!response.ok) throw Error(({ 401: '登录已变化，请重新打开', 403: '你已不在此群或无权更新', 404: '分享已撤回或不存在', 409: '分享已有更新，请打开最新版本', 400: '快照已过期或数据不完整，请重新读取' } as Record<number, string>)[response.status] ?? '请求未确认，请重试；同一快照不会重复发送')
  const result = await response.json()
  if (signal.aborted || getAuthToken() !== token) throw Error('账号已变化或窗口已关闭')
  return result as T
}
export function readShare(group: string, id: string, signal: AbortSignal) {
  return shareRequest<GridShareView>(sharePath(group, id), signal).then(view => {
    if (view.group_id !== group || view.snapshot_id !== id || view.document?.grid?.schema !== 'yilong.grid_share.v1') throw Error('分享格式不受支持，请更新客户端')
    return view
  })
}
export async function publishShare(group: string, grid: GridShare, signal: AbortSignal) {
  const document = shareDocument(grid)
  const key = gridShareDigest(document)
  const result = await shareRequest<{ snapshot_id: string; message: SocialMessage }>(sharePath(group), signal, { idempotency_key: key, document })
  const card = gridCard(result.message?.content ?? '')
  if (!card || card.snapshot_id !== result.snapshot_id || card.group_id !== group) throw Error('发送回执不匹配，请重试确认')
  return result
}
// Private local binding is never part of a message or API request. No token or cookie is stored.
interface Binding { strategy: string; account: string; kind: string }
const bindingKey = (owner: string, group: string, id: string) => `grid-share-binding:${resolveApiUrl('/')}:${owner}:${group}:${id}`
export function rememberShare(owner: string, group: string, id: string, attachment: GridAttachment) {
  try { localStorage.setItem(bindingKey(owner, group, id), JSON.stringify({ strategy: attachment.facts.id, account: attachment.source.account, kind: attachment.source.accountKind })) } catch { /* Sharing still succeeded; another update requires this local binding. */ }
}
export function readBinding(owner: string, group: string, id: string): Binding | null {
  try { return JSON.parse(localStorage.getItem(bindingKey(owner, group, id)) ?? 'null') } catch { return null }
}
export function matchesBinding(binding: Binding, source: GridSource) { return binding.account === source.account && binding.kind === source.accountKind }
