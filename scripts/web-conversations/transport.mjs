import { createHash } from 'node:crypto'
import { json, localUrl, result, rpc, sleep } from './local-rpc.mjs'
import { connectWin } from './win-runtime.mjs'
export { json, localUrl, result } from './local-rpc.mjs'

export async function winSource(env, projectRoot) {
  const { call, instance } = await connectWin(env, projectRoot)
  return { source: 'win', identity: instance, async read(input) {
    let response = await call('submit', { kind: 'read_conversation', instance_id: instance, query: JSON.stringify(input) })
    const actionId = response.action?.action_id, deadline = Date.now() + 25000
    if (!actionId) throw new Error('invalid_action_receipt')
    while (!response.terminal && Date.now() < deadline) {
      await sleep(300)
      response = await call('action_status', { action_id: actionId })
    }
    if (!response.terminal) throw new Error('win_action_timeout')
    if (response.action?.status !== 'succeeded') {
      const code = response.action?.receipt?.error_code
      throw new Error(['host_unavailable', 'host_mismatch', 'session_expired', 'unsupported', 'invalid_scope'].includes(code)
        ? `win_${code}` : 'win_read_failed')
    }
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
  const control = async args => {
    const reply = await json(base + '/mcp', rpc('ui_control', { ...args, auth_token: token }))
    // Native UI reports recoverable page readiness through an MCP error envelope.
    // Only read readiness may recover; navigation and unrelated errors still fail.
    const error = reply.result?.structuredContent?.error
    if (!reply.error && reply.result?.isError === true && args.action === 'chatgpt_read_conversation' &&
      ['chatgpt_web_chat_inactive', 'adapter_generation_not_ready', 'bridge_not_ready'].includes(error)) {
      return { error }
    }
    return result(reply)
  }
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
