'use strict';

// Source identity only. Execute both Android test suites and device acceptance separately.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');

const mainRoot = fs.realpathSync(path.resolve(__dirname, '..'));
const areas = [
  'android/app/src/main/kotlin/com/elon/eskcontract',
  'android/app/src/test/kotlin/com/elon/eskcontract',
];

function boundedFile(root, relative) {
  const candidate = path.join(root, relative);
  const stat = fs.lstatSync(candidate);
  assert.ok(stat.isFile() && !stat.isSymbolicLink() && stat.size <= 131072,
    'BOUNDED_REGULAR_SOURCE_REQUIRED');
  const resolved = fs.realpathSync(candidate);
  const inside = path.relative(root, resolved);
  assert.ok(inside && !path.isAbsolute(inside) && inside !== '..' && !inside.startsWith(`..${path.sep}`),
    'SOURCE_MUST_STAY_IN_REPOSITORY');
  return fs.readFileSync(resolved);
}

function newFiles(root, area) {
  const found = fs.readdirSync(path.join(root, area))
    .filter(name => /^EskPlatformProgress[A-Za-z0-9]*\.kt$/.test(name)).sort();
  assert.ok(found.length > 0 && found.length <= 20, 'BOUNDED_PROGRESS_SOURCE_SET_REQUIRED');
  return found;
}

function check(quantRoot) {
  assert.notEqual(quantRoot.toLowerCase(), mainRoot.toLowerCase(), 'SEPARATE_QUANT_REPOSITORY_REQUIRED');
  const receipts = [];
  for (const [index, area] of areas.entries()) {
    const names = newFiles(mainRoot, area);
    assert.deepEqual(names, newFiles(quantRoot, area), 'PROGRESS_SOURCE_SET_MISMATCH');
    assert.ok(names.includes(index === 0 ? 'EskPlatformProgressContract.kt' : 'EskPlatformProgressContractTest.kt'));
    const legacy = index === 0
      ? ['EskSnapshotContract.kt', 'EskPlatformSnapshotContract.kt']
      : ['EskSnapshotContractTest.kt', 'EskPlatformSnapshotContractTest.kt'];
    for (const name of [...names, ...legacy]) {
      const relative = `${area}/${name}`;
      const main = boundedFile(mainRoot, relative);
      const quant = boundedFile(quantRoot, relative);
      assert.ok(main.equals(quant), `CONTRACT_BYTE_MISMATCH: ${name}`);
      receipts.push({ file: relative, bytes: main.length,
        sha256: createHash('sha256').update(main).digest('hex') });
    }
  }
  return { schema: 'yilong.esk.platform_progress_source_parity.v1', status: 'passed',
    scope: 'byte_identity_only', runtime_verified: false, user_acceptance: false,
    financial_authority: false, files: receipts };
}

try {
  assert.equal(process.argv.length, 3, 'USAGE: node scripts/check-esk-platform-progress-parity.js <quant-root>');
  console.log(JSON.stringify(check(fs.realpathSync(path.resolve(process.argv[2])))));
} catch (error) {
  // No source contents, credentials or user data are part of a receipt.
  console.error(`ESK progress source parity failed: ${error.code || error.message}`);
  process.exitCode = 1;
}
