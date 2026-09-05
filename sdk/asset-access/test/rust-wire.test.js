import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { identityResponse, assetResponse } from '../src/contract.js';

const wirePath = process.env.ELON_ASSET_ACCESS_WIRE_OUTPUT;

test('actual Rust Store serialization passes the independent SDK contract', { skip: !wirePath }, async () => {
  // Produced by the synthetic_delegated_wire_export Rust test, never a live account.
  const wire = JSON.parse(await readFile(wirePath, 'utf8'));
  const now = Date.parse('2026-09-04T10:00:00Z');
  const identity = identityResponse(wire.identity, wire.identity, now);
  const page = assetResponse(wire.page, identity, {
    now, limit: 20, includeProgress: true, previous: null, cursor: null,
  });
  assert.deepEqual(page.balance, {
    total_base_units: '10000000', reserved_base_units: '3', available_base_units: '9999997',
  });
  assert.equal(page.progress.requests.length, 1);
  assert.equal(page.progress.requests[0].amount_base_units, '3');
  assert.equal(page.asset.source, 'platform_recorded');
  assert.equal(page.asset.funds_moved, false);
  assert.equal(Object.hasOwn(identity, 'nickname'), false);
  assert.equal(JSON.stringify(wire).includes('access_token'), false);
});
