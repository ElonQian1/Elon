#!/usr/bin/env node

import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import http from 'node:http'
import os from 'node:os'
import path from 'node:path'
import readline from 'node:readline'

const repoRoot = path.resolve(import.meta.dirname, '..')
const bridgePath = path.join(repoRoot, 'scripts', 'yilong-ui-mcp-bridge.mjs')
const oneShotPath = path.join(repoRoot, 'scripts', 'call-yilong-ui-mcp.mjs')
const tempRoot = await mkdtemp(path.join(os.tmpdir(), 'yilong-ui-bridge-test-'))
const bridges = []
let server

try {
  const reserved = await listen(http.createServer(), 0)
  const port = reserved.address().port
  await close(reserved)
  const adminUrl = `http://127.0.0.1:${port}`
  const client = startBridge(adminUrl)
  bridges.push(client.child)

  client.send({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {} })
  const offlineInitialize = await client.next((message) => message.id === 1)
  assert.equal(offlineInitialize.result.serverInfo.name, 'yilong-ui-bootstrap')
  assert.equal(offlineInitialize.result.capabilities.tools.listChanged, true)

  client.send({ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} })
  const fallbackTools = await client.next((message) => message.id === 2)
  assert.deepEqual(
    fallbackTools.result.tools.map((tool) => tool.name),
    ['ui_bridge_status', 'ui_bridge_reconnect', 'ui_bridge_proxy'],
  )

  client.send({
    jsonrpc: '2.0',
    id: 3,
    method: 'tools/call',
    params: { name: 'ui_bridge_status', arguments: {} },
  })
  const offlineStatus = await client.next((message) => message.id === 3)
  assert.equal(offlineStatus.result.structuredContent.status, 'TRANSPORT_UNAVAILABLE')

  const configPath = path.join(tempRoot, 'mcp.json')
  await writeFile(
    configPath,
    JSON.stringify({
      mcpServers: {
        yilong_ui_live: {
          url: `${adminUrl}/api/android-live/mcp/live_test?token=secret`,
        },
      },
    }),
  )
  server = await listen(createFakeNode(configPath), port)

  client.send({
    jsonrpc: '2.0',
    id: 4,
    method: 'tools/call',
    params: { name: 'ui_bridge_reconnect', arguments: { waitMs: 5_000 } },
  })
  const recovered = await client.next((message) => message.id === 4)
  assert.equal(recovered.result.structuredContent.connected, true)
  const listChanged = await client.next(
    (message) => message.method === 'notifications/tools/list_changed',
  )
  assert.equal(listChanged.jsonrpc, '2.0')

  client.send({ jsonrpc: '2.0', id: 5, method: 'tools/list', params: {} })
  const remoteTools = await client.next((message) => message.id === 5)
  assert.deepEqual(remoteTools.result.tools.map((tool) => tool.name), ['ui_fake_remote'])

  client.send({
    jsonrpc: '2.0',
    id: 6,
    method: 'tools/call',
    params: {
      name: 'ui_bridge_proxy',
      arguments: { name: 'ui_check_capabilities', arguments: { taskId: 'task_1' } },
    },
  })
  const proxied = await client.next((message) => message.id === 6)
  assert.equal(proxied.result.structuredContent.tool, 'ui_check_capabilities')
  assert.equal(proxied.result.structuredContent.arguments.taskId, 'task_1')

  const unicodeArgumentsPath = path.join(tempRoot, '中文参数.json')
  const unicodeArguments = {
    request: '首页顶部栏与聊天页对齐',
    attachments: [{ displayName: '当前批注截图', intent: 'ANNOTATED_CHANGE_REQUEST' }],
  }
  await writeFile(unicodeArgumentsPath, `\uFEFF${JSON.stringify(unicodeArguments)}`, 'utf8')
  const oneShot = await runOneShot(adminUrl, unicodeArgumentsPath)
  assert.equal(oneShot.tool, 'ui_import_desktop_task')
  assert.deepEqual(oneShot.arguments, unicodeArguments)

  const scannedClient = startBridge('', port)
  bridges.push(scannedClient.child)
  scannedClient.send({ jsonrpc: '2.0', id: 7, method: 'initialize', params: {} })
  const scannedInitialize = await scannedClient.next((message) => message.id === 7)
  assert.equal(scannedInitialize.result.serverInfo.name, 'fake-yilong-ui-live')

  process.stdout.write(
    'PASS: offline bootstrap, port discovery, runtime recovery, tool refresh, proxy fallback, and UTF-8 one-shot calls\n',
  )
} finally {
  for (const bridge of bridges) if (!bridge.killed) bridge.kill()
  if (server) await close(server)
  await rm(tempRoot, { recursive: true, force: true })
}

