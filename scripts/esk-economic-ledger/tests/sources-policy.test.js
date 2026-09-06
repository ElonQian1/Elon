'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { paymentKey, sourceFingerprint } = require('../../esk-paid-reconciliation/identity');
const { preview: previewPaid } = require('../../esk-paid-reconciliation/preview');
const { batch, sponsorOnly, clone, proposal, evaluate, encode, completePolicy,
  policyDigest, assertReport, assertIssues, assertInvalid } = require('./helpers');

test('independent small-unit oracle preserves pending policy and exact totals', () => {
  const input = batch();
  assert.equal(previewPaid(input.paid_reconciliation).status, 'review_ready');
  const report = evaluate(input);
  assertReport(report);
  assert.equal(report.review_status, 'consistent');
  assert.equal(report.policy_review_status, 'needs_decisions');
  assert.equal(report.policy_missing_decisions.length, 14);
  assert.equal(report.asset_symbol, 'USDT');
  assert.equal(report.decimals, 6);
  assert.deepEqual(report.issues, []);
  assert.deepEqual(report.totals, { funding_total_base_units: '180', esk_purchase_base_units: '100',
    sponsor_capital_base_units: '50', realized_profit_base_units: '30', investment_base_units: '80',
    guarantee_reserve_base_units: '70', profit_distribution_base_units: '10', unallocated_base_units: '20' });
  assert.deepEqual(report.counts, { funding_lots: 3, obligation_links: 1, journal_entries: 4,
    unique_events: 4, replayed_events: 0, proposals: 4, pending_proposals: 4, canceled_proposals: 0 });
});

test('fully populated policy still establishes neither funding nor an executable guarantee', () => {
  const input = batch();
  completePolicy(input);
  const report = evaluate(input);
  assertReport(report);
  assert.equal(report.review_status, 'consistent');
  assert.equal(report.policy_review_status, 'ready_for_policy_review');
  assert.deepEqual(report.policy_missing_decisions, []);
  assert.equal(JSON.stringify(report).includes('PRIVATE_SYNTHETIC_POLICY_281746'), false);
  assert.equal(report.policy_digest, policyDigest(input));
  assert.equal(input.obligation_links[0].protected_principal_base_units, null);
  assert.equal(input.obligation_links[0].minimum_return_base_units, null);
});

test('policy contradictions are visible even with matching version references', () => {
  const input = batch();
  input.policy_draft.decisions.minimum_return_terms = 'Unapproved synthetic minimum terms';
  input.obligation_links[0].policy_digest = policyDigest(input);
  assertIssues(input, 'POLICY_NEEDS_CORRECTION', 'POLICY_MINIMUM_RETURN_SCOPE_UNDECIDED');
  completePolicy(input);
  input.policy_draft.decisions.minimum_return_terms = 'Terms attached to the wrong protection scope';
  input.obligation_links[0].policy_digest = policyDigest(input);
  assertIssues(input, 'POLICY_NEEDS_CORRECTION', 'POLICY_MINIMUM_RETURN_TERMS_NOT_APPLICABLE');
});

test('same payment identity cannot be counted again under another classification or hex spelling', () => {
  for (const origin of ['esk_purchase', 'sponsor_capital', 'realized_profit']) {
    const input = batch();
    input.funding_lots.push({ ...input.funding_lots[0], lot_id: 'renamed-payment', origin,
      external_payment_reference: '0X' + 'A'.repeat(64) });
    assertIssues(input, 'DUPLICATE_PAYMENT_KEY');
  }
});

test('a non-purchase lot cannot reuse any declared paid row, including alternate routes and blocked rows', () => {
  for (const origin of ['sponsor_capital', 'realized_profit']) {
    for (const purpose of ['esk_purchase', 'service_purchase', 'quant_subscription', 'unconfirmed']) {
      const input = sponsorOnly();
      input.funding_lots[0].origin = origin;
      input.funding_lots[0].external_payment_reference = 'a'.repeat(64);
      const row = input.paid_reconciliation.rows[0];
      row.commercial_purpose = purpose;
      if (purpose !== 'esk_purchase') { row.esk_base_units = '0'; row.sale_batch_id = null; }
      assertIssues(input, 'NON_PURCHASE_REUSES_PAID_PAYMENT');
    }
  }
  for (const status of ['pending', 'reversed']) {
    const input = sponsorOnly();
    input.funding_lots[0].origin = 'realized_profit';
    input.funding_lots[0].external_payment_reference = 'a'.repeat(64);
    input.paid_reconciliation.rows[0].payment_status = status;
    assertIssues(input, 'NON_PURCHASE_REUSES_PAID_PAYMENT', 'PAID_RECONCILIATION_NEEDS_REVIEW');
  }
});

