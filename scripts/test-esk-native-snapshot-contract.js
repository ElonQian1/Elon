'use strict';

// Static parity only. Kotlin behavior is executed by the separate JUnit suite.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');

const ROOT = path.resolve(__dirname, '..');
const FILES = {
  contract: 'android/app/src/main/kotlin/com/elon/eskcontract/EskSnapshotContract.kt',
  tests: 'android/app/src/test/kotlin/com/elon/eskcontract/EskSnapshotContractTest.kt',
  document: 'docs/contracts/esk-android-snapshot-v1.md',
};
const KEYS = [
  'protocol', 'nonce', 'asset_id', 'symbol', 'mode', 'issuance_mode', 'chain_status',
  'simulated', 'funds_moved', 'total', 'available', 'reserved_for_sellback',
  'reserved_for_quant', 'reserved_total', 'revision', 'observed_elapsed_ms', 'expires_elapsed_ms',
];
const FIXTURE = Object.freeze({
  protocol: 'yilong.esk.android_snapshot.v1', nonce: 'a'.repeat(64),
  asset_id: 'esk', symbol: 'ESK', mode: 'paper', issuance_mode: 'paper_recorded',
  chain_status: 'not_deployed', simulated: 'true', funds_moved: 'false',
  total: '1250.000000', available: '900.000000', reserved_for_sellback: '100.000000',
  reserved_for_quant: '250.000000', reserved_total: '350.000000', revision: '7',
  observed_elapsed_ms: '2000', expires_elapsed_ms: '62000',
});

function boundedRead(root, relativePath) {
  const filename = path.join(root, relativePath);
  const metadata = fs.statSync(filename);
  assert.ok(metadata.isFile() && metadata.size <= 131072, 'BOUNDED_SOURCE_REQUIRED');
  return fs.readFileSync(filename);
}

function between(source, start, end) {
  const startAt = source.indexOf(start);
  const endAt = source.indexOf(end, startAt + start.length);
  assert.ok(startAt >= 0 && endAt > startAt, 'STATIC_MARKERS_REQUIRED');
  assert.equal(source.indexOf(start, startAt + start.length), -1, 'DUPLICATE_STATIC_MARKER');
  return source.slice(startAt + start.length, endAt);
}

function literalSet(source, name) {
  const match = source.match(new RegExp(`val ${name} = setOf\\(([\\s\\S]*?)\\)`));
  assert.ok(match, 'STATIC_KEY_SET_REQUIRED');
  const items = [...match[1].matchAll(/"([^"\\]*)"/g)].map(item => item[1]);
  assert.equal(new Set(items).size, items.length, 'DUPLICATE_STATIC_KEY');
  return items.sort();
}

function staticChecks(buffers) {
  const source = buffers.contract.toString('utf8');
  const tests = buffers.tests.toString('utf8');
  const document = buffers.document.toString('utf8');
  assert.deepEqual(literalSet(source, 'KEYS'), [...KEYS].sort());
  assert.deepEqual(literalSet(source, 'REQUEST_KEYS'), ['nonce', 'protocol']);
  for (const [name, expected] of [
    ['PROTOCOL', FIXTURE.protocol], ['ACTION', 'com.elon.app.action.READ_ESK_SNAPSHOT'],
  ]) {
    const value = source.match(new RegExp(`const val ${name} = "([^"\\n]+)"`));
    assert.ok(value, 'STATIC_CONSTANT_REQUIRED');
    assert.equal(value[1], expected);
  }
  for (const [name, expected] of [['REQUEST_WINDOW_MS', 120000], ['DISPLAY_WINDOW_MS', 60000]]) {
    const value = source.match(new RegExp(`const val ${name} = ([0-9_]+)L`));
    assert.ok(value, 'STATIC_WINDOW_REQUIRED');
    assert.equal(Number(value[1].replace(/_/g, '')), expected);
  }
  const kotlinFixture = between(tests, '// SYNTHETIC_WIRE_FIXTURE_START:', '// SYNTHETIC_WIRE_FIXTURE_END');
  const entries = [...kotlinFixture.matchAll(/"([^"\\]+)" to "([^"\\]*)"/g)]
    .map(match => [match[1], match[2]]);
  assert.equal(entries.length, 17);
  assert.equal(new Set(entries.map(([key]) => key)).size, 17);
  assert.deepEqual(Object.fromEntries(entries), FIXTURE);
  const fixtureBlock = between(document,
    '<!-- SYNTHETIC_WIRE_FIXTURE_START -->', '<!-- SYNTHETIC_WIRE_FIXTURE_END -->');
  const fixtureJson = fixtureBlock.match(/```json\s*([\s\S]*?)\s*```/);
  assert.ok(fixtureJson, 'DOCUMENT_FIXTURE_REQUIRED');
  assert.deepEqual(JSON.parse(fixtureJson[1]), FIXTURE);
  const fieldsBlock = between(document, '<!-- SNAPSHOT_FIELDS_START -->', '<!-- SNAPSHOT_FIELDS_END -->');
  const documented = [...fieldsBlock.matchAll(/^\| `([^`]+)` \|/gm)].map(match => match[1]);
  assert.deepEqual(documented.sort(), [...KEYS].sort());
  assert.deepEqual(Object.keys(FIXTURE).sort(), [...KEYS].sort());
  assert.ok(Object.values(FIXTURE).every(value => typeof value === 'string' && value.length <= 128));
}

function main(args) {
  assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--quant-root' && args[1]),
    'USAGE_TEST_ESK_NATIVE_SNAPSHOT_CONTRACT');
  const buffers = Object.fromEntries(Object.entries(FILES).map(([name, relativePath]) =>
    [name, boundedRead(ROOT, relativePath)]));
  staticChecks(buffers);
  const hashes = Object.fromEntries(Object.entries(buffers).map(([name, buffer]) =>
    [name, createHash('sha256').update(buffer).digest('hex')]));
  if (args.length) {
    const quantRoot = path.resolve(args[1]);
    assert.notEqual(fs.realpathSync(quantRoot), fs.realpathSync(ROOT), 'DISTINCT_REPOSITORIES_REQUIRED');
    for (const [name, relativePath] of Object.entries(FILES)) {
      assert.ok(buffers[name].equals(boundedRead(quantRoot, relativePath)), 'CROSS_REPOSITORY_BYTE_DRIFT');
    }
  }
  console.log(JSON.stringify({
    schema: 'yilong.esk.android_snapshot.static_check.v1', status: 'passed', field_count: 17,
    fixture: 'synthetic_strings_only', cross_repository: args.length ? 'byte_identical' : 'not_performed',
    sha256: hashes, kotlin_execution: 'not_performed_by_this_script',
    device_verification: 'not_performed', network_requests: 0, funds_moved: false,
  }));
}

try {
  main(process.argv.slice(2));
} catch {
  console.error('ESK_NATIVE_SNAPSHOT_STATIC_CHECK_FAILED');
  process.exitCode = 1;
}
