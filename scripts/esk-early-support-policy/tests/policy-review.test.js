'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const test = require('node:test');
const { evaluatePolicyBuffer } = require('../contract');
const {
  FIXTURE_PATH, DECISIONS, fixture, completeDraft, encode, evaluate,
  assertDraftOnly, assertInvalid,
} = require('./helpers');

test('the shipped policy preserves every unresolved choice as null', () => {
  const before = fs.readFileSync(FIXTURE_PATH);
  const input = fixture();
  assert.deepEqual(Object.keys(input.decisions).sort(), [...DECISIONS].sort());
  assert.ok(Object.values(input.decisions).every((value) => value === null));
  const report = evaluatePolicyBuffer(before);
  assertDraftOnly(report);
  assert.equal(report.review_status, 'needs_decisions');
  assert.deepEqual(report.missing_decisions, DECISIONS.filter((field) => field !== 'minimum_return_terms'));
  assert.deepEqual(report.consistency_issues, []);
  assert.deepEqual(fs.readFileSync(FIXTURE_PATH), before);
  assert.deepEqual(input, fixture());
});

test('a completely populated synthetic policy can enter review but cannot authorize anything', () => {
  for (const protection_scope of ['principal_only', 'principal_and_minimum_return', 'limited_funded_loss_support']) {
    for (const term_basis of ['program_window', 'per_purchase_anniversary']) {
      const input = completeDraft();
      Object.assign(input.decisions, { protection_scope, term_basis });
      if (protection_scope === 'principal_and_minimum_return') {
        input.decisions.minimum_return_terms = 'Synthetic minimum-return terms, not an approved promise';
      }
      const buffer = encode(input);
      const before = Buffer.from(buffer);
      const report = evaluatePolicyBuffer(buffer);
      assertDraftOnly(report);
      assert.equal(report.review_status, 'ready_for_policy_review');
      assert.deepEqual(report.missing_decisions, []);
      assert.deepEqual(report.consistency_issues, []);
      assert.deepEqual(buffer, before);
    }
  }
});

test('each unresolved required decision independently blocks review readiness', () => {
  for (const field of DECISIONS.filter((value) => value !== 'minimum_return_terms')) {
    const input = completeDraft();
    input.decisions[field] = null;
    const report = evaluate(input);
    assertDraftOnly(report);
    assert.equal(report.review_status, 'needs_decisions', field);
    assert.deepEqual(report.missing_decisions, [field]);
  }
});

test('minimum-return terms are required only for the matching scope', () => {
  const input = completeDraft();
  input.decisions.protection_scope = 'principal_and_minimum_return';
  let report = evaluate(input);
  assert.equal(report.review_status, 'needs_decisions');
  assert.deepEqual(report.missing_decisions, ['minimum_return_terms']);
  input.decisions.minimum_return_terms = 'Synthetic test minimum-return terms';
  assert.equal(evaluate(input).review_status, 'ready_for_policy_review');
  for (const scope of ['principal_only', 'limited_funded_loss_support', null]) {
    input.decisions.protection_scope = scope;
    report = evaluate(input);
    assertDraftOnly(report);
    assert.equal(report.review_status, 'needs_correction');
    assert.deepEqual(report.consistency_issues, [scope === null
      ? 'MINIMUM_RETURN_SCOPE_UNDECIDED' : 'MINIMUM_RETURN_TERMS_NOT_APPLICABLE']);
    assert.deepEqual(report.missing_decisions, scope === null ? ['protection_scope'] : []);
  }
});

test('reports never echo user-supplied free text or invent a maturity for purchases', () => {
  const input = completeDraft();
  const output = JSON.stringify(evaluate(input));
  for (const field of ['guarantor_id', 'funding_description', 'redemption_rule', 'anniversary_rule']) {
    assert.equal(output.includes(input.decisions[field]), false, field);
  }
  assert.equal(output.includes('2030-03-01'), false);
  assert.equal(output.includes('maturity'), false);
});

test('canonical digests survive JSON order/format changes and bind changed policy text', () => {
  const input = completeDraft();
  const reordered = {};
  for (const key of Object.keys(input).reverse()) {
    const value = input[key];
    reordered[key] = value && typeof value === 'object'
      ? Object.fromEntries(Object.entries(value).reverse()) : value;
  }
  const report = evaluate(input);
  const formatted = evaluatePolicyBuffer(Buffer.from(JSON.stringify(reordered, null, 4)));
  assert.deepEqual(formatted, report);
  input.decisions.funding_description += ' Changed.';
  const changed = evaluate(input);
  assert.notEqual(changed.input_digest, report.input_digest);
  assert.equal(changed.review_status, report.review_status);
});