test('historical used-payment keys block every funding classification', () => {
  for (const origin of ['esk_purchase', 'sponsor_capital', 'realized_profit']) {
    const input = origin === 'esk_purchase' ? batch() : sponsorOnly();
    const lot = input.funding_lots[0];
    lot.origin = origin;
    input.paid_reconciliation.snapshot.used_payment_keys.push(paymentKey(input.paid_reconciliation.source, lot));
    lot.external_payment_reference = '0x' + lot.external_payment_reference.toUpperCase();
    assertIssues(input, 'PAYMENT_ALREADY_USED');
  }
});

test('an unselected bad paid row or unreliable snapshot invalidates the complete batch', () => {
  const mutations = [
    paid => { paid.snapshot.history_complete = false; },
    paid => { paid.snapshot.observed_at = '2026-09-01T05:00:00.000Z'; },
    paid => { paid.snapshot.source_fingerprint = '0'.repeat(64); },
    paid => { paid.rows[0].approval_digest = null; },
    paid => { paid.rows[0].payment_status = 'reversed'; },
  ];
  for (const mutate of mutations) {
    const input = sponsorOnly();
    mutate(input.paid_reconciliation);
    assertIssues(input, 'PAID_RECONCILIATION_NEEDS_REVIEW');
  }
});

test('purchase references must resolve to one review-ready payment with exactly equal units', () => {
  const missing = batch();
  missing.funding_lots[0].external_payment_reference = 'f'.repeat(64);
  assertIssues(missing, 'PURCHASE_PAYMENT_NOT_FOUND');
  const wrongAmount = batch();
  wrongAmount.funding_lots[0].amount_base_units = '101';
  assertIssues(wrongAmount, 'PURCHASE_AMOUNT_MISMATCH');
  const blocked = batch();
  blocked.paid_reconciliation.rows[0].consent_digest = null;
  assertIssues(blocked, 'PURCHASE_PAYMENT_NOT_REVIEW_READY', 'PAID_RECONCILIATION_NEEDS_REVIEW');
  const ambiguous = batch();
  ambiguous.paid_reconciliation.rows.push({ ...ambiguous.paid_reconciliation.rows[0], row_id: 'second-row' });
  assertIssues(ambiguous, 'PURCHASE_PAYMENT_NOT_REVIEW_READY');
});

test('transfer indexes distinguish payments and opaque references remain case-sensitive', () => {
  const input = sponsorOnly();
  input.funding_lots.push({ ...input.funding_lots[0], lot_id: 'sponsor-second', transfer_index: 1,
    amount_base_units: '7' });
  input.journal.push(proposal(2, 'sponsor-second', 'investment', '7'));
  assert.equal(evaluate(input).totals.funding_total_base_units, '57');
  input.paid_reconciliation.source.reference_format = 'opaque';
  input.paid_reconciliation.snapshot.source_fingerprint = sourceFingerprint(input.paid_reconciliation.source);
  input.funding_lots[0].external_payment_reference = 'provider-Ref';
  input.funding_lots[1].external_payment_reference = 'provider-ref';
  input.funding_lots[1].transfer_index = 0;
  assert.equal(evaluate(input).review_status, 'consistent');
});

test('lot and obligation identities cannot be duplicated, omitted or redirected', () => {
  const duplicateLot = batch();
  duplicateLot.funding_lots[1].lot_id = 'purchase';
  assertIssues(duplicateLot, 'DUPLICATE_LOT_ID');
  const missing = batch();
  missing.obligation_links = [];
  assertIssues(missing, 'OBLIGATION_MISSING');
  const duplicate = batch();
  duplicate.obligation_links.push(clone(duplicate.obligation_links[0]));
  assertIssues(duplicate, 'OBLIGATION_DUPLICATE_ID', 'OBLIGATION_DUPLICATE_PURCHASE');
  for (const target of ['sponsor', 'profit', 'unknown']) {
    const invalid = batch();
    invalid.obligation_links[0].purchase_lot_id = target;
    assertIssues(invalid, 'OBLIGATION_PURCHASE_INVALID', 'OBLIGATION_MISSING');
  }
  const wrongPolicy = batch();
  wrongPolicy.obligation_links[0].policy_digest = 'f'.repeat(64);
  assertIssues(wrongPolicy, 'OBLIGATION_POLICY_MISMATCH');
});

test('pending unknown obligations cannot contain approved states or numerical promises', () => {
  for (const [field, values] of [['status', ['APPROVED', null]],
    ['protected_principal_base_units', ['0', '100', 0]], ['minimum_return_base_units', ['0', '1', 0]]]) {
    for (const value of values) {
      const input = batch();
      input.obligation_links[0][field] = value;
      assertInvalid(encode(input), 'INVALID_VALUE');
    }
  }
});
