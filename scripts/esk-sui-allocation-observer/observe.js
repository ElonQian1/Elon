const { createHash } = require('node:crypto')
const { validateInput, safeCode } = require('./contract')
const { readObservation } = require('./graphql')
const { validateObservation } = require('./validation')

function emptyReport() {
  return {
    schema: 'yilong.esk.sui.allocation_observation.v1',
    status: 'unverified', observed_at: new Date().toISOString(),
    allocation_observed: false, team_vesting_observed: false,
    publication_certified: false, source_verified: false,
    allocation_certified: false, address_control_verified: false,
    finality_certified: false, asset_identity_verified: false,
    balance_eligible: false, manifest_transition_allowed: false,
    trust_basis: 'two_rpc_reports_not_committee_signature_verification',
    expected: null, sources: [], evidence: null, error_code: null,
  }
}

async function observeAllocation(input, { read = readObservation } = {}) {
  const report = emptyReport()
  let expected
  try { expected = validateInput(input) } catch (error) {
    report.error_code = safeCode(error)
    return report
  }
  const { endpoints, ...identity } = expected
  report.expected = identity
  report.sources = await Promise.all(endpoints.map(async (url, index) => {
    const source = {
      source: index === 0 ? 'official_testnet' : 'secondary',
      endpoint_sha256: createHash('sha256').update(url).digest('hex'),
    }
    try {
      const evidence = validateObservation(await read(url, expected), expected)
      return { ...source, status: 'observed', evidence, error_code: null }
    } catch (error) {
      return { ...source, status: 'unverified', evidence: null, error_code: safeCode(error) }
    }
  }))
  const failure = report.sources.find(source => source.status !== 'observed')
  if (failure) report.error_code = failure.error_code
  else if (JSON.stringify(report.sources[0].evidence) !==
    JSON.stringify(report.sources[1].evidence)) {
    report.error_code = 'SOURCE_DISAGREEMENT'
  } else {
    report.status = 'observed'
    report.allocation_observed = true
    report.team_vesting_observed = true
    report.evidence = report.sources[0].evidence
  }
  return report
}

module.exports = { emptyReport, observeAllocation }