test('mutating a returned report cannot poison a later evaluation', () => {
  const baseline = evaluate(fixture());
  const poisoned = evaluate(fixture());
  poisoned.missing_decisions.length = 0;
  poisoned.consistency_issues.push('INJECTED');
  poisoned.production_authorized = true;
  assert.deepEqual(evaluate(fixture()), baseline);
});

test('missing and unknown keys are rejected at each contract object boundary', () => {
  for (const boundary of [null, 'confirmed_intent', 'decisions']) {
    const input = completeDraft();
    const target = boundary ? input[boundary] : input;
    target.production_authorized = true;
    assertInvalid(encode(input), 'INVALID_STRUCTURE');
    delete target.production_authorized;
    delete target[Object.keys(target)[0]];
    assertInvalid(encode(input), 'INVALID_STRUCTURE');
  }
});

test('fixed asset, draft status and two-year intent cannot be changed into an approved policy', () => {
  for (const [field, value] of [['schema', 'elon.esk.early_support_policy_draft.v2'],
    ['policy_status', 'approved'], ['asset', 'QSHARE']]) {
    const input = completeDraft();
    input[field] = value;
    assertInvalid(encode(input), 'INVALID_VALUE');
  }
  for (const key of Object.keys(fixture().confirmed_intent)) {
    const input = completeDraft();
    input.confirmed_intent[key] = key === 'policy_duration_years' ? 3 : false;
    assertInvalid(encode(input), 'INVALID_VALUE');
  }
});

test('types, decision enum values, denomination and responsibility identifiers are bounded', () => {
  for (const field of DECISIONS) {
    for (const value of [true, 0, [], {}]) {
      const input = completeDraft();
      input.decisions[field] = value;
      assertInvalid(encode(input), field === 'program_start_at' || field === 'program_end_at'
        ? 'INVALID_DATE' : 'INVALID_VALUE');
    }
  }
  for (const [field, value] of [['protection_scope', 'unlimited_guaranteed'],
    ['term_basis', '730_days'], ['principal_denomination', 'usdt'],
    ['principal_denomination', 'USD/T'], ['principal_denomination', 'A'],
    ['principal_denomination', 'A'.repeat(13)], ['guarantor_id', ' '],
    ['guarantor_id', 'x'.repeat(129)]]) {
    const input = completeDraft();
    input.decisions[field] = value;
    assertInvalid(encode(input), 'INVALID_VALUE');
  }
});

test('free text rejects emptiness, surrounding whitespace and control characters', () => {
  for (const field of ['funding_source', 'funding_description', 'minimum_return_terms']) {
    for (const value of ['', ' ', ' leading', 'trailing ', 'a\nb', 'a\tb', 'a\u0000b',
      'a\u007fb', 'a\u0085b', 'a\u009fb', 'a\u2028b ', 'a\u2029b ', 'a\u2028b\u00a0']) {
      const input = completeDraft();
      if (field === 'minimum_return_terms') input.decisions.protection_scope = 'principal_and_minimum_return';
      input.decisions[field] = value;
      assertInvalid(encode(input), 'INVALID_VALUE');
    }
  }
});

test('text length is measured by Unicode code points rather than UTF-16 units', () => {
  for (const [field, limit] of [['funding_description', 2000], ['funding_source', 512],
    ['minimum_return_terms', 2000]]) {
    const input = completeDraft();
    if (field === 'minimum_return_terms') input.decisions.protection_scope = 'principal_and_minimum_return';
    input.decisions[field] = '🧪'.repeat(limit);
    assert.equal(evaluate(input).review_status, 'ready_for_policy_review');
    input.decisions[field] += '🧪';
    assertInvalid(encode(input), 'INVALID_VALUE');
  }
});

test('UTC timestamps must name real calendar seconds in the exact supported format', () => {
  for (const value of ['2027-02-29T00:00:00Z', '2028-04-31T00:00:00Z',
    '2028-13-01T00:00:00Z', '2028-00-01T00:00:00Z', '2028-02-29T24:00:00Z',
    '2028-02-29T00:60:00Z', '2028-02-29T00:00:60Z', '2028-02-29T00:00:00.000Z',
    '2028-02-29T00:00:00+00:00', '2028-02-29', '2028-02-29T00:00:00z']) {
    const input = completeDraft();
    input.decisions.program_start_at = value;
    assertInvalid(encode(input), 'INVALID_DATE');
  }
});

test('the plan window must end after its start without treating two years as 730 days', () => {
  const input = completeDraft();
  assert.equal(evaluate(input).review_status, 'ready_for_policy_review');
  for (const end of ['2028-02-29T00:00:00Z', '2028-02-28T23:59:59Z']) {
    input.decisions.program_end_at = end;
    assertInvalid(encode(input), 'INVALID_DATE_ORDER');
  }
});
