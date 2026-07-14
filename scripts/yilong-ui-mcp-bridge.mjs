#!/usr/bin/env node

import { spawn } from 'node:child_process'
import { access, readFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import readline from 'node:readline'

const configuredNodeBaseUrl = (process.env.ELON_NODE_ADMIN_URL || '').replace(/\/+$/, '')
const firstNodePort = positiveInteger(process.env.ELON_UI_MCP_PORT_FIRST, 7799)
const fallbackPortLimit = positiveInteger(process.env.ELON_UI_MCP_PORT_LIMIT, 20)
const nodeBaseCandidates = configuredNodeBaseUrl
  ? [configuredNodeBaseUrl]
  : Array.from({ length: fallbackPortLimit + 1 }, (_, offset) => `http://127.0.0.1:${firstNodePort + offset}`)
let nodeBaseUrl = nodeBaseCandidates[0]
const projectRoot = process.env.CODEX_WORKSPACE_ROOT || process.cwd()
const autoStartEnabled = process.env.ELON_UI_MCP_AUTOSTART !== '0'
const reconnectTimeoutMs = positiveInteger(process.env.ELON_UI_MCP_RECONNECT_TIMEOUT_MS, 45_000)
const reconnectPollMs = positiveInteger(process.env.ELON_UI_MCP_RECONNECT_POLL_MS, 500)
let mcpUrl = ''
let lastError = ''
let lastStartResult = null
let toolsChangedPending = false
let proxyRequestId = 10_000
const recentStarts = new Map()

const bridgeTools = [
  tool(
    'ui_bridge_status',
    '即使一龙 PC 节点离线也可用：检查本机候选端口、项目 MCP 会话和最近一次自动启动结果。',
    { type: 'object', properties: {} },
    true,
  ),
  tool(
    'ui_bridge_reconnect',
    '启动或恢复本机一龙 PC 节点，重新创建项目 UI MCP 会话，并通知客户端刷新完整工具列表。',
    {
      type: 'object',
      properties: {
        waitMs: { type: 'integer', minimum: 1_000, maximum: 120_000 },
      },
    },
    false,
  ),
  tool(
    'ui_bridge_proxy',
    '客户端尚未刷新完整工具列表时的恢复入口：连接节点后按名称调用任意 yilong_ui_live 工具。',
    {
      type: 'object',
      required: ['name'],
      properties: {
        name: { type: 'string', pattern: '^ui_[a-z0-9_]{1,80}$' },
        arguments: { type: 'object' },
      },
    },
    false,
  ),
]

function tool(name, description, inputSchema, readOnly) {
  return {
    name,
    description,
    inputSchema,
    annotations: {
      readOnlyHint: readOnly,
      destructiveHint: false,
      idempotentHint: readOnly,
      openWorldHint: false,
    },
  }
}

function positiveInteger(value, fallback) {
  const parsed = Number.parseInt(value || '', 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
}

function errorText(error) {
  return error instanceof Error ? error.message : String(error)
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function clientExecutable() {
  if (process.env.ELON_NODE_CLIENT_EXE) return path.resolve(process.env.ELON_NODE_CLIENT_EXE)
  if (process.platform !== 'win32') return ''
  const localAppData = process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local')
  return path.join(localAppData, 'ElonNode', '一龙开发平台.exe')
}

async function startClient(mode, argument) {
  if (!autoStartEnabled) {
    const result = { mode, attempted: false, started: false, reason: 'AUTOSTART_DISABLED' }
    lastStartResult = { ...(lastStartResult || {}), [mode]: result }
    return result
  }
  const executable = clientExecutable()
  if (!executable) {
    const result = { mode, attempted: false, started: false, reason: 'CLIENT_EXECUTABLE_UNAVAILABLE' }
    lastStartResult = { ...(lastStartResult || {}), [mode]: result }
    return result
  }
  const previous = recentStarts.get(mode)
  if (previous && Date.now() - previous.at < 30_000) {
    const result = { ...previous.result, reused: true }
    lastStartResult = { ...(lastStartResult || {}), [mode]: result }
    return result
  }
  let result
  try {
    await access(executable)
    const child = spawn(executable, [argument], {
      detached: true,
      stdio: 'ignore',
      windowsHide: true,
    })
    child.unref()
    result = { mode, attempted: true, started: true, executable, pid: child.pid || null }
  } catch (error) {
    result = {
      mode,
      attempted: true,
      started: false,
      executable,
      reason: errorText(error),
    }
  }
  recentStarts.set(mode, { at: Date.now(), result })
  lastStartResult = { ...(lastStartResult || {}), [mode]: result }
  return result
}

async function startLocalRuntime() {
  return startClient('background', '--background')
}

async function startDirectRuntime() {
  return startClient('direct', '--agent-runtime')
}

async function probeNodeBase(baseUrl) {
  try {
    const response = await fetch(`${baseUrl}/api/status`, {
      headers: { accept: 'application/json' },
      signal: AbortSignal.timeout(1_000),
    })
    const text = await response.text()
    return {
      baseUrl,
      online: response.ok && text.includes('local_admin_token_header'),
      httpStatus: response.status,
    }
  } catch (error) {
    return { baseUrl, online: false, error: errorText(error) }
  }
}

async function discoverNodeBase() {
  const probes = await Promise.all(nodeBaseCandidates.map(probeNodeBase))
  const selected = probes.find((probe) => probe.online)
  if (selected) nodeBaseUrl = selected.baseUrl
  return { selected, probes }
}

async function bootstrapOnce() {
  const discovery = await discoverNodeBase()
  if (!discovery.selected) throw new Error(`一龙 PC 节点未监听候选端口：${nodeBaseCandidates.join(', ')}`)
  const response = await fetch(`${nodeBaseUrl}/api/android-live/project-mcp/bootstrap`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ projectRoot }),
    signal: AbortSignal.timeout(5_000),
  })
  const payload = await response.json().catch(() => ({}))
  if (!response.ok || !payload?.mcp?.configPath) {
    throw new Error(payload?.error || `一龙 PC 节点 MCP 启动失败（HTTP ${response.status}）`)
  }
  const config = JSON.parse(await readFile(payload.mcp.configPath, 'utf8'))
  const url = config?.mcpServers?.yilong_ui_live?.url
  if (typeof url !== 'string' || !/^http:\/\/(127\.0\.0\.1|localhost):\d+\/api\/android-live\/mcp\/live_/.test(url)) {
    throw new Error('一龙 PC 节点返回了无效的本机 MCP 地址')
  }
  mcpUrl = url
  lastError = ''
  return { mcpUrl, sessionId: payload?.mcp?.sessionId || null }
}

