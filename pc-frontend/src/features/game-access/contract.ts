export interface GameRequest {
  schema: 'esk.game.access.authorize.v1'
  client_id: 'esk-game.web'
  redirect_uri: string
  state: string
  code_challenge: string
  code_challenge_method: 'S256'
  scopes: string[]
  expires_in_seconds: number
}
const fields = ['schema', 'client_id', 'redirect_uri', 'state', 'code_challenge', 'code_challenge_method', 'scopes', 'expires_in_seconds']
export const scopeLabels: Record<string, string> = {
  play: '使用主账号进入游戏，并恢复角色和小岛',
  inventory_read: '读取本人游戏装备和回收订单',
  wallet_bind: '在游戏中确认并签名绑定自己的装备钱包',
  redeem: '申请装备回收报价并接受报价',
  principal_withdraw: '提交本金退出申请',
}
const scopeOrder = Object.keys(scopeLabels)
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('授权参数无效，请回到游戏重新发起登录。')
  return value as Record<string, unknown>
}
function safeString(value: unknown, min: number, max: number): value is string {
  return typeof value === 'string' && value.length >= min && value.length <= max && !/[^A-Za-z0-9._~-]/.test(value)
}
export function parseGameRequest(search: string): GameRequest {
  if (search.length > 8192) throw new Error('授权链接过长，请回到游戏重试。')
  const query = new URLSearchParams(search)
  if ([...query.keys()].length !== 1 || !query.has('request')) throw new Error('授权链接无效，请回到游戏重试。')
  const raw = query.get('request')!
  if (raw.length > 4096) throw new Error('授权参数过长。')
  const v = record(JSON.parse(raw))
  if (Object.keys(v).length !== fields.length || Object.keys(v).some(k => !fields.includes(k))
    || v.schema !== 'esk.game.access.authorize.v1' || v.client_id !== 'esk-game.web'
    || v.code_challenge_method !== 'S256' || !safeString(v.state, 43, 128)
    || typeof v.code_challenge !== 'string' || v.code_challenge.length !== 43
    || /[^A-Za-z0-9_-]/.test(v.code_challenge) || !'AEIMQUYcgkosw048'.includes(v.code_challenge[42])
    || typeof v.expires_in_seconds !== 'number' || !Number.isInteger(v.expires_in_seconds)
    || v.expires_in_seconds < 1 || v.expires_in_seconds > 900 || typeof v.redirect_uri !== 'string') {
    throw new Error('游戏授权参数不受支持，请重新发起登录。')
  }
  const url = new URL(v.redirect_uri)
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash
    || url.pathname !== '/api/account/callback' || url.href !== v.redirect_uri) throw new Error('游戏回调地址无效。')
  if (!Array.isArray(v.scopes) || v.scopes.length < 1 || v.scopes.length > 5 || v.scopes[0] !== 'play') throw new Error('游戏权限无效。')
  let previous = -1
  for (const scope of v.scopes) {
    if (typeof scope !== 'string' || scopeOrder.indexOf(scope) <= previous) throw new Error('游戏权限无效。')
    previous = scopeOrder.indexOf(scope)
  }
  return v as unknown as GameRequest
}
/** Only this in-app route is permitted after main login. No arbitrary redirect support. */
export function gameLoginReturn(value: string | null): string | null {
  if (!value?.startsWith('/game-access?')) return null
  try {
    parseGameRequest(value.slice('/game-access'.length))
    return value
  } catch { return null }
}
export function authorizedCallback(value: unknown, request: GameRequest, now: number): string {
  const v = record(value)
  if (v.schema !== 'esk.game.access.code.v1' || v.redirect_uri !== request.redirect_uri || v.state !== request.state
    || typeof v.code !== 'string' || v.code.length !== 68 || !v.code.startsWith('egc_') || /[^0-9a-f]/.test(v.code.slice(4))
    || !Array.isArray(v.scopes) || JSON.stringify(v.scopes) !== JSON.stringify(request.scopes)
    || typeof v.code_expires_at_ms !== 'string' || /[^0-9]/.test(v.code_expires_at_ms)
    || v.code_expires_at_ms.length > 16 || Number(v.code_expires_at_ms) <= now
    || Number(v.code_expires_at_ms) > now + 122_000) throw new Error('授权响应无法确认，请重新发起登录。')
  const url = new URL(request.redirect_uri)
  url.searchParams.set('code', v.code)
  url.searchParams.set('state', request.state)
  return url.href
}
