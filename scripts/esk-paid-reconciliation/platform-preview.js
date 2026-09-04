'use strict'

const { InputError, validateInput } = require('./input')
const { fingerprint } = require('./identity')
const { preview } = require('./preview')
const { SnapshotError, exactObject, validatePlatformSnapshot, joinHistory } = require('./platform-snapshot')

function failure(error) {
  return {
    schema: 'yilong.esk.platform_reconciliation_preview.v1', mode: 'dry_run', status: 'invalid_input',
    funds_moved: false, balances_written: false, commit_eligible: false,
    platform_snapshot_authenticity_verified: false,
    history_coverage_basis: 'operator_declared_external_history_plus_supplied_platform_snapshot',
    input_digest: null, platform_snapshot_digest: null, preview: null,
    error_code: error instanceof InputError || error instanceof SnapshotError ? error.code : 'INVALID_INPUT',
    report_digest: null,
  }
}

function previewWithPlatformSnapshot(envelope) {
  try {
    exactObject(envelope, ['schema', 'reconciliation', 'platform_snapshot'])
    if (envelope.schema !== 'yilong.esk.platform_reconciliation_input.v1') throw new InputError()
    const input = validateInput(envelope.reconciliation)
    const snapshot = validatePlatformSnapshot(envelope.platform_snapshot, input)
    const result = preview(joinHistory(input, snapshot))
    const report = failure(null)
    report.status = result.status
    report.input_digest = fingerprint(envelope)
    report.platform_snapshot_digest = snapshot.snapshot_digest
    report.preview = result
    report.error_code = result.error_code
    report.report_digest = fingerprint(report)
    return report
  } catch (error) { return failure(error) }
}

module.exports = { failure, previewWithPlatformSnapshot }
