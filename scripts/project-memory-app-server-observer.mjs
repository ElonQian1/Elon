#!/usr/bin/env node

import readline from 'node:readline'

const options = parseArgs(process.argv.slice(2))
const adminUrl = await resolveNodeAdminUrl(options.adminUrl)
const start = await post('/api/project-docs/native-context/observation/start', {
  project_root: options.projectRoot,
  benchmark_key: options.benchmarkKey,
  measurement_window: options.measurementWindow,
  session_id: options.sessionId,
})
const windowId = start.window_id
let accepted = 0
let queue = Promise.resolve()
const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity })
input.on('line', (line) => {
  if (!line.trim()) return
  queue = queue.then(async () => {
    if (Buffer.byteLength(line, 'utf8') > 128 * 1024) {
      throw new Error('Observation event exceeded the bounded 128 KiB input limit')
    }
    const event = sanitizeEvent(JSON.parse(line))
    if (!event) return
    await post('/api/project-docs/native-context/observation/event', {
      project_root: options.projectRoot,
      window_id: windowId,
      event,
    })
    accepted += 1
  })
})
await new Promise((resolve) => input.once('close', resolve))
await queue
const completed = await post('/api/project-docs/native-context/observation/finish', {
  project_root: options.projectRoot,
  window_id: windowId,
  selected_memory_count: options.selectedMemoryCount,
  returned_metadata_bytes: options.returnedMetadataBytes,
})
process.stdout.write(`${JSON.stringify({ ...completed, accepted_event_count: accepted })}\n`)

function sanitizeEvent(value) {
  const method = String(value?.method || '')
  const accepted = new Set([
    'hook/started',
    'hook/completed',
    'thread/tokenUsage/updated',
    'turn/started',
    'turn/completed',
    'item/started',
    'item/completed',
  ])
  if (!accepted.has(method)) return null
  const event = { method, params: {} }
  if (method === 'thread/tokenUsage/updated') event.params.tokenUsage = tokenCounters(value?.params)
  if (method.startsWith('item/')) {
    event.params.item = {
      type: bounded(value?.params?.item?.type, 80),
      kind: bounded(value?.params?.item?.kind, 80),
      toolName: bounded(value?.params?.item?.toolName || value?.params?.item?.tool_name, 120),
    }
  }
  return event
}

function tokenCounters(value) {
  const result = { input_tokens: 0, cached_input_tokens: 0, output_tokens: 0 }
  visit(value, (key, raw) => {
    const normalized = key.replaceAll('_', '').replaceAll('-', '').toLowerCase()
    const number = Number(raw)
    if (!Number.isFinite(number) || number < 0) return
    if (normalized === 'inputtokens') result.input_tokens = Math.max(result.input_tokens, Math.floor(number))
    if (['cachedinputtokens', 'cachereadinputtokens'].includes(normalized)) {
      result.cached_input_tokens = Math.max(result.cached_input_tokens, Math.floor(number))
    }
    if (normalized === 'outputtokens') result.output_tokens = Math.max(result.output_tokens, Math.floor(number))
  })
  return result
}

function visit(value, callback, depth = 0) {
  if (depth >= 8) return
  if (Array.isArray(value)) {
    for (const entry of value.slice(0, 64)) visit(entry, callback, depth + 1)
  } else if (value && typeof value === 'object') {
    for (const [key, entry] of Object.entries(value).slice(0, 128)) {
      callback(key, entry)
      visit(entry, callback, depth + 1)
    }
  }
}

async function post(route, body) {
  const response = await fetch(`${adminUrl}${route}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'application/json' },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(10_000),
  })
  const envelope = await response.json().catch(() => ({}))
  if (!response.ok || !envelope?.ok || !envelope?.result) {
    throw new Error(envelope?.error || `Observation HTTP ${response.status}`)
  }
  return envelope.result
}

async function resolveNodeAdminUrl(requested) {
  if (requested) return requested.replace(/\/$/, '')
  for (let port = 7799; port <= 7819; port += 1) {
    const candidate = `http://127.0.0.1:${port}`
    try {
      const response = await fetch(`${candidate}/api/health`, { signal: AbortSignal.timeout(800) })
      if (response.ok) return candidate
    } catch {
      // Continue bounded loopback discovery.
    }
  }
  throw new Error('No local Yilong node admin API was found on ports 7799-7819')
}

function parseArgs(args) {
  const values = new Map()
  for (let index = 0; index < args.length; index += 2) values.set(args[index], args[index + 1])
  const projectRoot = values.get('--project-root') || process.cwd()
  const benchmarkKey = values.get('--benchmark-key') || ''
  const measurementWindow = values.get('--window') || ''
  if (!/^[A-Za-z0-9._-]{1,80}$/.test(benchmarkKey)) throw new Error('--benchmark-key is required')
  if (!['baseline_without_project_memory', 'with_project_memory'].includes(measurementWindow)) {
    throw new Error('--window must be baseline_without_project_memory or with_project_memory')
  }
  return {
    projectRoot,
    benchmarkKey,
    measurementWindow,
    sessionId: values.get('--session-id') || `${process.pid}-${Date.now()}`,
    adminUrl: values.get('--node-admin-url') || process.env.ELON_NODE_ADMIN_URL || '',
    selectedMemoryCount: boundedNumber(values.get('--selected-memory-count'), 64),
    returnedMetadataBytes: boundedNumber(values.get('--returned-metadata-bytes'), 4 * 1024 * 1024),
  }
}

function boundedNumber(value, limit) {
  const number = Number(value || 0)
  return Number.isFinite(number) && number >= 0 ? Math.min(Math.floor(number), limit) : 0
}

function bounded(value, limit) {
  return String(value || '').replace(/[\r\n]+/g, ' ').slice(0, limit)
}
