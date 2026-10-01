import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parseScanPayload, friendQrPayload } from './scanPayload.mjs';
const fixtures = JSON.parse(readFileSync(new URL('./payload-fixtures.json', import.meta.url)));
for (const [index, fixture] of fixtures.entries()) test(`payload contract ${index}: ${fixture.kind}`, () => {
  const result = parseScanPayload(fixture.raw);
  assert.equal(result.kind, fixture.kind);
  if (fixture.target) assert.equal(result.target, fixture.target);
  if (['text', 'wifi', 'contact'].includes(result.kind)) assert.equal(result.target, '');
});
test('empty and oversized input fail, typed friend payload round trips', () => {
  for (const value of ['', null, 'x'.repeat(8193)]) assert.throws(() => parseScanPayload(value));
  const id = '0123456789abcdef0123456789abcdef';
  assert.equal(parseScanPayload(friendQrPayload(id)).target, id);
  assert.throws(() => friendQrPayload('javascript:evil'));
});
