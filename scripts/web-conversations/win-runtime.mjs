import { spawn } from 'node:child_process'
import { access } from 'node:fs/promises'
import path from 'node:path'
import { json, localUrl, result, rpc, sleep } from './local-rpc.mjs'

export async function launchInstalledWin(env = process.env) {
  if (process.platform !== 'win32') throw new Error('win_start_requires_windows')
  if (!env.LOCALAPPDATA || !path.isAbsolute(env.LOCALAPPDATA)) throw new Error('win_not_installed')
  // Use the installed launcher: it owns singleton, update locks and node recovery.
  // There is deliberately no tool argument for an executable, URL or command line.
  const executable = path.join(env.LOCALAPPDATA, 'ElonNode', '一龙开发平台.exe')
  try { await access(executable) } catch { throw new Error('win_not_installed') }
  await new Promise((resolve, reject) => {
    const child = spawn(executable, [], { cwd: path.dirname(executable), windowsHide: true,
      detached: true, stdio: 'ignore' })
    child.once('error', () => reject(new Error('win_start_failed')))
    child.once('spawn', () => { child.unref(); resolve() })
  })
}

export async function discoverWin(env) {
  const candidates = env.ELON_NODE_ADMIN_URL ? [localUrl(env.ELON_NODE_ADMIN_URL)] :
    Array.from({ length: 21 }, (_, index) => `http://127.0.0.1:${7799 + index}`)
  const results = await Promise.allSettled(candidates.map(async base => {
    const response = await fetch(base + '/api/health', { redirect: 'error', signal: AbortSignal.timeout(350) })
    if (!response.ok) return null
    const health = await response.json()
    return health.status === 'ok' && health.service === 'elon-node-agent' ? base : null
  }))
  const found = results.filter(r => r.status === 'fulfilled' && r.value).map(r => r.value)
  if (found.length > 1) throw new Error('win_node_selection_required')
  return found[0] || null
}

export async function connectWin(env, projectRoot, dependencies = {}) {
  const discover = dependencies.discover || (() => discoverWin(env))
  const launch = dependencies.launch || (() => launchInstalledWin(env))
  const delay = dependencies.sleep || sleep, now = dependencies.now || Date.now
  const bootstrap = dependencies.bootstrap || (async base => {
    const descriptor = await json(base + '/api/project-docs/mcp/bootstrap', { projectRoot, profile: 'browser_research' })
    const endpoint = descriptor.mcp?.url
    if (!endpoint || new URL(endpoint).origin !== base) throw new Error('invalid_mcp_descriptor')
    return (action, payload = {}) => json(endpoint, rpc('browser_research', { action, payload })).then(result)
  })
  let started = false
  const start = async () => {
    if (env.ELON_WEB_CONVERSATION_AUTOSTART === '0') throw new Error('win_host_offline')
    if (!started) { await launch(); started = true }
  }
  let base = await discover()
  if (!base) {
    await start()
    const deadline = now() + 30000
    while (!base && now() < deadline) { await delay(500); base = await discover() }
    if (!base) throw new Error('win_node_start_timeout')
  }
  const call = await bootstrap(base)
  const contract = await call('describe')
  if (!contract.commands?.read_conversation) throw new Error('win_reader_update_required')
  const choose = hosts => {
    const selected = env.ELON_WEB_CONVERSATION_WIN_INSTANCE
    if (selected) return hosts.find(h => h.instance_id === selected)?.instance_id
    if (hosts.length > 1) throw new Error('win_host_selection_required')
    return hosts[0]?.instance_id
  }
  let hosts = (await call('hosts')).hosts || [], instance = choose(hosts)
  if (!instance) {
    // A pinned instance belongs to a specific process/owner; never silently replace it.
    if (env.ELON_WEB_CONVERSATION_WIN_INSTANCE) throw new Error('win_host_offline')
    await start()
    const deadline = now() + 30000
    while (!instance && now() < deadline) {
      await delay(500); hosts = (await call('hosts')).hosts || []; instance = choose(hosts)
    }
    if (!instance) throw new Error('win_host_start_timeout')
  }
  return { call, instance }
}