function runOneShot(adminUrl, argumentsPath) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      [
        oneShotPath,
        '--name',
        'ui_import_desktop_task',
        '--arguments-file',
        argumentsPath,
        '--workspace-root',
        repoRoot,
        '--timeout-ms',
        '5000',
      ],
      {
        cwd: repoRoot,
        env: {
          ...process.env,
          ELON_NODE_ADMIN_URL: adminUrl,
          ELON_UI_MCP_AUTOSTART: '0',
        },
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    )
    const stdout = []
    const stderr = []
    child.stdout.on('data', (chunk) => stdout.push(chunk))
    child.stderr.on('data', (chunk) => stderr.push(chunk))
    child.once('error', reject)
    child.once('exit', (code) => {
      const output = Buffer.concat(stdout).toString('utf8').trim()
      const error = Buffer.concat(stderr).toString('utf8').trim()
      if (code !== 0) return reject(new Error(`one-shot exited ${code}: ${error}`))
      try {
        resolve(JSON.parse(output))
      } catch (parseError) {
        reject(new Error(`invalid one-shot JSON: ${output}; ${parseError}`))
      }
    })
  })
}

function startBridge(adminUrl, scanPort = null) {
  const env = {
    ...process.env,
    CODEX_WORKSPACE_ROOT: repoRoot,
    ELON_UI_MCP_AUTOSTART: '0',
    ELON_UI_MCP_RECONNECT_TIMEOUT_MS: '5000',
    ELON_UI_MCP_RECONNECT_POLL_MS: '50',
  }
  if (adminUrl) env.ELON_NODE_ADMIN_URL = adminUrl
  else delete env.ELON_NODE_ADMIN_URL
  if (scanPort) {
    env.ELON_UI_MCP_PORT_FIRST = String(scanPort)
    env.ELON_UI_MCP_PORT_LIMIT = '0'
  }
  const child = spawn(process.execPath, [bridgePath], {
    cwd: repoRoot,
    env,
    stdio: ['pipe', 'pipe', 'pipe'],
  })
  const messages = []
  const waiters = []
  let stderr = ''
  readline.createInterface({ input: child.stdout, crlfDelay: Infinity }).on('line', (line) => {
    const message = JSON.parse(line)
    const waiterIndex = waiters.findIndex((waiter) => waiter.predicate(message))
    if (waiterIndex >= 0) {
      const [waiter] = waiters.splice(waiterIndex, 1)
      clearTimeout(waiter.timer)
      waiter.resolve(message)
    } else {
      messages.push(message)
    }
  })
  child.stderr.on('data', (chunk) => {
    stderr += chunk.toString()
  })
  return {
    child,
    send(message) {
      child.stdin.write(`${JSON.stringify(message)}\n`)
    },
    next(predicate, timeoutMs = 5_000) {
      const existingIndex = messages.findIndex(predicate)
      if (existingIndex >= 0) return Promise.resolve(messages.splice(existingIndex, 1)[0])
      return new Promise((resolve, reject) => {
        const waiter = { predicate, resolve, timer: null }
        waiter.timer = setTimeout(() => {
          const index = waiters.indexOf(waiter)
          if (index >= 0) waiters.splice(index, 1)
          reject(new Error(`Timed out waiting for bridge response. stderr=${stderr}`))
        }, timeoutMs)
        waiters.push(waiter)
      })
    },
  }
}

function createFakeNode(configPath) {
  return http.createServer(async (request, response) => {
    if (request.method === 'GET' && request.url === '/api/status') {
      return json(response, 200, { ok: true, local_admin_token_header: 'x-elon-local-token' })
    }
    if (request.method === 'POST' && request.url === '/api/android-live/project-mcp/bootstrap') {
      return json(response, 200, {
        ok: true,
        mcp: { configPath, sessionId: 'live_test' },
      })
    }
    if (request.method === 'POST' && request.url?.startsWith('/api/android-live/mcp/live_test')) {
      const body = JSON.parse(await readBody(request))
      if (body.method === 'initialize') {
        return json(response, 200, {
          jsonrpc: '2.0',
          id: body.id,
          result: {
            protocolVersion: '2025-03-26',
            capabilities: { tools: { listChanged: false } },
            serverInfo: { name: 'fake-yilong-ui-live', version: '1.0.0' },
          },
        })
      }
      if (body.method === 'tools/list') {
        return json(response, 200, {
          jsonrpc: '2.0',
          id: body.id,
          result: {
            tools: [
              {
                name: 'ui_fake_remote',
                description: 'test tool',
                inputSchema: { type: 'object', properties: {} },
              },
            ],
          },
        })
      }
      if (body.method === 'tools/call') {
        return json(response, 200, {
          jsonrpc: '2.0',
          id: body.id,
          result: {
            content: [{ type: 'text', text: JSON.stringify(body.params) }],
            structuredContent: {
              tool: body.params.name,
              arguments: body.params.arguments,
            },
            isError: false,
          },
        })
      }
    }
    json(response, 404, { error: 'not found' })
  })
}

function json(response, status, value) {
  response.writeHead(status, { 'content-type': 'application/json' })
  response.end(JSON.stringify(value))
}

async function readBody(request) {
  const chunks = []
  for await (const chunk of request) chunks.push(chunk)
  return Buffer.concat(chunks).toString('utf8')
}

function listen(target, port) {
  return new Promise((resolve, reject) => {
    target.once('error', reject)
    target.listen(port, '127.0.0.1', () => resolve(target))
  })
}

function close(target) {
  return new Promise((resolve, reject) => {
    target.close((error) => (error ? reject(error) : resolve()))
  })
}