async function tryBootstrap() {
  try {
    return await bootstrapOnce()
  } catch (error) {
    mcpUrl = ''
    lastError = errorText(error)
    return null
  }
}

async function reconnect(waitMs = reconnectTimeoutMs) {
  mcpUrl = ''
  const immediate = await tryBootstrap()
  if (immediate) {
    toolsChangedPending = true
    return { connected: true, ...immediate, start: lastStartResult }
  }

  const background = await startLocalRuntime()
  let direct = null
  const startedAt = Date.now()
  const deadline = Date.now() + Math.max(1_000, Math.min(waitMs, 120_000))
  while (Date.now() < deadline) {
    await sleep(reconnectPollMs)
    const connected = await tryBootstrap()
    if (connected) {
      toolsChangedPending = true
      return { connected: true, ...connected, start: { background, direct } }
    }
    if (!autoStartEnabled) break
    if (!direct && Date.now() - startedAt >= 10_000) direct = await startDirectRuntime()
  }
  return {
    connected: false,
    status: 'TRANSPORT_UNAVAILABLE',
    nodeBaseUrl,
    projectRoot,
    start: { background, direct },
    error: lastError || '一龙 PC 节点未在等待时间内上线',
    next: background.reason === 'AUTOSTART_DISABLED' ? 'START_NODE_MANUALLY_AND_RETRY' : 'REPAIR_OR_UPDATE_NODE_CLIENT',
  }
}

async function nodeStatus() {
  const discovery = await discoverNodeBase()
  return discovery.selected || {
    online: false,
    candidates: nodeBaseCandidates,
    errors: discovery.probes.map((probe) => ({ baseUrl: probe.baseUrl, error: probe.error || probe.httpStatus })),
  }
}

