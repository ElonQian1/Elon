const record = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === 'object' && !Array.isArray(value)

/** Validate attachment pages before the desktop host puts them on its local queue. */
export function validConversationPage(page: unknown, input: Record<string, unknown>): boolean {
  if (!record(page) || page.conversation_id !== input.conversation_id) return false
  const cursor = typeof input.cursor === 'string' ? input.cursor : ''
  if (!cursor.startsWith('asset.')) {
    return page.schema === 'yilong.web-conversation.snapshot.v1' && Array.isArray(page.blocks)
  }
  const match = /^asset\.([a-f0-9]{32})\.(\d{1,6})\.(\d{1,8})$/.exec(cursor)
  if (!match || page.schema !== 'yilong.web-conversation.asset.v1' ||
      page.revision !== match[1] || page.asset_index !== Number(match[2]) || page.byte_offset !== Number(match[3]) ||
      !Number.isSafeInteger(page.total_bytes) || (page.total_bytes as number) < 0 || (page.total_bytes as number) > 8 * 1024 * 1024 ||
      typeof page.data !== 'string' || page.data.length > 32768 ||
      !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(page.data) ||
      typeof page.name !== 'string' || page.name.length > 180 || typeof page.media_type !== 'string' ||
      !/^[a-z0-9!#$&^_.+-]+\/[a-z0-9!#$&^_.+-]+$/i.test(page.media_type)) return false
  const start = Number(match[3]), total = page.total_bytes as number
  const padding = page.data.endsWith('==') ? 2 : page.data.endsWith('=') ? 1 : 0
  const bytes = page.data.length / 4 * 3 - padding, next = start + bytes
  return start % 24576 === 0 && start <= total && bytes === Math.min(24576, total - start) &&
    page.has_more === (next < total) &&
    page.next_cursor === (next < total ? `asset.${match[1]}.${match[2]}.${next}` : null)
}
