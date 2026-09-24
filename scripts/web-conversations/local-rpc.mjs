// Local MCP credentials never leave the transport closure.
export function localUrl(value) {
  const url = new URL(value)
  if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || url.username || url.password || url.search || url.hash || url.pathname !== '/') throw new Error('invalid_local_endpoint')
  return url.origin
}
export async function json(url, body, headers = {}) {
  const response = await fetch(url, { method: body === undefined ? 'GET' : 'POST',
    headers: { 'content-type': 'application/json', ...headers },
    body: body === undefined ? undefined : JSON.stringify(body), redirect: 'error', signal: AbortSignal.timeout(12000) })
  if (!response.ok) throw new Error(`transport_http_${response.status}`)
  const text = await response.text()
  if (text.length > 128 * 1024) throw new Error('transport_result_too_large')
  return JSON.parse(text)
}
export function result(reply) {
  if (reply.error || reply.result?.isError) throw new Error('mcp_call_failed')
  const value = reply.result?.structuredContent ?? reply.result?.content?.filter(item => item.type === 'text')
    .map(item => { try { return JSON.parse(item.text) } catch { return null } }).find(value => value && typeof value === 'object')
  if (!value) throw new Error('invalid_mcp_result')
  return value
}
export const rpc = (name, args) => ({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name, arguments: args } })
export const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))
