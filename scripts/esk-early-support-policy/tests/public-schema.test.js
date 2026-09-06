'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const test = require('node:test');
const { ROOT, DECISIONS } = require('./helpers');
const schema = require(path.join(ROOT, 'contracts/esk/early-support-policy-draft-v1.schema.json'));

test('the published input schema makes choices explicit and rejects undeclared fields', () => {
  for (const object of [schema, schema.properties.confirmed_intent, schema.properties.decisions]) {
    assert.equal(object.type, 'object');
    assert.equal(object.additionalProperties, false);
    assert.deepEqual([...object.required].sort(), Object.keys(object.properties).sort());
  }
  assert.deepEqual(schema.properties.decisions.required, DECISIONS);
  assert.equal(JSON.stringify(schema).includes('"default":'), false);
  assert.equal(schema.properties.policy_status.const, 'draft');
  assert.equal(schema.properties.asset.const, 'ESK');
  assert.equal(schema.properties.confirmed_intent.properties.policy_duration_years.const, 2);
});

test('schema decision alternatives match the independently exercised policy combinations', () => {
  const fields = schema.properties.decisions.properties;
  assert.deepEqual(fields.protection_scope.enum, [null, 'principal_only',
    'principal_and_minimum_return', 'limited_funded_loss_support']);
  assert.deepEqual(fields.term_basis.enum, [null, 'program_window', 'per_purchase_anniversary']);
  for (const field of ['principal_denomination', 'guarantor_id']) {
    assert.deepEqual(fields[field].type, ['string', 'null']);
  }
  for (const field of ['program_start_at', 'program_end_at']) {
    assert.equal(fields[field].$ref, '#/$defs/utcTimestamp');
  }
});

test('published text constraints cover Unicode boundary cases faced by other clients', () => {
  for (const [name, maximum] of [['ruleText', 512], ['longText', 2000]]) {
    const definition = schema.$defs[name];
    assert.deepEqual(definition.type, ['string', 'null']);
    assert.equal(definition.minLength, 1);
    assert.equal(definition.maxLength, maximum);
    const pattern = new RegExp(definition.pattern, 'u');
    for (const value of ['资金来源待核实', 'Synthetic 🧪 terms', 'A\u2028B']) {
      assert.equal(pattern.test(value), true);
    }
    for (const value of ['', ' leading', 'trailing ', 'A\u2028B ', 'A\u2029B\u00a0',
      'a\u0085b', 'a\u009fb', 'a\nb', 'a\u0000b']) {
      assert.equal(pattern.test(value), false, `${name} must refuse ${JSON.stringify(value)}`);
    }
  }
});

test('schema timestamps expose the strict UTC shape and document extra calendar checks', () => {
  const definition = schema.$defs.utcTimestamp;
  const pattern = new RegExp(definition.pattern, 'u');
  assert.equal(pattern.test('2028-02-29T00:00:00Z'), true);
  for (const value of ['2028-02-29', '2028-02-29T00:00:00.000Z', '2028-02-29T00:00:00+00:00']) {
    assert.equal(pattern.test(value), false);
  }
  // A shape regex alone cannot decide leap years or compare two field values.
  assert.match(definition.description, /invalid dates/);
  assert.match(definition.description, /non-increasing/);
});
