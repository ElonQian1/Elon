'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { evaluatePolicyBuffer } = require('../../esk-early-support-policy/contract');
const { evaluateLedgerBuffer } = require('../preview');

const ROOT = path.resolve(__dirname, '../../..');
const CLI = path.join(ROOT, 'scripts/esk-economic-ledger/cli.js');
const U128_MAX = (1n << 128n) - 1n;
const FALSE_FIELDS = ['production_authorized', 'funding_verified', 'profit_realization_verified',
  'coverage_verified', 'funds_moved', 'balances_written'];
const encode = value => Buffer.from(JSON.stringify(value));
const evaluate = value => evaluateLedgerBuffer(encode(value));
const clone = value => JSON.parse(JSON.stringify(value));

function readFixture(relative) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, relative), 'utf8'));
}

function policyDigest(input) {
  return evaluatePolicyBuffer(encode(input.policy_draft)).input_digest;
}

function proposal(sequence, lot_id, purpose, amount_base_units, request_id = `request-${sequence}`) {
  return { sequence, event_id: `event-${sequence}`, idempotency_key: `idempotency-${sequence}`,
    operation: 'propose', request_id, lot_id, purpose, amount_base_units };
}

function cancellation(sequence, request_id) {
  return { sequence, event_id: `event-${sequence}`, idempotency_key: `idempotency-${sequence}`,
    operation: 'cancel', request_id };
}

function batch() {
  const policy_draft = readFixture('contracts/esk/early-support-policy-draft-v1.fixture.json');
  const paid_reconciliation = readFixture('contracts/assets/esk-paid-reconciliation-v1.fixture.json');
  paid_reconciliation.rows[0].payment_amount = '0.000100';
  paid_reconciliation.rows[0].esk_base_units = '50';
  const input = {
    schema: 'elon.esk.economic_ledger_preview_input.v1', mode: 'offline_draft',
    policy_draft, paid_reconciliation,
    funding_lots: [
      { lot_id: 'purchase', origin: 'esk_purchase', external_payment_reference: 'a'.repeat(64),
        transfer_index: 0, amount_base_units: '100' },
      { lot_id: 'sponsor', origin: 'sponsor_capital', external_payment_reference: 'b'.repeat(64),
        transfer_index: 0, amount_base_units: '50' },
      { lot_id: 'profit', origin: 'realized_profit', external_payment_reference: 'c'.repeat(64),
        transfer_index: 0, amount_base_units: '30' },
    ],
    obligation_links: [],
    journal: [proposal(1, 'purchase', 'investment', '80'),
      proposal(2, 'purchase', 'guarantee_reserve', '20'),
      proposal(3, 'sponsor', 'guarantee_reserve', '50'),
      proposal(4, 'profit', 'profit_distribution', '10')],
  };
  input.obligation_links.push({ obligation_id: 'obligation-1', purchase_lot_id: 'purchase',
    policy_digest: policyDigest(input), status: 'PENDING',
    protected_principal_base_units: null, minimum_return_base_units: null });
  return input;
}

function sponsorOnly(amount = '50') {
  const input = batch();
  input.funding_lots = [input.funding_lots[1]];
  input.funding_lots[0].amount_base_units = amount;
  input.obligation_links = [];
  input.journal = [proposal(1, 'sponsor', 'investment', amount)];
  return input;
}

function completePolicy(input) {
  Object.assign(input.policy_draft.decisions, {
    protection_scope: 'principal_only', principal_denomination: 'USDT', term_basis: 'program_window',
    program_start_at: '2028-02-29T00:00:00Z', program_end_at: '2030-03-01T00:00:00Z',
    guarantor_id: 'synthetic-not-real', funding_source: 'Synthetic source declaration',
    funding_description: 'PRIVATE_SYNTHETIC_POLICY_281746', redemption_rule: 'Proposed redemption rule',
    transfer_rule: 'Proposed transfer rule', consumption_rule: 'Proposed consumption rule',
    redemption_token_rule: 'Proposed token return rule', prior_returns_rule: 'Proposed returns rule',
    anniversary_rule: 'Proposed anniversary rule', minimum_return_terms: null,
  });
  for (const obligation of input.obligation_links) obligation.policy_digest = policyDigest(input);
}

function assertFalseFlags(report) {
  assert.equal(report.policy_status, 'PENDING');
  for (const field of FALSE_FIELDS) assert.equal(report[field], false, field);
}

function assertReport(report) {
  assert.equal(report.schema, 'elon.esk.economic_ledger_preview_report.v1');
  assertFalseFlags(report);
  assert.equal(report.evidence_basis, 'operator_declared_consistency_only');
  assert.match(report.input_digest, /^[0-9a-f]{64}$/);
  assert.match(report.policy_digest, /^[0-9a-f]{64}$/);
  assert.match(report.source_fingerprint, /^[0-9a-f]{64}$/);
  assert.deepEqual(report.issues, [...new Set(report.issues)].sort());
}

function assertIssues(input, ...codes) {
  const report = evaluate(input);
  assertReport(report);
  assert.equal(report.review_status, 'needs_review');
  assert.equal(report.totals, null);
  for (const code of codes) assert.ok(report.issues.includes(code), `missing issue ${code}`);
  return report;
}

function assertInvalid(buffer, code) {
  assert.throws(() => evaluateLedgerBuffer(buffer), error => {
    assert.equal(error.code, code);
    assert.equal(error.message, code);
    return true;
  });
}

module.exports = { ROOT, CLI, U128_MAX, FALSE_FIELDS, encode, evaluate, clone, policyDigest,
  proposal, cancellation, batch, sponsorOnly, completePolicy, assertFalseFlags, assertReport,
  assertIssues, assertInvalid };
