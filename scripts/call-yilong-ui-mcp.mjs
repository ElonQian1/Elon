#!/usr/bin/env node

import { spawn } from 'node:child_process'
import { access, readFile } from 'node:fs/promises'
import path from 'node:path'
import readline from 'node:readline'

try {
  await main()
} catch (error) {
  process.stderr.write(`${errorText(error)}\n`)
  process.exitCode = 1
}

async function main() {
  const options = parseOptions(process.argv.slice(2))
  const bridgePath = path.join(import.meta.dirname, 'yilong-ui-mcp-bridge.mjs')
  const workspaceRoot = path.resolve(options.workspaceRoot || process.cwd())
  const argumentsPath = path.resolve(options.argumentsFile)
  await access(bridgePath)
  await access(workspaceRoot)
  const argumentsText = (await readFile(argumentsPath, 'utf8')).replace(/^\uFEFF/, '')
  const argumentsValue = JSON.parse(argumentsText)
  if (!isPlainObject(argumentsValue)) fail('--arguments-file 必须是 UTF-8 JSON 对象')

  const child = spawn(process.execPath, [bridgePath], {
    cwd: workspaceRoot,
    env: { ...process.env, CODEX_WORKSPACE_ROOT: workspaceRoot },
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true,
  })
  const client = createClient(child, options.timeoutMs)

  try {
    client.send({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {} })
    await client.next((message) => message.id === 1)
    client.send({
      jsonrpc: '2.0',
      id: 2,
      method: 'tools/call',
      params: {
        name: 'ui_bridge_proxy',
        arguments: {
          name: options.name,
          arguments: argumentsValue,
          waitMs: options.timeoutMs,
        },
      },
    })
    const response = await client.next((message) => message.id === 2)
    if (response.error) fail(response.error.message || 'UI MCP 调用失败')
    if (response.result?.isError) {
      fail(
        response.result.content?.map((item) => item.text).filter(Boolean).join('\n') ||
          'UI MCP 工具返回错误',
      )
    }
    process.stdout.write(`${JSON.stringify(response.result?.structuredContent ?? response.result)}\n`)
  } finally {
    child.stdin.end()
    if (!child.killed) child.kill()
  }
}

function parseOptions(args) {
  const values = {}
  const allowed = new Set(['name', 'arguments-file', 'workspace-root', 'timeout-ms'])
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index]
    const value = args[index + 1]
    if (!key?.startsWith('--') || value === undefined) usage()
    const option = key.slice(2)
    if (!allowed.has(option)) fail(`未知参数: ${key}`)
    values[option] = value
  }
  const name = values.name?.trim()
  if (!/^ui_[a-z0-9_]{1,80}$/.test(name || '') || name.startsWith('ui_bridge_')) {
    fail('--name 必须是非 bridge 的 ui_* 工具名')
  }
  if (!values['arguments-file']) fail('缺少 --arguments-file')
  const timeoutMs = Number.parseInt(values['timeout-ms'] || '45000', 10)
  if (!Number.isFinite(timeoutMs) || timeoutMs < 1_000 || timeoutMs > 120_000) {
    fail('--timeout-ms 必须为 1000..120000')
  }
  return {
    name,
    argumentsFile: values['arguments-file'],
    workspaceRoot: values['workspace-root'],
    timeoutMs,
  }
}

function createClient(child, timeoutMs) {
  const messages = []
  const waiters = []
  let stderr = ''
  let terminalError = null
  readline.createInterface({ input: child.stdout, crlfDelay: Infinity }).on('line', (line) => {
    try {
      const message = JSON.parse(line)
      const index = waiters.findIndex((waiter) => waiter.predicate(message))
      if (index < 0) return messages.push(message)
      const [waiter] = waiters.splice(index, 1)
      clearTimeout(waiter.timer)
      waiter.resolve(message)
    } catch (error) {
      rejectAll(new Error(`UI MCP bridge 返回了无效 JSON: ${errorText(error)}`))
    }
  })
  child.stderr.on('data', (chunk) => {
    stderr += chunk.toString('utf8')
  })
  child.once('error', rejectAll)
  child.once('exit', (code) => {
    if (code !== 0) rejectAll(new Error(`UI MCP bridge 已退出（${code}）。${stderr}`))
  })

  function rejectAll(error) {
    terminalError ||= error
    for (const waiter of waiters.splice(0)) {
      clearTimeout(waiter.timer)
      waiter.reject(terminalError)
    }
  }

  return {
    send(message) {
      child.stdin.write(`${JSON.stringify(message)}\n`)
    },
    next(predicate) {
      if (terminalError) return Promise.reject(terminalError)
      const index = messages.findIndex(predicate)
      if (index >= 0) return Promise.resolve(messages.splice(index, 1)[0])
      return new Promise((resolve, reject) => {
        const waiter = { predicate, resolve, reject, timer: null }
        waiter.timer = setTimeout(() => {
          const pendingIndex = waiters.indexOf(waiter)
          if (pendingIndex >= 0) waiters.splice(pendingIndex, 1)
          reject(new Error(`等待 UI MCP 响应超时。${stderr}`))
        }, timeoutMs)
        waiters.push(waiter)
      })
    },
  }
}

function isPlainObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function usage() {
  fail(
    '用法: node scripts/call-yilong-ui-mcp.mjs --name <ui_tool> --arguments-file <utf8.json> [--workspace-root <path>] [--timeout-ms <ms>]',
  )
}

function fail(message) {
  throw new Error(message)
}

function errorText(error) {
  return error instanceof Error ? error.message : String(error)
}
