// Separate from the legacy node API: these responses carry original main-account credentials.
export async function readJson(response: Response): Promise<unknown> {
  if (!response.headers.get('content-type')?.toLowerCase().startsWith('application/json') || !response.body) throw new Error('账号服务返回了无效响应。')
  const reader = response.body.getReader()
  const chunks: Uint8Array[] = []
  let length = 0
  try {
    for (;;) {
      const { done, value } = await reader.read()
      if (done) break
      length += value.length
      if (length > 16_384) throw new Error('账号响应超过允许大小。')
      chunks.push(value)
    }
  } finally { await reader.cancel(); reader.releaseLock() }
  const bytes = new Uint8Array(length)
  let offset = 0
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length }
  return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes))
}

export function loginSession(value: unknown, now: number) {
  const session = value as { token?: unknown; expires_at?: unknown; user?: { id?: unknown; account?: unknown; nickname?: unknown } } | null
  if (!session || typeof session.token !== 'string' || session.token.length < 32 || session.token.length > 512
    || /[^a-zA-Z0-9._-]/.test(session.token) || typeof session.expires_at !== 'string'
    || !Number.isFinite(Date.parse(session.expires_at)) || Date.parse(session.expires_at) <= now
    || typeof session.user?.id !== 'string' || !session.user.id || session.user.id === 'local-owner'
    || typeof session.user.account !== 'string' || !session.user.account
    || (session.user.nickname != null && typeof session.user.nickname !== 'string')) throw new Error('账号服务返回了无效会话。')
  return { token: session.token, expires_at: session.expires_at, user: {
    id: session.user.id, account: session.user.account, nickname: session.user.nickname || undefined,
  } }
}
