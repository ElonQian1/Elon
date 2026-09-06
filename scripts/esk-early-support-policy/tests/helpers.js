'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { evaluatePolicyBuffer } = require('../contract');

const ROOT = path.resolve(__dirname, '../../..');
const FIXTURE_PATH = path.join(ROOT, 'contracts/esk/early-support-policy-draft-v1.fixture.json');
const CLI_PATH = path.join(ROOT, 'scripts/esk-early-support-policy/cli.js');
const DECISIONS = [
  'protection_scope', 'principal_denomination', 'term_basis', 'program_start_at',
  'program_end_at', 'guarantor_id', 'funding_source', 'funding_description',
  'redemption_rule', 'transfer_rule', 'consumption_rule', 'redemption_token_rule',
  'prior_returns_rule', 'anniversary_rule', 'minimum_return_terms',
];

function fixture() {
  return JSON.parse(fs.readFileSync(FIXTURE_PATH, 'utf8'));
}

function completeDraft() {
  const draft = fixture();
  Object.assign(draft.decisions, {
    protection_scope: 'principal_only',
    principal_denomination: 'USDT',
    term_basis: 'per_purchase_anniversary',
    program_start_at: '2028-02-29T00:00:00Z',
    program_end_at: '2030-03-01T00:00:00Z',
    guarantor_id: 'synthetic-only:no-real-guarantor',
    funding_source: 'Synthetic source for an offline test only',
    funding_description: 'PRIVATE_SYNTHETIC_FUNDING_TEXT_3291',
    redemption_rule: 'Synthetic review text for redemption timing',
    transfer_rule: 'Synthetic review text for transfer eligibility',
    consumption_rule: 'Synthetic review text for spent tokens',
    redemption_token_rule: 'Synthetic review text for token return',
    prior_returns_rule: 'Synthetic review text for prior returns',
    anniversary_rule: 'Synthetic review text for leap-day anniversaries',
    minimum_return_terms: null,
  });
  return draft;
}

function encode(value) {
  return Buffer.from(JSON.stringify(value));
}

function evaluate(value) {
  return evaluatePolicyBuffer(encode(value));
}

function assertDraftOnly(report) {
  assert.equal(report.schema, 'elon.esk.early_support_policy_review.v1');
  assert.equal(report.policy_status, 'draft');
  assert.equal(report.production_authorized, false);
  assert.equal(report.funding_verified, false);
  assert.equal(report.funds_moved, false);
  assert.match(report.input_digest, /^[a-f0-9]{64}$/);
  assert.ok(Array.isArray(report.missing_decisions));
  assert.ok(Array.isArray(report.consistency_issues));
  assert.deepEqual(Object.keys(report).sort(), [
    'schema', 'policy_status', 'review_status', 'input_digest', 'missing_decisions',
    'consistency_issues', 'production_authorized', 'funding_verified', 'funds_moved',
  ].sort());
}

function assertInvalid(buffer, code) {
  assert.throws(() => evaluatePolicyBuffer(buffer), (error) => {
    assert.equal(error.code, code);
    assert.equal(error.message, code);
    return true;
  });
}

module.exports = {
  ROOT, FIXTURE_PATH, CLI_PATH, DECISIONS, fixture, completeDraft, encode,
  evaluate, assertDraftOnly, assertInvalid,
};
