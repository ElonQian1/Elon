#!/usr/bin/env node

import readline from 'node:readline'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { readFileSync, statSync } from 'node:fs'
import path from 'node:path'

const options = parseArgs(process.argv.slice(2))
if (options.validateManifestOnly) {
  process.stdout.write(`${JSON.stringify({
    schema: 'elon.project_memory_ab_manifest_validation.v1',
    benchmark_key: options.benchmarkKey,
    benchmark_protocol_verified: Boolean(options.benchmarkProtocol),
    project_root: options.projectRoot,
  })}\n`)
  process.exit(0)
}
const adminUrl = await resolveNodeAdminUrl(options.adminUrl)
const start = await post('/api/project-docs/native-context/observation/start', {
  project_root: options.projectRoot,
  benchmark_key: options.benchmarkKey,
  measurement_window: options.measurementWindow,
  session_id: options.sessionId,
  benchmark_protocol: options.benchmarkProtocol,
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
process.stdout.write(`${JSON.stringify({
  ...completed,
  accepted_event_count: accepted,
  benchmark_protocol_verified: Boolean(options.benchmarkProtocol),
})}\n`)

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
  const supported = new Set([
    '--project-root', '--benchmark-key', '--benchmark-manifest', '--task-file', '--model-id',
    '--codex-build', '--window', '--session-id', '--node-admin-url', '--selected-memory-count',
    '--returned-metadata-bytes', '--validate-manifest-only',
  ])
  for (let index = 0; index < args.length; index += 1) {
    const key = args[index]
    if (!supported.has(key)) throw new Error(`Unsupported argument ${key || '<empty>'}`)
    if (key === '--validate-manifest-only') {
      values.set(key, 'true')
      continue
    }
    const value = args[index + 1]
    if (!key?.startsWith('--') || value === undefined || value.startsWith('--')) {
      throw new Error(`Missing value for ${key || 'argument'}`)
    }
    values.set(key, value)
    index += 1
  }
  const manifestPath = values.get('--benchmark-manifest') || ''
  const benchmarkProtocol = manifestPath ? validateBenchmarkManifest({
    manifestPath,
    requestedProjectRoot: values.get('--project-root') || '',
    taskFile: values.get('--task-file') || '',
    modelId: values.get('--model-id') || '',
    codexBuild: values.get('--codex-build') || '',
  }) : null
  const projectRoot = benchmarkProtocol?.project_root || values.get('--project-root') || process.cwd()
  const benchmarkKey = benchmarkProtocol?.benchmark_key || values.get('--benchmark-key') || ''
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
    benchmarkProtocol,
    validateManifestOnly: values.get('--validate-manifest-only') === 'true',
  }
}

