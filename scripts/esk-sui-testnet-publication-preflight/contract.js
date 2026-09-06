'use strict'

const CANDIDATE_SCHEMA = 'yilong.esk.sui.testnet_publication_candidate.v1'
const PREFLIGHT_SCHEMA = 'yilong.esk.sui.testnet_publication_preflight.v1'
const MAX_CANDIDATE_BYTES = 128 * 1024
const MAX_REPOSITORY_FILE_BYTES = 256 * 1024
const MAX_GAS_BUDGET_MIST = 1_000_000_000n

const SAFE_CODES = new Set([
  'USAGE', 'INVALID_INPUT_PATH', 'INPUT_NOT_REGULAR_FILE', 'INPUT_TOO_LARGE',
  'INVALID_UTF8', 'INVALID_JSON', 'DUPLICATE_JSON_KEY', 'UNKNOWN_FIELD',
  'SECRET_MATERIAL_REJECTED', 'INVALID_CANDIDATE', 'REPOSITORY_DRIFT',
])

class PreflightError extends Error {
  constructor(code) {
    super('candidate rejected')
    this.name = 'PreflightError'
    this.code = SAFE_CODES.has(code) ? code : 'INVALID_CANDIDATE'
  }
}

function fail(code) { throw new PreflightError(code) }

function safeCode(error) {
  return error instanceof PreflightError ? error.code : 'INTERNAL_ERROR'
}

function isPlainObject(value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return false
  const prototype = Object.getPrototypeOf(value)
  return prototype === Object.prototype || prototype === null
}

function exactKeys(value, expected, code = 'INVALID_CANDIDATE') {
  if (!isPlainObject(value)) fail(code)
  const actual = Object.keys(value).sort()
  const wanted = [...expected].sort()
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail(actual.some((key) => !wanted.includes(key)) ? 'UNKNOWN_FIELD' : code)
  }
  return value
}

function literal(value, expected, code = 'INVALID_CANDIDATE') {
  if (value !== expected) fail(code)
  return value
}

function oneOf(value, allowed, code = 'INVALID_CANDIDATE') {
  if (!allowed.includes(value)) fail(code)
  return value
}

function asciiIdentifier(value, pattern, code = 'INVALID_CANDIDATE') {
  if (typeof value !== 'string' || !pattern.test(value)) fail(code)
  return value
}

function digest(value, { allowNull = false, allowZero = false } = {}) {
  if (allowNull && value === null) return null
  if (typeof value !== 'string' || !/^sha256:[0-9a-f]{64}$/.test(value)) {
    fail('INVALID_CANDIDATE')
  }
  if (!allowZero && value === `sha256:${'0'.repeat(64)}`) fail('INVALID_CANDIDATE')
  return value
}

function commit(value) {
  if (typeof value !== 'string' || !/^[0-9a-f]{40}$/.test(value) || /^0+$/.test(value)) {
    fail('INVALID_CANDIDATE')
  }
  return value
}

function decimal(value, { minimum = 0n, maximum = 18_446_744_073_709_551_615n,
  allowNull = false } = {}) {
  if (allowNull && value === null) return null
  if (typeof value !== 'string' || !/^(?:0|[1-9][0-9]*)$/.test(value)) {
    fail('INVALID_CANDIDATE')
  }
  const parsed = BigInt(value)
  if (parsed < minimum || parsed > maximum) fail('INVALID_CANDIDATE')
  return parsed
}

function integer(value, minimum, maximum, { allowNull = false } = {}) {
  if (allowNull && value === null) return null
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    fail('INVALID_CANDIDATE')
  }
  return value
}

function timestamp(value, { allowNull = false } = {}) {
  if (allowNull && value === null) return null
  if (typeof value !== 'string') fail('INVALID_CANDIDATE')
  const match = value.match(/^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\.(\d{3})Z$/)
  if (!match) fail('INVALID_CANDIDATE')
  const milliseconds = Date.parse(value)
  if (!Number.isFinite(milliseconds) || new Date(milliseconds).toISOString() !== value) {
    fail('INVALID_CANDIDATE')
  }
  return milliseconds
}

const SECRET_KEY = /(?:mnemonic|seed(?:_phrase)?|private(?:_key)?|secret|password|passwd|bearer|api[_-]?token|cookie|keystore|signature|signed[_-]?transaction|gas[_-]?coin|transaction[_-]?(?:bytes|data)|ptb[_-]?bytes)/i
const SECRET_VALUE = /(?:-----BEGIN [A-Z ]*PRIVATE KEY-----|\bBearer\s+[A-Za-z0-9._~+/=-]+|\bsuiprivkey1[0-9a-z]+|\bxprv[1-9A-HJ-NP-Za-km-z]+|\b(?:mnemonic|seed phrase|private key|api token|cookie)\s*[:=])/i

function rejectSecretMaterial(value) {
  const visit = (current) => {
    if (typeof current === 'string') {
      if (SECRET_VALUE.test(current)) fail('SECRET_MATERIAL_REJECTED')
      return
    }
    if (Array.isArray(current)) {
      for (const item of current) visit(item)
      return
    }
    if (!isPlainObject(current)) return
    for (const [key, item] of Object.entries(current)) {
      if (SECRET_KEY.test(key)) fail('SECRET_MATERIAL_REJECTED')
      visit(item)
    }
  }
  visit(value)
}

module.exports = {
  CANDIDATE_SCHEMA, PREFLIGHT_SCHEMA, MAX_CANDIDATE_BYTES,
  MAX_REPOSITORY_FILE_BYTES, MAX_GAS_BUDGET_MIST, PreflightError, fail, safeCode,
  isPlainObject, exactKeys, literal, oneOf, asciiIdentifier, digest, commit,
  decimal, integer, timestamp, rejectSecretMaterial,
}
