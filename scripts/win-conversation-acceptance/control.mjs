import { discoverWin, launchInstalledWin } from '../web-conversations/win-runtime.mjs'
import { json, localUrl, result, rpc, sleep } from '../web-conversations/local-rpc.mjs'

export const exactRelease = value => typeof value === 'string' && /^[A-Za-z0-9._-]{1,48}\+[a-fA-F0-9]{40,64}$/.test(value)
export function safeCode(error) {
  return /^[a-z][a-z_0-9]{0,79}$/.test(error?.message || '') ? error.message : 'acceptance_failed'
}

// Only the installed launcher and existing project-bound semantic MCP are used.
// A lost mutation reply is never retried here. The workflow owns reconciliation.
export function createControl({ env, projectRoot, request = json, discover = () => discoverWin(env),
  launch = () => launchInstalledWin(env), delay = sleep, now = Date.now } = {}) {
  let base, endpoint, launched = false
  const start = async () => {
    if (env.ELON_WEB_CONVERSATION_AUTOSTART === '0') throw Error('win_host_offline')
    if (!launched) { launched = true; await launch() }
  }
  async function bind() {
    const descriptor = await request(base + '/api/project-docs/mcp/bootstrap', { projectRoot, profile: 'win_control' })
    const url = new URL(descriptor.mcp?.url || 'invalid:')
    if (url.origin !== base || url.username || url.password || url.hash) throw Error('invalid_mcp_descriptor')
    endpoint = url.href
  }
  async function call(name, args = {}) {
    if (!endpoint) await bind()
    try { return result(await request(endpoint, rpc(name, args))) }
    catch (error) { endpoint = null; throw error }
  }
  async function status() {
    const value = (await call('win_control_status')).capabilities
    if (value?.schema !== 'elon.win_codex_control.v1' || !exactRelease(value.release_identity) ||
        typeof value.tauri_available !== 'boolean' || typeof value.frontend_available !== 'boolean') throw Error('invalid_win_capabilities')
    return { release_identity: value.release_identity, tauri_available: value.tauri_available,
      frontend_available: value.frontend_available }
  }
  async function connect(pinnedBase, { waitOnly = false } = {}) {
    base = pinnedBase ? localUrl(pinnedBase) : await discover()
    const deadline = now() + (waitOnly ? 300000 : 60000)
    if (!base && !waitOnly) await start()
    while (!base && now() < deadline) { await delay(500); base = await discover() }
    if (!base) throw Error('win_node_start_timeout')
    let value
    while (now() < deadline) {
      try { value = await status() } catch (error) {
        if (['invalid_win_capabilities', 'invalid_mcp_descriptor'].includes(safeCode(error))) throw error
      }
      if (value?.tauri_available && value?.frontend_available) break
      if (!waitOnly) await start()
      await delay(500)
    }
    if (!value?.tauri_available || !value?.frontend_available) throw Error('win_host_start_timeout')
    return { base, status: value }
  }
  const kinds = new Set(['update_and_restart', 'reload_page', 'navigate', 'capture_state'])
  async function submit(kind, target, trace) {
    if (!kinds.has(kind) || kind === 'update_and_restart' && !exactRelease(target)) throw Error('invalid_control_action')
    const args = { kind, trace_id: trace }
    if (kind === 'update_and_restart') args.target_release_identity = target
    if (kind === 'navigate') args.route = '/ai'
    const value = await call('win_control_action', args)
    return validateAction(value.action, kind)
  }
  function validateAction(action, kind, id) {
    if (!/^win_act_[a-f0-9]{32}$/.test(action?.action_id || '') || action.kind !== kind ||
        id && action.action_id !== id || !['queued', 'executing', 'succeeded', 'failed', 'rejected', 'expired', 'host_unavailable'].includes(action.status)) {
      throw Error('invalid_action_receipt')
    }
    // Keep the receipt's actual fixed route, never titles or arbitrary window data.
    return { action_id: action.action_id, kind, status: action.status,
      ...(action.receipt?.route === '/ai' ? { route: '/ai' } : {}) }
  }
  async function actionStatus(id, kind) {
    return validateAction((await call('win_control_action_status', { action_id: id })).action, kind, id)
  }
  async function waitAction(action) {
    const deadline = now() + 45000
    while (['queued', 'executing'].includes(action.status) && now() < deadline) {
      await delay(500); action = await actionStatus(action.action_id, action.kind)
    }
    if (action.status !== 'succeeded') throw Error('win_action_' + (['queued', 'executing'].includes(action.status) ? 'timeout' : action.status))
    return action
  }
  async function waitRelease(target, timeoutMs = 300000) {
    const deadline = now() + timeoutMs
    while (now() < deadline) {
      try {
        const value = await status()
        if (value.release_identity === target && value.tauri_available && value.frontend_available) return value
      } catch { /* Node restart invalidates short-lived MCP tokens; rebind next poll. */ }
      await delay(1000)
    }
    throw Error('win_release_timeout')
  }
  return { connect, status, submit, actionStatus, waitAction, waitRelease }
}
