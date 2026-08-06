#!/usr/bin/env node

import { spawn } from 'node:child_process'
import readline from 'node:readline'
import path from 'node:path'

const options = parseArgs(process.argv.slice(2))
const child = spawn(options.codexBin, [
  'app-server', '--stdio', '--disable', 'apps',
  '-c', 'mcp_servers.openaiDeveloperDocs.enabled=false',
  '-c', 'mcp_servers.node_repl.enabled=false',
], {
  cwd: options.codexProjectRoot,
  env: {
    ...process.env,
    ELON_NODE_ADMIN_URL: options.nodeAdminUrl,
    ELON_PROJECT_ROOT: options.memoryProjectRoot,
  },
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true,
})
const pending = new Map()
const startupStates = new Map()
const startupEvents = []
const stderr = []
let nextId = 1
const lines = readline.createInterface({ input: child.stdout, crlfDelay: Infinity })
lines.on('line', (line) => {
  if (!line.trim()) return
  let message
  try {
    message = JSON.parse(line)
  } catch {
    return
  }
  if (message.method === 'mcpServer/startupStatus/updated') {
    const event = message.params || {}
    startupStates.set(String(event.name || ''), event)
    startupEvents.push(event)
    return
  }
  if (message.id === undefined || message.id === null) return
  const waiter = pending.get(String(message.id))
  if (!waiter) return
  pending.delete(String(message.id))
  clearTimeout(waiter.timer)
  if (message.error) waiter.reject(new Error(JSON.stringify(message.error)))
  else waiter.resolve(message.result)
})
child.stderr.on('data', (chunk) => {
  const text = String(chunk)
  if (stderr.join('').length < 32 * 1024) stderr.push(text)
})
child.once('exit', (code) => {
  for (const waiter of pending.values()) {
    clearTimeout(waiter.timer)
    waiter.reject(new Error(`Codex app-server exited early with code ${code}: ${stderr.join('').slice(-4000)}`))
  }
  pending.clear()
})

try {
  const startedAt = Date.now()
  await request('initialize', {
    clientInfo: { name: 'yilong-project-memory-test', title: 'Yilong Project Memory Test', version: '1.0.0' },
    capabilities: null,
  })
  const thread = await request('thread/start', {
    cwd: options.codexProjectRoot,
    approvalPolicy: 'never',
    sandbox: 'read-only',
    ephemeral: true,
  })
  const threadId = thread?.thread?.id
  if (!threadId) throw new Error('Codex app-server did not return a thread id')
  const expected = {
    'yilong-project-context': 'project_context_plan',
    'yilong-project-features': 'project_feature_workflow',
    'yilong-project-memory-receipt': 'project_docs_record_native_context_receipt',
  }
  const status = await waitForExpectedMcp(threadId, expected)
  const selected = {}
  let toolSchemaBytes = 0
  for (const [serverName, toolName] of Object.entries(expected)) {
    const server = status?.data?.find((entry) => entry?.name === serverName)
    if (!server) throw new Error(`Codex did not register MCP server ${serverName}`)
    const tools = Object.keys(server.tools || {})
    if (tools.length !== 1 || tools[0] !== toolName) {
      throw new Error(`${serverName} exposed ${JSON.stringify(tools)} instead of only ${toolName}`)
    }
    toolSchemaBytes += Buffer.byteLength(JSON.stringify(server.tools || {}), 'utf8')
    selected[serverName] = { tool_count: tools.length, tool: tools[0] }
  }
  const callStartedAt = Date.now()
  const call = await request('mcpServer/tool/call', {
    threadId,
    server: 'yilong-project-features',
    tool: 'project_feature_workflow',
    arguments: { action: 'list', payload: { query: options.featureId, limit: 1 } },
  }, 30_000)
  if (call?.isError) throw new Error(`Codex MCP tool call returned isError: ${JSON.stringify(call)}`)
  const payload = call?.structuredContent || {}
  const payloadText = JSON.stringify(payload)
  if (!payloadText.includes(options.featureId)) {
    throw new Error(`Feature ${options.featureId} was not returned through Codex: ${payloadText.slice(0, 1000)}`)
  }
  process.stdout.write(`${JSON.stringify({
    schema: 'elon.project_memory_codex_app_server_test.v1',
    codex_native_mcp_ready: true,
    tool_call_succeeded: true,
    feature_id: options.featureId,
    server_count: Object.keys(selected).length,
    tool_schema_bytes: toolSchemaBytes,
    tool_schema_estimated_tokens: Math.ceil(toolSchemaBytes / 4),
    servers: selected,
    startup_ms: callStartedAt - startedAt,
    tool_call_ms: Date.now() - callStartedAt,
    response_bytes: Buffer.byteLength(JSON.stringify(call), 'utf8'),
  }, null, 2)}\n`)
} finally {
  lines.close()
  child.kill()
}

function request(method, params, timeoutMs = 30_000) {
  const id = nextId++
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(String(id))
      reject(new Error(`${method} timed out after ${timeoutMs} ms: ${stderr.join('').slice(-4000)}`))
    }, timeoutMs)
    pending.set(String(id), { resolve, reject, timer })
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`)
  })
}

async function waitForExpectedMcp(threadId, expected) {
  let latest = null
  try {
    latest = await request('mcpServerStatus/list', {
      threadId,
      detail: 'full',
      limit: 100,
    }, 10_000)
  } catch (error) {
    if (!String(error?.message || '').includes('timed out')) throw error
  }
  const deadline = Date.now() + 30_000
  while (Date.now() < deadline) {
    const ready = Object.entries(expected).every(([serverName, toolName]) => {
      const server = latest?.data?.find((entry) => entry?.name === serverName)
      const tools = Object.keys(server?.tools || {})
      return tools.length === 1 && tools[0] === toolName
    })
    if (ready) return latest
    const failed = Object.keys(expected).map((name) => startupStates.get(name)).find((entry) => entry?.status === 'failed')
    if (failed) throw new Error(`Codex MCP startup failed: ${JSON.stringify(failed)}`)
    const allReady = Object.keys(expected).every((name) => startupStates.get(name)?.status === 'ready')
    if (allReady) {
      latest = await request('mcpServerStatus/list', {
        threadId,
        detail: 'full',
        limit: 100,
      }, 10_000)
      continue
    }
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  const observed = Object.fromEntries(Object.entries(expected).map(([serverName]) => {
    const server = latest?.data?.find((entry) => entry?.name === serverName)
    return [serverName, Object.keys(server?.tools || {})]
  }))
  throw new Error(`Codex MCP tools did not become ready: ${JSON.stringify({ observed, startupEvents })}`)
}

function parseArgs(args) {
  const values = new Map()
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index]
    const value = args[index + 1]
    if (!key?.startsWith('--') || value === undefined) throw new Error(`Missing value for ${key || 'argument'}`)
    values.set(key, value)
  }
  const codexBin = path.resolve(required('--codex-bin'))
  const codexProjectRoot = path.resolve(required('--codex-project-root'))
  const memoryProjectRoot = path.resolve(required('--memory-project-root'))
  const nodeAdminUrl = required('--node-admin-url').replace(/\/$/, '')
  if (!/^http:\/\/127\.0\.0\.1:\d+$/.test(nodeAdminUrl)) throw new Error('--node-admin-url must be loopback HTTP')
  return {
    codexBin,
    codexProjectRoot,
    memoryProjectRoot,
    nodeAdminUrl,
    featureId: values.get('--feature-id') || 'runtime-feature',
  }

  function required(key) {
    const value = values.get(key)
    if (!value) throw new Error(`${key} is required`)
    return value
  }
}
