#!/usr/bin/env node
import { readFile, mkdir, open, rename, rm } from 'node:fs/promises'
import { randomUUID } from 'node:crypto'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { prepareRegistration } from './registration.mjs'
import { localUrl } from './local-rpc.mjs'

const object = value => value && typeof value === 'object' && !Array.isArray(value)
const deviceKeys = ['ELON_APK_MCP_URL', 'ELON_WEB_CONVERSATION_WIN_INSTANCE',
  'ELON_NODE_ADMIN_URL', 'ELON_WEB_CONVERSATION_AUTOSTART']
async function existing(file) {
  try { return await readFile(file, 'utf8') } catch (error) {
    if (error.code === 'ENOENT') return null
    throw error
  }
}

// Merge only our server. Never print the existing configuration or its secrets.
// Immutable assets survive task-worktree cleanup; backup permits exact rollback.
export async function registerClaude({ configPath, projectRoot, references, storageRoot, apkMcpUrl, hostEnv = process.env }) {
  if (!path.isAbsolute(configPath)) throw Error('absolute_config_path_required')
  if (apkMcpUrl !== undefined && apkMcpUrl !== '') localUrl(apkMcpUrl)
  await mkdir(path.dirname(configPath), { recursive: true })
  const lockPath = configPath + '.yilong.lock'
  let lock
  try { lock = await open(lockPath, 'wx', 0o600) } catch { throw Error('claude_config_locked') }
  const temporary = configPath + '.' + randomUUID() + '.tmp'
  try {
    const original = await existing(configPath)
    let config
    try { config = original === null ? {} : JSON.parse(original.replace(/^\uFEFF/, '')) }
    catch { throw Error('claude_config_invalid') }
    if (!object(config) || config.mcpServers !== undefined && !object(config.mcpServers)) throw Error('claude_config_invalid')
    const previous = config.mcpServers?.yilong_web_conversations
    if (previous !== undefined && (!object(previous) || previous.env !== undefined && !object(previous.env))) throw Error('claude_reader_config_invalid')
    const registration = await prepareRegistration({ projectRoot, references, storageRoot })
    const env = { ELON_PROJECT_ROOT: registration.projectRoot, ELON_WEB_CONVERSATION_IDS: registration.ids }
    // Desktop MCP hosts can inherit a reduced environment. The fixed Win launcher
    // needs its actual per-user installation root even after a cold start.
    for (const key of ['LOCALAPPDATA', 'APPDATA', 'USERPROFILE', 'SystemRoot']) {
      if (typeof hostEnv[key] === 'string' && path.isAbsolute(hostEnv[key])) env[key] = hostEnv[key]
    }
    for (const key of deviceKeys) if (typeof previous?.env?.[key] === 'string') env[key] = previous.env[key]
    if (apkMcpUrl !== undefined) env.ELON_APK_MCP_URL = apkMcpUrl
    for (const key of ['ELON_APK_MCP_URL', 'ELON_NODE_ADMIN_URL']) if (env[key]) localUrl(env[key])
    const server = { command: registration.command, args: [registration.entrypoint], env }
    const updated = { ...config, mcpServers: { ...config.mcpServers, yilong_web_conversations: server } }
    if (JSON.stringify(config) === JSON.stringify(updated)) return { status: 'unchanged', config: configPath, restart_required: true }
    if (await existing(configPath) !== original) throw Error('claude_config_changed')
    let backup
    if (original !== null) {
      backup = configPath + '.yilong-' + randomUUID() + '.bak'
      const file = await open(backup, 'wx', 0o600)
      try { await file.writeFile(original); await file.sync() } finally { await file.close() }
    }
    const file = await open(temporary, 'wx', 0o600)
    try { await file.writeFile(JSON.stringify(updated, null, 2) + '\n'); await file.sync() } finally { await file.close() }
    if (await existing(configPath) !== original) throw Error('claude_config_changed')
    await rename(temporary, configPath)
    return { status: 'registered', config: configPath, backup, restart_required: true }
  } finally { await rm(temporary, { force: true }); await lock.close(); await rm(lockPath) }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    let input = ''; for await (const part of process.stdin) { input += part; if (input.length > 16384) throw Error('registration_input_limit') }
    const args = JSON.parse(input)
    const configPath = args.configPath || path.join(process.env.APPDATA, 'Claude', 'claude_desktop_config.json')
    const storageRoot = path.join(process.env.LOCALAPPDATA, 'Elon', 'web-conversation-reader')
    process.stdout.write(JSON.stringify(await registerClaude({ ...args, configPath, storageRoot, hostEnv: process.env })) + '\n')
  } catch (error) {
    const code = /^[a-z_]{1,80}$/.test(error.message) ? error.message : 'claude_registration_failed'
    process.stderr.write(code + '\n'); process.exitCode = 1
  }
}
