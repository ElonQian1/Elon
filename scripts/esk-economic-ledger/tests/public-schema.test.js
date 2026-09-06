'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const { ROOT, FALSE_FIELDS, batch, evaluate, assertReport } = require('./helpers');
const schema = JSON.parse(fs.readFileSync(path.join(ROOT,
  'contracts/esk/economic-ledger-preview-v1.schema.json'), 'utf8'));

test('the shipped synthetic fixture remains consistent while every obligation stays unknown', () => {
  const input = JSON.parse(fs.readFileSync(path.join(ROOT,
    'contracts/esk/economic-ledger-preview-v1.fixture.json'), 'utf8'));
  const report = evaluate(input);
  assertReport(report);
  assert.equal(report.review_status, 'consistent');
  assert.equal(report.policy_review_status, 'needs_decisions');
  assert.equal(report.policy_missing_decisions.length, 14);
  assert.deepEqual(report.issues, []);
  assert.equal(report.totals.funding_total_base_units, '180');
  assert.equal(report.totals.investment_base_units, '80');
  assert.equal(report.totals.guarantee_reserve_base_units, '70');
  assert.equal(report.totals.profit_distribution_base_units, '10');
  assert.equal(report.totals.unallocated_base_units, '20');
  assert.equal(input.obligation_links.length, 1);
  for (const obligation of input.obligation_links) {
    assert.equal(obligation.status, 'PENDING');
    assert.equal(obligation.protected_principal_base_units, null);
    assert.equal(obligation.minimum_return_base_units, null);
  }
});

test('published identifiers, amounts, digests and references require an absolute string end', () => {
  for (const [name, valid] of [['id', 'synthetic-id'], ['key', 'provider:synthetic'],
    ['digest', 'a'.repeat(64)], ['amount', '123'], ['totalAmount', '0']]) {
    const pattern = new RegExp(schema.$defs[name].pattern, 'u');
    assert.equal(pattern.test(valid), true, name);
    for (const suffix of ['\n', '\r', '\r\n', '\u2028', '\u2029', '\u0085']) {
      assert.equal(pattern.test(valid + suffix), false, `${name} trailing ${JSON.stringify(suffix)}`);
    }
  }
  const reference = schema.$defs.fundingLot.properties.external_payment_reference;
  assert.equal(reference.$ref, '#/$defs/key');
});

test('input contract has closed shapes and exactly the intended proposal operations', () => {
  for (const shape of [schema, schema.$defs.fundingLot, schema.$defs.obligationLink,
    schema.$defs.propose, schema.$defs.cancel]) {
    assert.equal(shape.additionalProperties, false);
    assert.deepEqual([...shape.required].sort(), Object.keys(shape.properties).sort());
  }
  assert.equal(schema.properties.mode.const, 'offline_draft');
  assert.equal(schema.$defs.propose.properties.operation.const, 'propose');
  assert.equal(schema.$defs.cancel.properties.operation.const, 'cancel');
  assert.deepEqual(schema.$defs.propose.properties.purpose.enum,
    ['investment', 'guarantee_reserve', 'profit_distribution']);
  assert.deepEqual(schema.$defs.fundingLot.properties.origin.enum,
    ['esk_purchase', 'sponsor_capital', 'realized_profit']);
});

test('public report and error schemas fix all execution flags and unknown obligations', () => {
  for (const shape of [schema.$defs.report, schema.$defs.error]) {
    assert.equal(shape.additionalProperties, false);
    assert.equal(shape.properties.policy_status.const, 'PENDING');
    for (const field of FALSE_FIELDS) {
      assert.equal(shape.properties[field].const, false, field);
      assert.ok(shape.required.includes(field));
    }
  }
  const obligation = schema.$defs.obligationLink.properties;
  assert.equal(obligation.status.const, 'PENDING');
  assert.equal(obligation.protected_principal_base_units.const, null);
  assert.equal(obligation.minimum_return_base_units.const, null);
  assert.equal(schema.$defs.report.else.properties.totals.type, 'null');
  assert.equal(schema.$defs.report.then.properties.totals.$ref, '#/$defs/totals');
});

test('actual reports fit the published field and issue vocabulary without leaking private records', () => {
  const input = batch();
  for (const change of [() => {}, () => { input.obligation_links = []; },
    () => { input.journal[1].amount_base_units = '21'; }]) {
    change();
    const report = evaluate(input);
    assertReport(report);
    assert.deepEqual(Object.keys(report).sort(), [...schema.$defs.report.required].sort());
    assert.deepEqual(Object.keys(report.counts).sort(), [...schema.$defs.counts.required].sort());
    for (const issue of report.issues) assert.ok(schema.$defs.issue.enum.includes(issue));
    if (report.totals) {
      assert.deepEqual(Object.keys(report.totals).sort(), [...schema.$defs.totals.required].sort());
    }
    assert.equal(Object.hasOwn(report, 'rows'), false);
    assert.equal(Object.hasOwn(report, 'funding_lots'), false);
    assert.equal(Object.hasOwn(report, 'journal'), false);
  }
});
