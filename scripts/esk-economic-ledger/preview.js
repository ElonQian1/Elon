'use strict';

const { fingerprint } = require('../esk-paid-reconciliation/identity');
const { MAX_INPUT_BYTES, LedgerInputError, readLedgerInput } = require('./input');
const { checkSources } = require('./sources');
const { projectJournal } = require('./journal');

function fixedBoundaries() {
  return {
    policy_status: 'PENDING', production_authorized: false, funding_verified: false,
    profit_realization_verified: false, coverage_verified: false,
    funds_moved: false, balances_written: false,
  };
}

function evaluateLedgerBuffer(buffer) {
  const { input, policyReview, paidReview } = readLedgerInput(buffer);
  const issues = new Set();
  if (policyReview.review_status === 'needs_correction') {
    issues.add('POLICY_NEEDS_CORRECTION');
    for (const code of policyReview.consistency_issues) issues.add(`POLICY_${code}`);
  }
  if (paidReview.status === 'needs_review') issues.add('PAID_RECONCILIATION_NEEDS_REVIEW');
  const { lotById, funding } = checkSources(input, paidReview, policyReview.input_digest, issues);
  const { counts, purposes } = projectJournal(input.journal, lotById, issues);
  const activeTotal = Object.values(purposes).reduce((sum, value) => sum + value, 0n);
  const totals = issues.size ? null : {
    funding_total_base_units: funding.total.toString(),
    esk_purchase_base_units: funding.esk_purchase.toString(),
    sponsor_capital_base_units: funding.sponsor_capital.toString(),
    realized_profit_base_units: funding.realized_profit.toString(),
    investment_base_units: purposes.investment.toString(),
    guarantee_reserve_base_units: purposes.guarantee_reserve.toString(),
    profit_distribution_base_units: purposes.profit_distribution.toString(),
    unallocated_base_units: (funding.total - activeTotal).toString(),
  };
  return {
    schema: 'elon.esk.economic_ledger_preview_report.v1',
    review_status: issues.size ? 'needs_review' : 'consistent',
    ...fixedBoundaries(),
    input_digest: fingerprint(input), policy_digest: policyReview.input_digest,
    policy_review_status: policyReview.review_status,
    policy_missing_decisions: [...policyReview.missing_decisions],
    issues: [...issues].sort(), asset_symbol: 'USDT', decimals: input.paid_reconciliation.source.decimals,
    source_fingerprint: paidReview.source_fingerprint,
    counts: { funding_lots: input.funding_lots.length, obligation_links: input.obligation_links.length,
      journal_entries: input.journal.length, ...counts },
    totals,
    evidence_basis: 'operator_declared_consistency_only',
  };
}

function failureReport(error) {
  return {
    schema: 'elon.esk.economic_ledger_preview_error.v1', ...fixedBoundaries(),
    error: { code: new LedgerInputError(error && error.code).code },
  };
}

module.exports = { MAX_INPUT_BYTES, LedgerInputError, evaluateLedgerBuffer, failureReport };
