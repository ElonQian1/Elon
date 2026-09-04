const { validateInput, parseAmount, I64_MAX, InputError } = require('./input')
const { fingerprint, sourceFingerprint, paymentKey } = require('./identity')

function indexed(values, key) {
  const result = new Map()
  for (const value of values) {
    const id = key(value)
    result.set(id, [...(result.get(id) || []), value])
  }
  return result
}

function emptyReport() {
  return {
    schema: 'yilong.esk.paid_reconciliation_preview.v1', mode: 'dry_run', status: 'invalid_input',
    funds_moved: false, balances_written: false, commit_eligible: false,
    payment_authenticity_verified: false, identity_verified: false, approvals_verified: false,
    evidence_basis: 'operator_supplied_snapshot_consistency_only',
    as_of: null, input_digest: null, source_fingerprint: null, snapshot_digest: null,
    rows: [], proposed_totals: [], counts: null, error_code: null, report_digest: null,
  }
}

function failureReport(error) {
  const report = emptyReport()
  report.error_code = error instanceof InputError ? error.code : 'INVALID_INPUT'
  return report
}

function snapshotReasons(input, actualSourceFingerprint) {
  const reasons = []
  const snapshot = input.snapshot
  if (snapshot.source_fingerprint !== actualSourceFingerprint) reasons.push('SNAPSHOT_SOURCE_MISMATCH')
  if (!snapshot.history_complete) reasons.push('HISTORY_INCOMPLETE')
  const age = Date.parse(input.as_of) - Date.parse(snapshot.observed_at)
  if (age < 0) reasons.push('SNAPSHOT_FROM_FUTURE')
  else if (age > 24 * 60 * 60 * 1000) reasons.push('SNAPSHOT_STALE')
  if (new Set(snapshot.used_payment_keys).size !== snapshot.used_payment_keys.length) {
    reasons.push('HISTORY_DUPLICATE_KEYS')
  }
  return reasons
}

function checkTerms(row, amount, sales, reasons) {
  const matches = sales.get(row.sale_batch_id) || []
  if (matches.length !== 1) {
    reasons.push(matches.length ? 'SALE_BATCH_AMBIGUOUS' : 'SALE_BATCH_MISSING')
    return null
  }
  const sale = matches[0]
  if (sale.disclosure_revision !== row.disclosure_revision) reasons.push('DISCLOSURE_MISMATCH')
  const numerator = amount * BigInt(sale.esk_base_units_per_lot)
  const denominator = BigInt(sale.payment_base_units_per_lot)
  if (numerator % denominator !== 0n) reasons.push('NON_INTEGRAL_ESK_QUOTE')
  else {
    const expected = numerator / denominator
    if (expected > I64_MAX) reasons.push('QUOTE_OVERFLOW')
    if (expected !== BigInt(row.esk_base_units)) reasons.push('ESK_QUOTE_MISMATCH')
  }
  return sale.terms_digest
}

function previewValidated(input) {
  const report = emptyReport()
  report.as_of = input.as_of
  report.input_digest = fingerprint(input)
  report.source_fingerprint = sourceFingerprint(input.source)
  report.snapshot_digest = fingerprint(input.snapshot)
  const baseReasons = snapshotReasons(input, report.source_fingerprint)
  const subjects = indexed(input.users, user => user.opaque_subject)
  const targets = indexed(input.users, user => user.target_user_ref)
  const sales = indexed(input.sale_batches, sale => sale.sale_batch_id)
  const keys = input.rows.map(row => paymentKey(input.source, row))
  const duplicateKeys = indexed(keys, key => key)
  const rowIds = indexed(input.rows, row => row.row_id)
  const history = new Set(input.snapshot.used_payment_keys)
  report.rows = input.rows.map((row, index) => {
    const reasons = [...baseReasons]
    const matches = subjects.get(row.opaque_subject) || []
    let target = null
    if (matches.length !== 1) reasons.push(matches.length ? 'SUBJECT_MAPPING_AMBIGUOUS' : 'SUBJECT_MAPPING_MISSING')
    else {
      target = matches[0].target_user_ref
      if (targets.get(target).length !== 1) reasons.push('TARGET_MAPPING_AMBIGUOUS')
    }
    if (duplicateKeys.get(keys[index]).length !== 1) reasons.push('DUPLICATE_BATCH_PAYMENT')
    if (rowIds.get(row.row_id).length !== 1) reasons.push('DUPLICATE_ROW_ID')
    if (history.has(keys[index])) reasons.push('PAYMENT_ALREADY_USED')
    if (row.payment_status !== 'confirmed') reasons.push(`PAYMENT_${row.payment_status.toUpperCase()}`)
    if (!row.consent_digest) reasons.push('CONSENT_MISSING')
    if (!row.approval_digest) reasons.push('APPROVAL_MISSING')
    const amount = parseAmount(row.payment_amount, input.source.decimals)
    let route = null
    let termsDigest = null
    if (row.commercial_purpose === 'esk_purchase') {
      if (row.esk_base_units === '0') reasons.push('ESK_AMOUNT_REQUIRED')
      termsDigest = checkTerms(row, amount, sales, reasons)
    } else if (row.commercial_purpose === 'unconfirmed') reasons.push('PURPOSE_UNCONFIRMED')
    else {
      route = row.commercial_purpose === 'service_purchase' ? 'service_orders' : 'qshare_subscription'
      if (row.esk_base_units !== '0' || row.sale_batch_id !== null) reasons.push('NON_ESK_ALLOCATION_FORBIDDEN')
    }
    return {
      row_number: index + 1, row_ref_sha256: fingerprint(row.row_id), payment_key: keys[index],
      opaque_subject: row.opaque_subject, target_user_ref: target,
      status: reasons.length ? 'blocked' : route ? 'routed_elsewhere' : 'review_ready',
      reasons: reasons.sort(), route, payment_base_units: amount.toString(),
      proposed_esk_base_units: row.esk_base_units, sale_terms_digest: termsDigest,
    }
  })
  const readyByTarget = indexed(report.rows.filter(row => row.status === 'review_ready'), row => row.target_user_ref)
  for (const rows of readyByTarget.values()) {
    const total = rows.reduce((sum, row) => sum + BigInt(row.proposed_esk_base_units), 0n)
    if (total > I64_MAX) {
      for (const row of rows) { row.status = 'blocked'; row.reasons.push('USER_TOTAL_OVERFLOW') }
    } else {
      report.proposed_totals.push({ opaque_subject: rows[0].opaque_subject,
        target_user_ref: rows[0].target_user_ref, row_count: rows.length,
        payment_base_units: rows.reduce((sum, row) => sum + BigInt(row.payment_base_units), 0n).toString(),
        esk_base_units: total.toString(),
      })
    }
  }
  report.proposed_totals.sort((a, b) => a.target_user_ref < b.target_user_ref ? -1 : a.target_user_ref > b.target_user_ref ? 1 : 0)
  report.counts = { rows: report.rows.length, review_ready: 0, blocked: 0, routed_elsewhere: 0 }
  for (const row of report.rows) report.counts[row.status] += 1
  report.status = report.counts.blocked ? 'needs_review' : report.counts.review_ready ? 'review_ready' : 'routed_only'
  // Verification replaces this field with null before recomputing the canonical digest.
  report.report_digest = fingerprint(report)
  return report
}

function preview(input) {
  try { return previewValidated(validateInput(input)) } catch (error) { return failureReport(error) }
}

module.exports = { emptyReport, failureReport, preview }
