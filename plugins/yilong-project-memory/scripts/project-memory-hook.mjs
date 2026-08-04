#!/usr/bin/env node

import { createHash } from 'node:crypto'
import { promises as fs } from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const MAX_PATHS = 48
const MAX_CONTINUATIONS = 3
const MAX_PROMPT_PATH_CHARS = 360
const input = await readStdin(256 * 1024)
const event = JSON.parse(input || '{}')
const sessionId = bounded(event.session_id, 200)
const cwd = bounded(event.cwd, 1000)
if (!sessionId || !cwd) process.exit(0)

const pluginDataRoot = process.env.PLUGIN_DATA || path.join(os.tmpdir(), 'yilong-project-memory-plugin')
const dataRoot = path.join(pluginDataRoot, 'session-ledgers')
await fs.mkdir(dataRoot, { recursive: true })
await cleanupOldLedgers(dataRoot)
const ledgerPath = path.join(dataRoot, `${fingerprint(`${cwd}\0${sessionId}`)}.json`)
const eventName = bounded(event.hook_event_name, 40)

if (eventName === 'SessionEnd') {
  await fs.rm(ledgerPath, { force: true })
  process.stdout.write('{}\n')
  process.exit(0)
}

const ledger = await readLedger(ledgerPath)
if (eventName === 'PostToolUse') {
  for (const item of extractPathAccesses(event.tool_name, event.tool_input, cwd)) {
    ledger.paths[item.path] = strongestAccess(ledger.paths[item.path], item.access)
  }
  const entries = Object.entries(ledger.paths).slice(-MAX_PATHS)
  ledger.paths = Object.fromEntries(entries)
  ledger.updated_at_ms = Date.now()
  await writeLedger(ledgerPath, ledger)
  process.exit(0)
}

if (eventName === 'Stop') {
  const paths = Object.keys(ledger.paths).sort()
  const readCount = Object.values(ledger.paths).filter((access) => access === 'read').length
  const snapshot = fingerprint(paths.join('\0'))
  const selected = selectPromptPaths(paths)
  if (
    paths.length < 2 ||
    readCount < 1 ||
    selected.length < 1 ||
    ledger.continuations >= MAX_CONTINUATIONS ||
    ledger.last_snapshot === snapshot
  ) {
    process.stdout.write('{}\n')
    process.exit(0)
  }
  ledger.continuations += 1
  ledger.last_snapshot = snapshot
  ledger.updated_at_ms = Date.now()
  await writeLedger(ledgerPath, ledger)
  const reason = [
    'Before finishing, consider whether this task produced a durable, non-obvious project navigation fact.',
    'If and only if it is reusable, evidence-backed, non-duplicative, and not merely task-local, call project_docs_record_native_context_receipt.',
    `Use only current relative evidence paths from this bounded set: ${selected.join(', ')}`,
    'Do not include source bodies, commands, prompts, chat text, tool output, or Codex private memories. If there is no qualifying fact, finish now.',
  ].join(' ')
  process.stdout.write(`${JSON.stringify({ decision: 'block', reason })}\n`)
  process.exit(0)
}

process.stdout.write('{}\n')

function extractPathAccesses(toolNameValue, toolInput, root) {
  const toolName = bounded(toolNameValue, 120)
  const access = /apply_patch|write|edit|create|delete|move|copy/i.test(toolName)
    ? 'write'
    : /read|view|search|find|list|open/i.test(toolName) ? 'read' : 'access'
  const candidates = []
  collectExplicitPaths(toolInput, candidates)
  if (/apply_patch/i.test(toolName)) {
    const patchText = typeof toolInput === 'string'
      ? toolInput
      : String(toolInput?.patch || toolInput?.input || toolInput?.command || '')
    for (const match of patchText.matchAll(/^\*\*\* (?:Add|Update|Delete) File: (.+)$/gm)) {
      candidates.push(match[1])
    }
  }
  return [...new Set(candidates.map((value) => normalizeRelativePath(value, root)).filter(Boolean))]
    .slice(0, 16)
    .map((relativePath) => ({ path: relativePath, access }))
}

function collectExplicitPaths(value, output, key = '') {
  if (output.length >= 32 || value === null || value === undefined) return
  if (typeof value === 'string') {
    if (/^(path|file|file_path|filename|target_path|source_path)$/i.test(key)) output.push(value)
    return
  }
  if (Array.isArray(value)) {
    for (const entry of value.slice(0, 32)) collectExplicitPaths(entry, output, key)
    return
  }
  if (typeof value === 'object') {
    for (const [childKey, childValue] of Object.entries(value).slice(0, 64)) {
      collectExplicitPaths(childValue, output, childKey)
    }
  }
}

