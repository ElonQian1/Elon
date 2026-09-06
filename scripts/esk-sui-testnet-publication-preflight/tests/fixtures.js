'use strict'

const { createTemplate } = require('../template')
const { APPROVED_BASELINE_COMMIT } = require('../repository')

const REVIEWED_AT = '2026-09-06T12:00:00.000Z'
const APPROVED_AT = '2026-09-05T12:00:00.000Z'
const EXPIRES_AT = '2026-12-31T23:59:59.000Z'
const ROLE_NAMES = [
  'distribution', 'treasury', 'liquidity', 'team_beneficiary', 'metadata',
  'currency_upgrade', 'participation_upgrade', 'pause', 'gas_sponsor', 'deployer',
]
const APPROVAL_NAMES = [
  'economic_parameters', 'role_controls', 'multisig_recovery',
  'release_candidate_review',
]

function clone(value) { return JSON.parse(JSON.stringify(value)) }

function address(index, synthetic) {
  const value = `0x${index.toString(16).padStart(64, '0')}`
  return synthetic ? `synthetic:sui:${value}` : value
}

function complete(mode) {
  const synthetic = mode === 'synthetic_test'
  const candidate = clone(createTemplate())
  candidate.candidate_id = synthetic
    ? 'esk-sui-testnet-synthetic-v1'
    : 'esk-sui-testnet-release-v1'
  candidate.scope = { network: 'testnet', mode, synthetic, reviewed_at: REVIEWED_AT }
  candidate.repository.baseline_commit = APPROVED_BASELINE_COMMIT
  ROLE_NAMES.forEach((name, index) => { candidate.roles[name] = address(index + 1, synthetic) })
  candidate.team_vesting.start_ms = '1893456000000'
  candidate.team_vesting.cliff_ms = '1924992000000'
  candidate.team_vesting.end_ms = '2019686400000'
  candidate.upgrade_policies = { currency: 'compatible', participation: 'additive' }
  candidate.gas_budgets = {
    currency_publish: 100000000,
    currency_registry_finalize: 50000000,
    participation_publish: 100000000,
    genesis_allocation: 100000000,
    capability_handoff: 50000000,
  }
  APPROVAL_NAMES.forEach((name, index) => {
    candidate.approvals[name] = {
      digest: `sha256:${(index + 10).toString(16).repeat(64).slice(0, 64)}`,
      approved_at: APPROVED_AT,
      expires_at: EXPIRES_AT,
    }
  })
  return candidate
}

function templateCandidate() { return clone(createTemplate()) }
function syntheticCandidate() { return complete('synthetic_test') }
function releaseCandidate() { return complete('release_candidate') }

function reverseObjectKeys(value) {
  if (Array.isArray(value)) return value.map(reverseObjectKeys)
  if (!value || typeof value !== 'object') return value
  return Object.fromEntries(Object.keys(value).reverse()
    .map(key => [key, reverseObjectKeys(value[key])]))
}

module.exports = {
  REVIEWED_AT, APPROVED_AT, EXPIRES_AT, ROLE_NAMES, APPROVAL_NAMES,
  clone, address, templateCandidate, syntheticCandidate, releaseCandidate,
  reverseObjectKeys,
}
