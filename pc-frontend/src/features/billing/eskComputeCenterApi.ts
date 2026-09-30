import { resolveApiUrl } from '../../api/runtime'
import { parseSnapshot } from './eskComputeCenterModel'

export function secureCenterUrl(page: number): string {
  const url = new URL(resolveApiUrl(`/api/me/esk-compute-center?page=${page}`), location.href)
  // The repository's dedicated verified asset ingress; never guess other hosts/ports.
  if (url.hostname === '43.139.149.158' && url.port === '8080') {
    url.protocol = 'https:'; url.port = '8443'
  }
  if (url.protocol !== 'https:' || url.username || url.password || url.hash) {
    throw new Error('当前服务连接尚不支持安全资产读取，请使用已配置 HTTPS 的主服务。')
  }
  return url.href
}
export async function readCenter(page: number, tokenProvider: () => string | null, signal: AbortSignal) {
  if (location.protocol !== 'https:') throw new Error('请在安全窗口登录和查看正式资产。原工作台不会向资产服务发送凭据。')
  const url = secureCenterUrl(page) // HTTPS check precedes credential access.
  const token = tokenProvider()
  if (!token) throw new Error('请先登录自己的账户')
  const response = await fetch(url, {
    headers: { Accept:'application/json',Authorization:`Bearer ${token}` },
    signal,cache:'no-store',credentials:'omit',redirect:'error',referrerPolicy:'no-referrer',
  })
  if (!response.ok) throw new Error(response.status === 401 ? '登录已失效，请重新登录' : '账户中心暂不可用，请重试')
  if (!(response.headers.get('content-type') || '').startsWith('application/json')) throw new Error('账户资料格式无效')
  const reader = response.body?.getReader()
  if (!reader) throw new Error('账户资料为空')
  const chunks: Uint8Array[] = []
  let size = 0
  try {
    while (true) {
      const { value,done } = await reader.read()
      if (done) break
      size += value.length
      if (size > 131072) throw new Error('账户资料超出安全范围')
      chunks.push(value)
    }
  } finally { await reader.cancel().catch(() => undefined); reader.releaseLock() }
  const bytes = new Uint8Array(size)
  let offset = 0
  chunks.forEach(chunk => { bytes.set(chunk,offset); offset += chunk.length })
  return parseSnapshot(JSON.parse(new TextDecoder('utf-8', { fatal:true }).decode(bytes)))
}
