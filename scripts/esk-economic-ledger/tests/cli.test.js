'use strict';

const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const test = require('node:test');
const { MAX_INPUT_BYTES } = require('../preview');
const { ROOT, CLI, batch, encode, evaluate, completePolicy, FALSE_FIELDS,
  assertReport, assertFalseFlags } = require('./helpers');

function run(input, args = []) {
  const environment = { ...process.env };
  delete environment.NODE_OPTIONS;
  const result = spawnSync(process.execPath, [CLI, ...args], {
    input, cwd: ROOT, env: environment, encoding: 'utf8', timeout: 5000,
    maxBuffer: 8192, windowsHide: true,
  });
  assert.ifError(result.error);
  assert.equal(result.signal, null);
  assert.equal(result.stderr, '');
  assert.ok(Buffer.byteLength(result.stdout) < 4096);
  assert.equal(result.stdout.trim().split('\n').length, 1);
  assert.equal(result.stdout.includes(ROOT), false);
  return { status: result.status, report: JSON.parse(result.stdout) };
}

function assertError(input, code, args = []) {
  const result = run(input, args);
  assert.equal(result.status, 2);
  assertFalseFlags(result.report);
  assert.deepEqual(result.report, {
    schema: 'elon.esk.economic_ledger_preview_error.v1', policy_status: 'PENDING',
    ...Object.fromEntries(FALSE_FIELDS.map(field => [field, false])), error: { code },
  });
}

test('real stdin CLI returns the same consistent report as the public API', () => {
  const input = batch();
  const result = run(encode(input));
  assert.equal(result.status, 0);
  assertReport(result.report);
  assert.deepEqual(result.report, evaluate(input));
});

test('completed policy never changes pending status or execution flags in CLI output', () => {
  const input = batch();
  completePolicy(input);
  const result = run(encode(input));
  assert.equal(result.status, 0);
  assertReport(result.report);
  assert.equal(result.report.policy_review_status, 'ready_for_policy_review');
  const serialized = JSON.stringify(result.report);
  for (const marker of ['PRIVATE_SYNTHETIC_POLICY_281746', 'synthetic-not-real',
    'a'.repeat(64), '1'.repeat(64), '2'.repeat(64), 'synthetic-row-1', 'request-1']) {
    assert.equal(serialized.includes(marker), false, marker);
  }
});

test('business inconsistency exits two and suppresses all aggregate totals', () => {
  const input = batch();
  input.journal[1].amount_base_units = '21';
  const result = run(encode(input));
  assert.equal(result.status, 2);
  assertReport(result.report);
  assert.equal(result.report.review_status, 'needs_review');
  assert.equal(result.report.totals, null);
  assert.ok(result.report.issues.includes('LOT_OVERALLOCATED'));
});

test('empty, malformed, duplicate-key and invalid UTF-8 streams fail with bounded fixed codes', () => {
  assertError(Buffer.alloc(0), 'INVALID_JSON');
  assertError(Buffer.from('{"PRIVATE_RAW_CONTENT_2819":'), 'INVALID_JSON');
  assertError(Buffer.from('{"PRIVATE_RAW_CONTENT_2819":1,"PRIVATE_RAW_CONTENT_2819":2}'), 'DUPLICATE_JSON_KEY');
  assertError(Buffer.from([0xc0, 0xaf]), 'INVALID_UTF8');
});

test('stdin enforces the inherited one-MiB limit without truncation', () => {
  const content = encode(batch());
  const boundary = Buffer.concat([content, Buffer.alloc(MAX_INPUT_BYTES - content.length, 0x20)]);
  const result = run(boundary);
  assert.equal(result.status, 0);
  assert.deepEqual(result.report, evaluate(batch()));
  assertError(Buffer.concat([boundary, Buffer.from(' ')]), 'INPUT_TOO_LARGE');
});

test('unknown arguments never activate file input, publication or money movement', () => {
  for (const args of [['--input', 'PRIVATE_FILE_PATH_29418.json'], ['--approve'], ['--publish'],
    ['--help', 'extra'], ['--commit'], ['--funds-moved', 'true']]) {
    assertError(encode(batch()), 'INVALID_ARGUMENTS', args);
  }
});

test('help is stable JSON and does not require or expose a document', () => {
  const result = run(Buffer.alloc(0), ['--help']);
  assert.equal(result.status, 0);
  assert.deepEqual(result.report, {
    schema: 'elon.esk.economic_ledger_preview_cli_help.v1',
    usage: 'node scripts/esk-economic-ledger/cli.js < input.json',
    max_input_bytes: MAX_INPUT_BYTES, offline: true,
  });
});