async function proxy(message, retry = true) {
  if (!mcpUrl) {
    const connected = await tryBootstrap()
    if (!connected) throw new Error(lastError || '一龙 UI MCP 尚未连接；请先调用 ui_bridge_reconnect')
  }
  const response = await fetch(mcpUrl, {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'application/json' },
    body: JSON.stringify(message),
  })
  if (retry && (response.status === 401 || response.status === 404)) {
    mcpUrl = ''
    return proxy(message, false)
  }
  if (response.status === 202 || message.id === undefined || message.id === null) return null
  const responseText = await response.text()
  if (!response.ok) throw new Error(responseText || `一龙 UI MCP 请求失败（HTTP ${response.status}）`)
  return JSON.parse(responseText)
}

function localInitialize(id) {
  return {
    jsonrpc: '2.0',
    id,
    result: {
      protocolVersion: '2025-03-26',
      capabilities: { tools: { listChanged: true } },
      serverInfo: { name: 'yilong-ui-bootstrap', version: '1.1.0' },
      instructions:
        '一龙 UI 节点当前未连接。先调用 ui_bridge_reconnect；连接后客户端会刷新完整工具列表。若客户端未刷新，使用 ui_bridge_proxy 调用 ui_check_capabilities、ui_report_capability_gap 和其他 yilong_ui_live 工具。不要静默退化为全仓搜索或手工 ADB。',
    },
  }
}

function localToolResult(id, value, isError = false) {
  return {
    jsonrpc: '2.0',
    id,
    result: {
      content: [{ type: 'text', text: JSON.stringify(value, null, 2) }],
      structuredContent: value,
      isError,
    },
  }
}

async function callBridgeTool(message) {
  const name = message?.params?.name
  const args = message?.params?.arguments || {}
  if (name === 'ui_bridge_status') {
    const node = await nodeStatus()
    return localToolResult(message.id, {
      status: node.online ? 'ONLINE' : 'TRANSPORT_UNAVAILABLE',
      node,
      nodeBaseUrl,
      projectRoot,
      mcpConnected: Boolean(mcpUrl),
      lastError: lastError || null,
      lastStartResult,
      next: 'ui_bridge_reconnect',
    })
  }
  if (name === 'ui_bridge_reconnect') {
    const result = await reconnect(args.waitMs)
    return localToolResult(message.id, result, !result.connected)
  }
  if (name === 'ui_bridge_proxy') {
    const remoteName = typeof args.name === 'string' ? args.name.trim() : ''
    if (!/^ui_[a-z0-9_]{1,80}$/.test(remoteName) || remoteName.startsWith('ui_bridge_')) {
      throw new Error('ui_bridge_proxy.name 必须是非 bridge 的 ui_* 工具名')
    }
    if (!mcpUrl) {
      const result = await reconnect(args.waitMs)
      if (!result.connected) return localToolResult(message.id, result, true)
    }
    const remote = await proxy({
      jsonrpc: '2.0',
      id: ++proxyRequestId,
      method: 'tools/call',
      params: { name: remoteName, arguments: args.arguments || {} },
    })
    if (remote?.error) throw new Error(remote.error.message || '远程 UI MCP 工具调用失败')
    return { jsonrpc: '2.0', id: message.id, result: remote?.result }
  }
  return null
}

async function dispatch(message) {
  if (message.method === 'initialize') {
    const remote = await tryBootstrap()
    if (remote) return proxy(message)
    await startLocalRuntime()
    return localInitialize(message.id)
  }
  if (message.method === 'notifications/initialized' && !mcpUrl) return null
  if (message.method === 'tools/list' && !mcpUrl) {
    const remote = await tryBootstrap()
    if (!remote) return { jsonrpc: '2.0', id: message.id, result: { tools: bridgeTools } }
  }
  if (message.method === 'tools/call' && bridgeTools.some((item) => item.name === message?.params?.name)) {
    return callBridgeTool(message)
  }
  return proxy(message)
}

function write(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`)
}

const lines = readline.createInterface({ input: process.stdin, crlfDelay: Infinity })
for await (const line of lines) {
  if (!line.trim()) continue
  let message
  try {
    message = JSON.parse(line)
    const response = await dispatch(message)
    if (response) write(response)
    if (toolsChangedPending) {
      toolsChangedPending = false
      write({ jsonrpc: '2.0', method: 'notifications/tools/list_changed' })
    }
  } catch (error) {
    lastError = errorText(error)
    if (message?.id !== undefined && message?.id !== null) {
      write({
        jsonrpc: '2.0',
        id: message.id,
        error: { code: -32001, message: lastError },
      })
    }
  }
}