function validateBenchmarkManifest({ manifestPath, requestedProjectRoot, taskFile, modelId, codexBuild }) {
  if (!taskFile) throw new Error('--task-file is required with --benchmark-manifest')
  if (!modelId) throw new Error('--model-id is required with --benchmark-manifest')
  if (!codexBuild) throw new Error('--codex-build is required with --benchmark-manifest')
  const resolvedManifestPath = path.resolve(manifestPath)
  if (statSync(resolvedManifestPath).size > 64 * 1024) throw new Error('Benchmark manifest exceeds 64 KiB')
  const manifest = JSON.parse(readFileSync(resolvedManifestPath, 'utf8'))
  const required = ['benchmark_key', 'case_id', 'model_id', 'task_sha256', 'git_head', 'codex_build', 'project_root', 'manifest_sha256']
  if (manifest?.schema !== 'elon.project_memory_ab_manifest.v1') throw new Error('Unsupported benchmark manifest schema')
  for (const field of required) if (typeof manifest[field] !== 'string' || !manifest[field]) throw new Error(`Benchmark manifest is missing ${field}`)
  if (!/^[A-Za-z0-9._-]{1,64}$/.test(manifest.case_id)) throw new Error('Benchmark manifest case_id is invalid')
  if (!manifestTextIsBounded(manifest.model_id, 80) || !manifestTextIsBounded(manifest.codex_build, 80)) {
    throw new Error('Benchmark manifest model_id or codex_build is invalid')
  }
  if (!/^[0-9a-f]{64}$/.test(manifest.task_sha256) || !/^[0-9a-f]{64}$/.test(manifest.manifest_sha256)) {
    throw new Error('Benchmark manifest SHA-256 field is invalid')
  }
  if (!/^[0-9a-f]{40,64}$/.test(manifest.git_head) || !/^pmab-[0-9a-f]{32}$/.test(manifest.benchmark_key)) {
    throw new Error('Benchmark manifest Git head or benchmark key is invalid')
  }
  if (!manifestTextIsBounded(manifest.project_root, 1024)) throw new Error('Benchmark manifest project_root is invalid')
  if (manifest.tracked_worktree_clean !== true || manifest.stores_task_text !== false) {
    throw new Error('Benchmark manifest must bind a clean worktree and must not store task text')
  }
  if (manifest.model_id !== modelId || manifest.codex_build !== codexBuild) {
    throw new Error('Benchmark model or Codex build does not match the manifest')
  }
  const projectRoot = path.resolve(manifest.project_root)
  if (requestedProjectRoot && normalizedPath(requestedProjectRoot) !== normalizedPath(projectRoot)) {
    throw new Error('--project-root does not match the benchmark manifest')
  }
  const taskPath = path.resolve(taskFile)
  const taskSize = statSync(taskPath).size
  if (taskSize < 1 || taskSize > 1024 * 1024) throw new Error('Benchmark task file must contain 1 byte to 1 MiB')
  const taskSha256 = createHash('sha256').update(readFileSync(taskPath)).digest('hex')
  if (taskSha256 !== manifest.task_sha256) throw new Error('Benchmark task file does not match the manifest')
  const gitHead = git(projectRoot, ['rev-parse', 'HEAD']).toLowerCase()
  if (gitHead !== manifest.git_head.toLowerCase()) throw new Error('Current Git HEAD does not match the benchmark manifest')
  if (git(projectRoot, ['status', '--porcelain', '--untracked-files=no'])) {
    throw new Error('Benchmark observer requires a clean tracked worktree')
  }
  const canonical = [
    `schema=${manifest.schema}`,
    `case_id=${manifest.case_id}`,
    `model_id=${manifest.model_id}`,
    `task_sha256=${manifest.task_sha256}`,
    `git_head=${manifest.git_head.toLowerCase()}`,
    `codex_build=${manifest.codex_build}`,
  ].join('\n')
  const manifestSha256 = createHash('sha256').update(canonical, 'utf8').digest('hex')
  const benchmarkKey = `pmab-${manifestSha256.slice(0, 32)}`
  if (manifestSha256 !== manifest.manifest_sha256 || benchmarkKey !== manifest.benchmark_key) {
    throw new Error('Benchmark manifest integrity check failed')
  }
  return {
    schema: manifest.schema,
    benchmark_key: benchmarkKey,
    case_id: bounded(manifest.case_id, 64),
    model_id: bounded(manifest.model_id, 80),
    task_sha256: manifest.task_sha256,
    git_head: manifest.git_head.toLowerCase(),
    codex_build: bounded(manifest.codex_build, 80),
    manifest_sha256: manifestSha256,
    project_root: projectRoot,
  }
}

function git(projectRoot, args) {
  return execFileSync('git', ['-C', projectRoot, ...args], {
    encoding: 'utf8',
    windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim()
}

function normalizedPath(value) {
  const resolved = path.resolve(value).replace(/[\\/]+$/, '')
  return process.platform === 'win32' ? resolved.toLowerCase() : resolved
}

function manifestTextIsBounded(value, limit) {
  return typeof value === 'string' && value.length >= 1 && value.length <= limit && !/[\r\n]/.test(value)
}

function boundedNumber(value, limit) {
  const number = Number(value || 0)
  return Number.isFinite(number) && number >= 0 ? Math.min(Math.floor(number), limit) : 0
}

function bounded(value, limit) {
  return String(value || '').replace(/[\r\n]+/g, ' ').slice(0, limit)
}