function normalizeRelativePath(value, root) {
  const candidate = bounded(value, 1000).replace(/^['"]|['"]$/g, '')
  if (!candidate || candidate.includes('\0')) return ''
  const absolute = path.isAbsolute(candidate) ? path.resolve(candidate) : path.resolve(root, candidate)
  const relative = path.relative(path.resolve(root), absolute).replaceAll('\\', '/')
  if (!relative || relative === '.' || relative.startsWith('../') || path.isAbsolute(relative)) return ''
  if (ignoredPath(relative) || !allowedPath(relative)) return ''
  return relative.slice(0, 500)
}

function strongestAccess(current, next) {
  if (current === 'write' || next === 'write') return 'write'
  if (current === 'read' || next === 'read') return 'read'
  return 'access'
}

function selectPromptPaths(paths) {
  const selected = []
  let used = 0
  for (const candidate of paths) {
    const separator = selected.length ? 2 : 0
    if (used + separator + candidate.length > MAX_PROMPT_PATH_CHARS) continue
    selected.push(candidate)
    used += separator + candidate.length
    if (selected.length >= 6) break
  }
  return selected
}

function ignoredPath(value) {
  const lower = value.toLowerCase()
  return lower === '.git' || lower.startsWith('.git/')
    || lower === '.env' || lower.endsWith('/.env')
    || lower.startsWith('node_modules/') || lower.includes('/node_modules/')
    || lower.startsWith('target/') || lower.includes('/target/')
    || lower.startsWith('.ai-tmp/')
    || lower.endsWith('cargo.lock') || lower.endsWith('package-lock.json')
    || lower.endsWith('pnpm-lock.yaml') || lower.endsWith('yarn.lock')
    || lower.includes('credential') || lower.includes('secret') || lower.includes('private_key')
}

function allowedPath(value) {
  const lower = value.toLowerCase()
  const name = lower.split('/').at(-1)
  if (['agents.md', 'codex.md', 'readme', 'readme.md', 'makefile', 'dockerfile', 'cargo.toml', 'package.json', 'tsconfig.json'].includes(name)) return true
  return ['.rs', '.toml', '.md', '.json', '.yaml', '.yml', '.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.kt', '.kts', '.java', '.py', '.go', '.cs', '.cpp', '.c', '.h', '.html', '.css', '.scss', '.sql', '.proto', '.ps1', '.sh']
    .some((extension) => lower.endsWith(extension))
}

async function readLedger(filePath) {
  try {
    const value = JSON.parse(await fs.readFile(filePath, 'utf8'))
    return {
      schema: 'elon.project_memory_plugin_ledger.v1',
      paths: value?.paths && typeof value.paths === 'object' ? value.paths : {},
      continuations: Number.isInteger(value?.continuations) ? value.continuations : 0,
      last_snapshot: bounded(value?.last_snapshot, 64),
      updated_at_ms: Number(value?.updated_at_ms) || Date.now(),
    }
  } catch {
    return {
      schema: 'elon.project_memory_plugin_ledger.v1',
      paths: {},
      continuations: 0,
      last_snapshot: '',
      updated_at_ms: Date.now(),
    }
  }
}

async function writeLedger(filePath, value) {
  const temporary = `${filePath}.${process.pid}.tmp`
  await fs.writeFile(temporary, JSON.stringify(value), { encoding: 'utf8', mode: 0o600 })
  await fs.rename(temporary, filePath)
}

async function cleanupOldLedgers(root) {
  const cutoff = Date.now() - 24 * 60 * 60 * 1000
  const names = await fs.readdir(root).catch(() => [])
  await Promise.all(names.slice(0, 256).filter((name) => name.endsWith('.json')).map(async (name) => {
    const filePath = path.join(root, name)
    const stat = await fs.stat(filePath).catch(() => null)
    if (stat && stat.mtimeMs < cutoff) await fs.rm(filePath, { force: true })
  }))
}

async function readStdin(limit) {
  const chunks = []
  let length = 0
  for await (const chunk of process.stdin) {
    length += chunk.length
    if (length > limit) throw new Error('Hook input exceeded the bounded input limit')
    chunks.push(chunk)
  }
  return Buffer.concat(chunks).toString('utf8')
}

function fingerprint(value) {
  return createHash('sha256').update(value).digest('hex').slice(0, 32)
}

function bounded(value, limit) {
  return String(value ?? '').replace(/[\r\n]+/g, ' ').slice(0, limit)
}
