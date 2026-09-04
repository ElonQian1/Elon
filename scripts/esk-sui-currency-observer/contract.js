const publication = require('../esk-sui-publication-observer/contract')

const MAX_U64 = 18446744073709551615n
const MAX_UINT53 = 9007199254740991n
const CODES = new Set([
  'INVALID_INPUT', 'INVALID_RESPONSE', 'REGISTRATION_MISMATCH', 'CURRENCY_MISMATCH',
  'SUPPLY_MISMATCH', 'VERSION_MISMATCH', 'CHECKPOINT_MISSING',
  'TRANSACTION_NOT_SUCCESSFUL', 'SOURCE_DISAGREEMENT', 'SDK_UNAVAILABLE',
])

class CurrencyObservationError extends Error {
  constructor(code) {
    const safe = CODES.has(code) ? code : 'INVALID_RESPONSE'
    super(safe)
    this.code = safe
  }
}

function requireValue(condition, code = 'INVALID_INPUT') {
  if (!condition) throw new CurrencyObservationError(code)
}

function positiveDecimal(value, max, code = 'INVALID_INPUT') {
  requireValue(typeof value === 'string' && value.length <= 20 &&
    /^[1-9][0-9]*$/.test(value), code)
  requireValue(BigInt(value) <= max, code)
  return value
}

function validateInput(input) {
  const fields = ['network', 'chain_identifier', 'package_id', 'publication_digest',
    'registration_digest', 'registration_version', 'expected_supply_base_units', 'endpoints']
  requireValue(input && typeof input === 'object' && !Array.isArray(input) &&
    Object.keys(input).length === fields.length && fields.every(key => Object.hasOwn(input, key)))
  requireValue(publication.digest32(input.registration_digest))
  const registrationVersion = positiveDecimal(input.registration_version, MAX_UINT53)
  const supply = positiveDecimal(input.expected_supply_base_units, MAX_U64)
  const base = publication.validateInput({
    network: input.network, chain_identifier: input.chain_identifier,
    package_id: input.package_id, publication_digest: input.publication_digest,
    endpoints: input.endpoints,
  })
  return {
    ...base, registration_digest: input.registration_digest,
    registration_version: registrationVersion, expected_supply_base_units: supply,
    coin_type: `${base.package_id}::esk::ESK`,
  }
}

function safeCode(error) {
  if (error instanceof CurrencyObservationError) return error.code
  return publication.safeCode(error)
}

module.exports = {
  CurrencyObservationError, requireValue, positiveDecimal, validateInput, safeCode,
  MAX_U64, MAX_UINT53, digest32: publication.digest32, objectId: publication.objectId,
  OFFICIAL_TESTNET: publication.OFFICIAL_TESTNET,
}
