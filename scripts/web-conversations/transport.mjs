import { createHash } from 'node:crypto'
import { json, localUrl, result, rpc, sleep } from './local-rpc.mjs'
export { json, localUrl, result } from './local-rpc.mjs'

export async function winSource(env, projectRoot) {
  const candidates = env.ELON_NODE_ADMIN_URL ? [localUrl(env.ELON_NODE_ADMIN_URL)] :
    Array.from({ length: 21 }, (_, index) => `http://127.0.0.1:${7799 + index}`)
  let base
  for (const candidate of candidates) {
    try {
      const response = await fetch(candidate + '/api/health', { redirect: 'error', signal: AbortSignal.timeout(350) })
      if (response.ok) { base = candidate; break }
    } catch { /* Bounded local discovery. */ }
  }
  if (!base) throw new Error('win_node_offline')
  const descriptor = await json(base + '/api/project-docs/mcp/bootstrap', { projectRoot, profile: 'browser_research' })
  const endpoint = descriptor.mcp?.url
  if (!endpoint || new URL(endpoint).origin !== base) throw new Error('invalid_mcp_descriptor')
  const call = async (action, payload = {}) => result(await json(endpoint, rpc('browser_research', { action, payload })))
  const { hosts } = await call('hosts')
  const instance = env.ELON_WEB_CONVERSATION_WIN_INSTANCE || (hosts?.length === 1 ? hosts[0].instance_id : '')
  if (!instance || !hosts?.some(host => host.instance_id === instance)) throw new Error(hosts?.length > 1 ? 'win_host_selection_required' : 'win_host_offline')
  return { source: 'win', identity: instance, async read(input) {
    let response = await call('submit', { kind: 'read_conversation', instance_id: instance, query: JSON.stringify(input) })
    const actionId = response.action?.action_id, deadline = Date.now() + 25000
    if (!actionId) throw new Error('invalid_action_receipt')
    while (!response.terminal && Date.now() < deadline) {
      await sleep(300)
      response = await call('action_status', { action_id: actionId })
    }
    if (!response.terminal) throw new Error('win_action_timeout')
    if (response.action?.status !== 'succeeded') throw new Error('win_read_failed')
    const value = response.action.receipt?.result?.reader
    if (!value) throw new Error('win_reader_update_required')
    return value
  } }
}

export async function apkSource(env) {
  if (!env.ELON_APK_MCP_URL) throw new Error('apk_endpoint_not_configured')
  const base = localUrl(env.ELON_APK_MCP_URL)
  const health = await json(base + '/health')
  if (typeof health.auth_token !== 'string' || !health.auth_token) throw new Error('apk_unavailable')
  const token = health.auth_token
  let opened = false
  const control = args => json(base + '/mcp', rpc('ui_control', { ...args, auth_token: token })).then(result)
  return { source: 'apk', identity: createHash('sha256').update(token).digest('hex'), async read(input) {
    if ((await json(base + '/health')).auth_token !== token) throw new Error('apk_session_changed')
    const value = await control({
      action: 'chatgpt_read_conversation', conversation_id: input.conversation_id,
      request_id: input.request_id, message_cursor: input.cursor || '',
    })
    if (value.error === 'chatgpt_web_chat_inactive' && !opened) {
      opened = true
      await control({ action: 'open_chatgpt_web' })
      await sleep(1000)
      return { status: 'pending', request_id: input.request_id }
    }
    if (value.error === 'adapter_generation_not_ready' || value.error === 'bridge_not_ready') {
      return { status: 'pending', request_id: input.request_id }
    }
    if (!['ready', 'pending', 'failed'].includes(value.status)) throw new Error(value.error || 'apk_reader_unavailable')
    return value
  } }
}
