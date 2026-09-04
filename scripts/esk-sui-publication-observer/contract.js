const { isIP } = require('node:net')

const OFFICIAL_TESTNET = 'https://graphql.testnet.sui.io/graphql'
const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
const ERROR_CODES = new Set([
  'INVALID_INPUT', 'INVALID_ENDPOINT', 'PRIVATE_ADDRESS', 'NETWORK_ERROR', 'TIMEOUT',
  'HTTP_ERROR', 'RESPONSE_TOO_LARGE', 'INVALID_RESPONSE', 'GRAPHQL_ERROR',
  'CHAIN_MISMATCH', 'PACKAGE_MISMATCH', 'TRANSACTION_MISMATCH',
  'TRANSACTION_NOT_SUCCESSFUL', 'CHECKPOINT_MISSING', 'SOURCE_DISAGREEMENT',
])

class ObservationError extends Error {
  constructor(code) {
    super(code)
    this.code = ERROR_CODES.has(code) ? code : 'INVALID_RESPONSE'
  }
}

function requireValue(condition, code = 'INVALID_INPUT') {
  if (!condition) throw new ObservationError(code)
}

function digest32(value) {
  if (typeof value !== 'string' || value.length < 32 || value.length > 44) return false
  let number = 0n
  for (const character of value) {
    const digit = ALPHABET.indexOf(character)
    if (digit < 0) return false
    number = number * 58n + BigInt(digit)
  }
  let bytes = 0
  for (let rest = number; rest > 0n; rest >>= 8n) bytes += 1
  return bytes + (value.match(/^1*/)[0].length) === 32
}

function objectId(value) {
  requireValue(typeof value === 'string' && /^0x[0-9a-fA-F]{1,64}$/.test(value))
  const normalized = `0x${value.slice(2).toLowerCase().padStart(64, '0')}`
  requireValue(!/^0x0+$/.test(normalized))
  return normalized
}

function publicAddress(address) {
  if (isIP(address) === 4) {
    const [a, b, c] = address.split('.').map(Number)
    return !(a === 0 || a === 10 || a === 127 || a >= 224 ||
      (a === 100 && b >= 64 && b <= 127) || (a === 169 && b === 254) ||
      (a === 172 && b >= 16 && b <= 31) || (a === 192 && (b === 0 || b === 168)) ||
      (a === 192 && b === 88 && c === 99) || (a === 198 && (b === 18 || b === 19)) ||
      (a === 198 && b === 51 && c === 100) || (a === 203 && b === 0 && c === 113))
  }
  // Only native global unicast; exclude documentation, Teredo and 6to4 ranges.
  if (isIP(address) !== 6 || !/^[23][0-9a-f]{3}:/i.test(address)) return false
  const [first, second = '0'] = address.split(':').map(part => parseInt(part || '0', 16))
  return !(first === 0x2002 || (first === 0x2001 && (second < 0x200 || second === 0xdb8)) ||
    (first === 0x3fff && second < 0x1000))
}

function endpoint(value) {
  requireValue(typeof value === 'string' && value.length <= 300 &&
    !/[\s\\%]/.test(value), 'INVALID_ENDPOINT')
  let url
  try { url = new URL(value) } catch { throw new ObservationError('INVALID_ENDPOINT') }
  requireValue(url.protocol === 'https:' && !url.username && !url.password &&
    !url.search && !url.hash && !url.port &&
    /^[a-z0-9](?:[a-z0-9.-]*[a-z0-9])$/i.test(url.hostname) &&
    url.hostname.includes('.') && !isIP(url.hostname) &&
    !/\.(localhost|local|internal|test|invalid|example)$/i.test(url.hostname) &&
    ['/', '/graphql'].includes(url.pathname), 'INVALID_ENDPOINT')
  return url.href
}

function validateInput(input) {
  const fields = ['network', 'chain_identifier', 'package_id', 'publication_digest', 'endpoints']
  requireValue(input && typeof input === 'object' && !Array.isArray(input) &&
    Object.keys(input).length === fields.length && fields.every(key => Object.hasOwn(input, key)))
  requireValue(input.network === 'testnet' && digest32(input.chain_identifier) &&
    digest32(input.publication_digest))
  requireValue(Array.isArray(input.endpoints) && input.endpoints.length === 2)
  const endpoints = input.endpoints.map(endpoint)
  // The official primary anchors the testnet label. A caller cannot rename mainnet as testnet.
  requireValue(endpoints[0] === OFFICIAL_TESTNET &&
    new URL(endpoints[0]).hostname !== new URL(endpoints[1]).hostname, 'INVALID_ENDPOINT')
  return {
    network: 'testnet', chain_identifier: input.chain_identifier,
    package_id: objectId(input.package_id), publication_digest: input.publication_digest,
    endpoints,
  }
}

function safeCode(error) {
  return error instanceof ObservationError ? error.code : 'NETWORK_ERROR'
}

module.exports = {
  OFFICIAL_TESTNET, ObservationError, requireValue, digest32, objectId,
  publicAddress, endpoint, validateInput, safeCode,
}
