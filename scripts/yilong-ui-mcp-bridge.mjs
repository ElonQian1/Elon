#!/usr/bin/env node

import { readFile } from 'node:fs/promises'
import readline from 'node:readline'

const nodeBaseUrl = (process.env.ELON_NODE_ADMIN_URL || 'http://127.0.0.1:7799').replace(/\/+$/, '')
const projectRoot = process.env.CODEX_WORKSPACE_ROOT || process.cwd()
let mcpUrl = ''

async function bootstrap() {
  const response = await fetch(`${nodeBaseUrl}/api/android-live/project-mcp/bootstrap`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ projectRoot }),
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
}

async function proxy(message, retry = true) {
  if (!mcpUrl) await bootstrap()
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
  const text = await response.text()
  if (!response.ok) throw new Error(text || `一龙 UI MCP 请求失败（HTTP ${response.status}）`)
  return JSON.parse(text)
}

const lines = readline.createInterface({ input: process.stdin, crlfDelay: Infinity })
for await (const line of lines) {
  if (!line.trim()) continue
  let message
  try {
    message = JSON.parse(line)
    const response = await proxy(message)
    if (response) process.stdout.write(`${JSON.stringify(response)}\n`)
  } catch (error) {
    if (message?.id !== undefined && message?.id !== null) {
      process.stdout.write(`${JSON.stringify({
        jsonrpc: '2.0',
        id: message.id,
        error: { code: -32001, message: error instanceof Error ? error.message : String(error) },
      })}\n`)
    }
  }
}
