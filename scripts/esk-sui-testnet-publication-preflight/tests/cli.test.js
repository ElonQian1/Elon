'use strict'

const test = require('node:test')
const assert = require('node:assert/strict')
const { mkdirSync, mkdtempSync, writeFileSync, rmSync, symlinkSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { spawnSync } = require('node:child_process')
const { releaseCandidate, reverseObjectKeys } = require('./fixtures')

const CLI = join(__dirname, '../../prepare-esk-sui-testnet-publication.js')
const GUARD = join(__dirname, 'no-network-guard.js')
const ERROR_PREFIX = 'ESK_SUI_TESTNET_PUBLICATION_PREFLIGHT_ERROR='

function spawnCli(args) {
  return spawnSync(process.execPath, ['--require', GUARD, CLI, ...args], {
    encoding: 'utf8', timeout: 5000, env: {},
  })
}

function assertFixedError(result, expected) {
  assert.equal(result.status, 1)
  assert.equal(result.stdout, '')
  assert.match(result.stderr, new RegExp(`^${ERROR_PREFIX}${expected}\\r?\\n$`))
}

function walk(value, visit, path = []) {
  visit(value, path)
  if (Array.isArray(value)) value.forEach((item, index) => walk(item, visit, [...path, index]))
  else if (value && typeof value === 'object') {
    for (const [key, item] of Object.entries(value)) walk(item, visit, [...path, key])
  }
}

test('template CLI prints one inert machine JSON document', () => {
  const result = spawnCli(['template'])
  assert.equal(result.status, 0, result.stderr)
  assert.equal(result.stderr, '')
  const output = JSON.parse(result.stdout)
  assert.equal(output.schema, 'yilong.esk.sui.testnet_publication_candidate.v1')
  assert.equal(output.scope.network, 'testnet')
  assert.equal(output.scope.mode, 'template')
  assert.ok(Object.values(output.roles).every(value => value === null))
  assert.ok(Object.values(output.gas_budgets).every(value => value === null))
  assert.ok(Object.values(output.approvals).every(approval =>
    Object.values(approval).every(value => value === null)))
  walk(output, (value, path) => {
    const key = path.at(-1)
    if (typeof key === 'string' && /address$/.test(key)) assert.equal(value, null, path.join('.'))
  })
})

test('CLI rejects unsupported commands and argument counts with a fixed code', () => {
  for (const args of [[], ['--help'], ['unknown'], ['preflight'],
    ['preflight', 'a.json', 'b.json'], ['template', 'a.json']]) {
    assertFixedError(spawnCli(args), 'USAGE')
  }
})

test('CLI preflight is canonical across reordered and whitespace-varied release input', () => {
  const dir = mkdtempSync(join(tmpdir(), 'esk-publication-preflight-'))
  try {
    const compactPath = join(dir, 'compact.json')
    const spacedPath = join(dir, 'spaced.json')
    const candidate = releaseCandidate()
    writeFileSync(compactPath, JSON.stringify(candidate))
    writeFileSync(spacedPath, JSON.stringify(reverseObjectKeys(candidate), null, 4))
    const compact = spawnCli(['preflight', compactPath])
    const spaced = spawnCli(['preflight', spacedPath])
    assert.equal(compact.status, 0, compact.stderr)
    assert.equal(spaced.status, 0, spaced.stderr)
    assert.equal(compact.stderr, '')
    assert.equal(spaced.stderr, '')
    const left = JSON.parse(compact.stdout)
    const right = JSON.parse(spaced.stdout)
    assert.equal(left.candidate_status, 'prepared_not_authorized')
    assert.equal(left.plan_sha256, right.plan_sha256)
    assert.deepEqual(left, right)
  } finally { rmSync(dir, { recursive: true, force: true }) }
})

test('CLI does not echo secret fields, secret values, addresses, paths or parser errors', () => {
  const dir = mkdtempSync(join(tmpdir(), 'esk-publication-preflight-'))
  try {
    const secret = 'NEVER_PRINT_THIS_SECRET'
    const address = `0x${'d'.repeat(64)}`
    const file = join(dir, `${secret}.json`)
    writeFileSync(file, JSON.stringify({ private_key: secret, address }))
    const result = spawnCli(['preflight', file])
    assertFixedError(result, 'SECRET_MATERIAL_REJECTED')
    const combined = result.stdout + result.stderr
    assert.doesNotMatch(combined, new RegExp(secret))
    assert.doesNotMatch(combined, new RegExp(address))
    assert.equal(combined.includes(file), false)

    writeFileSync(file, `{"broken":"${secret}"`)
    const malformed = spawnCli(['preflight', file])
    assertFixedError(malformed, 'INVALID_JSON')
    assert.equal((malformed.stdout + malformed.stderr).includes(secret), false)
  } finally { rmSync(dir, { recursive: true, force: true }) }
})

test('CLI rejects duplicate keys, BOM and oversized input before permissive parsing', () => {
  const dir = mkdtempSync(join(tmpdir(), 'esk-publication-preflight-'))
  try {
    const file = join(dir, 'candidate.json')
    writeFileSync(file, '{"schema":"one","\\u0073chema":"two"}')
    assertFixedError(spawnCli(['preflight', file]), 'DUPLICATE_JSON_KEY')

    writeFileSync(file, `\ufeff{"schema":"x"}`)
    assertFixedError(spawnCli(['preflight', file]), 'INVALID_UTF8')

    writeFileSync(file, 'x'.repeat(128 * 1024 + 1))
    assertFixedError(spawnCli(['preflight', file]), 'INPUT_TOO_LARGE')
  } finally { rmSync(dir, { recursive: true, force: true }) }
})

test('CLI rejects symbolic-link input instead of following it', t => {
  const dir = mkdtempSync(join(tmpdir(), 'esk-publication-preflight-'))
  try {
    const target = join(dir, 'target.json')
    const link = join(dir, 'link.json')
    writeFileSync(target, '{}')
    try { symlinkSync(target, link, 'file') } catch (error) {
      if (error && ['EPERM', 'EACCES'].includes(error.code)) return t.skip('symlink unavailable')
      throw error
    }
    assertFixedError(spawnCli(['preflight', link]), 'INPUT_NOT_REGULAR_FILE')
  } finally { rmSync(dir, { recursive: true, force: true }) }
})

test('CLI rejects network and Windows device paths without opening them', () => {
  for (const value of [
    '\\\\example.invalid\\share\\candidate.json',
    '//example.invalid/share/candidate.json',
    '\\\\?\\UNC\\example.invalid\\share\\candidate.json',
    '\\\\.\\pipe\\candidate',
  ]) assertFixedError(spawnCli(['preflight', value]), 'INVALID_INPUT_PATH')
})

test('CLI rejects Windows alternate data stream paths', t => {
  if (process.platform !== 'win32') return t.skip('Windows ADS is unavailable')
  const dir = mkdtempSync(join(tmpdir(), 'esk-publication-preflight-'))
  try {
    const file = join(dir, 'candidate.json')
    writeFileSync(file, JSON.stringify(releaseCandidate()))
    for (const suffix of ['::$DATA', ':candidate']) {
      assertFixedError(spawnCli(['preflight', `${file}${suffix}`]), 'INVALID_INPUT_PATH')
    }
  } finally { rmSync(dir, { recursive: true, force: true }) }
})

test('CLI rejects candidate files reached through a directory junction', t => {
  if (process.platform !== 'win32') return t.skip('Windows junction is unavailable')
  const dir = mkdtempSync(join(tmpdir(), 'esk-publication-preflight-'))
  try {
    const target = join(dir, 'target')
    const junction = join(dir, 'junction')
    mkdirSync(target)
    writeFileSync(join(target, 'candidate.json'), JSON.stringify(releaseCandidate()))
    try { symlinkSync(target, junction, 'junction') } catch (error) {
      if (error && ['EPERM', 'EACCES'].includes(error.code)) return t.skip('junction unavailable')
      throw error
    }
    assertFixedError(spawnCli(['preflight', join(junction, 'candidate.json')]),
      'INPUT_NOT_REGULAR_FILE')
  } finally { rmSync(dir, { recursive: true, force: true }) }
})
