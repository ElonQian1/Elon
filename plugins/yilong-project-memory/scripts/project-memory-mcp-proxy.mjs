#!/usr/bin/env node

import readline from 'node:readline'
import { promises as fs } from 'node:fs'
import path from 'node:path'

const profile = process.argv[2]
if (!['context', 'receipt'].includes(profile)) {
  throw new Error('Expected project memory MCP profile: context or receipt')
}

const adminUrl = await resolveNodeAdminUrl()
const projectRoot = await resolveProjectRoot()
const descriptor = await postJson(`${adminUrl}/api/project-docs/mcp/bootstrap`, {
  projectRoot,
  profile,
})
if (!descriptor?.ok || !descriptor?.mcp?.url) {
  throw new Error(descriptor?.error || 'The local Yilong node did not return an MCP descriptor')
}
const mcpUrl = descriptor.mcp.url
const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity })
let queue = Promise.resolve()
input.on('line', (line) => {
  if (!line.trim()) return
  queue = queue.then(() => forward(line)).catch((error) => {
    process.stderr.write(`yilong-project-memory: ${safeError(error)}\n`)
  })
})
await new Promise((resolve) => input.once('close', resolve))
await queue

async function forward(line) {
  const message = JSON.parse(line)
  const response = await fetch(mcpUrl, {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'application/json' },
    body: JSON.stringify(message),
    signal: AbortSignal.timeout(65_000),
  })
  if (response.status === 202 || response.status === 204) return
  const body = await response.text()
  if (!response.ok) throw new Error(`MCP HTTP ${response.status}: ${bounded(body, 240)}`)
  if (body.trim()) process.stdout.write(`${body.trim()}\n`)
}

async function resolveNodeAdminUrl() {
  const configured = String(process.env.ELON_NODE_ADMIN_URL || '').replace(/\/$/, '')
  if (configured) return configured
  for (let port = 7799; port <= 7819; port += 1) {
    const candidate = `http://127.0.0.1:${port}`
    try {
      const response = await fetch(`${candidate}/health`, { signal: AbortSignal.timeout(800) })
      if (response.ok) return candidate
    } catch {
      // Continue bounded loopback discovery without logging local responses.
    }
  }
  throw new Error('No local Yilong node admin API was found on ports 7799-7819')
}

async function resolveProjectRoot() {
  const configured = String(process.env.ELON_PROJECT_ROOT || '').trim()
  if (configured) return path.resolve(configured)
  let current = path.resolve(process.cwd())
  while (true) {
    const gitMarker = await fs.stat(path.join(current, '.git')).catch(() => null)
    if (gitMarker) return current
    const parent = path.dirname(current)
    if (parent === current) break
    current = parent
  }
  throw new Error('No Git project root was found from the MCP process cwd; set ELON_PROJECT_ROOT')
}

async function postJson(url, body) {
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'application/json' },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(10_000),
  })
  const value = await response.json().catch(() => ({}))
  if (!response.ok) throw new Error(value?.error || `Bootstrap HTTP ${response.status}`)
  return value
}

function safeError(error) {
  return bounded(error instanceof Error ? error.message : String(error), 300)
}

function bounded(value, limit) {
  return String(value).replace(/[\r\n]+/g, ' ').slice(0, limit)
}
