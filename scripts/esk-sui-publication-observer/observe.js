const { readObservation } = require('./graphql')
const { createHash } = require('node:crypto')
const {
  ObservationError, requireValue, validateInput, objectId, digest32, safeCode,
} = require('./contract')

function uint53(value, code) {
  requireValue(typeof value === 'number' && Number.isSafeInteger(value) && value >= 0, code)
  return String(value)
}

function validateObservation(data, expected) {
  requireValue(data && typeof data === 'object', 'INVALID_RESPONSE')
  requireValue(data.chainIdentifier === expected.chain_identifier, 'CHAIN_MISMATCH')
  const object = data.object
  requireValue(object && object.asMovePackage, 'PACKAGE_MISMATCH')
  try {
    requireValue(objectId(object.address) === expected.package_id &&
      objectId(object.asMovePackage.address) === expected.package_id, 'PACKAGE_MISMATCH')
  } catch { throw new ObservationError('PACKAGE_MISMATCH') }
  requireValue(object.version === object.asMovePackage.version && object.version > 0 &&
    digest32(object.digest), 'PACKAGE_MISMATCH')
  const version = uint53(object.version, 'PACKAGE_MISMATCH')
  requireValue(object.previousTransaction?.digest === expected.publication_digest &&
    data.transaction?.digest === expected.publication_digest, 'TRANSACTION_MISMATCH')
  const effects = data.transaction.effects
  requireValue(effects?.status === 'SUCCESS', 'TRANSACTION_NOT_SUCCESSFUL')
  const checkpoint = effects.checkpoint
  requireValue(checkpoint && digest32(checkpoint.digest), 'CHECKPOINT_MISSING')
  return {
    chain_identifier: data.chainIdentifier, package_id: expected.package_id,
    package_version: version, package_digest: object.digest,
    publication_digest: expected.publication_digest,
    checkpoint_sequence: uint53(checkpoint.sequenceNumber, 'CHECKPOINT_MISSING'),
    checkpoint_digest: checkpoint.digest,
  }
}

async function observePublication(input, { read = readObservation } = {}) {
  const report = {
    schema: 'yilong.esk.sui.publication_observation.v1',
    status: 'unverified', observed_at: new Date().toISOString(),
    publication_certified: false, asset_identity_verified: false,
    balance_eligible: false, manifest_transition_allowed: false,
    trust_basis: 'rpc_reports_not_committee_signature_verification',
    expected: null, sources: [], evidence: null, error_code: null,
  }
  let expected
  try { expected = validateInput(input) } catch (error) {
    report.error_code = safeCode(error)
    return report
  }
  report.expected = {
    network: expected.network, chain_identifier: expected.chain_identifier,
    package_id: expected.package_id, publication_digest: expected.publication_digest,
  }
  // A failed or missing source cannot fall back to the successful one.
  report.sources = await Promise.all(expected.endpoints.map(async (endpoint, index) => {
    const sourceRef = {
      source: index === 0 ? 'official_testnet' : 'secondary',
      endpoint_sha256: createHash('sha256').update(endpoint).digest('hex'),
    }
    try {
      const evidence = validateObservation(await read(endpoint, expected), expected)
      return { ...sourceRef,
        status: 'observed', evidence, error_code: null }
    } catch (error) {
      return { ...sourceRef,
        status: 'unverified', evidence: null, error_code: safeCode(error) }
    }
  }))
  const failed = report.sources.find(source => source.status !== 'observed')
  if (failed) report.error_code = failed.error_code
  else if (JSON.stringify(report.sources[0].evidence) !== JSON.stringify(report.sources[1].evidence)) {
    report.error_code = 'SOURCE_DISAGREEMENT'
  } else {
    report.status = 'observed'
    report.evidence = report.sources[0].evidence
  }
  return report
}

module.exports = { uint53, validateObservation, observePublication }
