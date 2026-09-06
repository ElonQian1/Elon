'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { test, after } = require('node:test');
const { MAX_INPUT_BYTES } = require('../contract');
const { CLI_PATH, fixture, completeDraft, encode, evaluate, assertDraftOnly } = require('./helpers');

const temporaryRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'esk-policy-cli-test-'));
const ownedFiles = [];

function createFile(name, content) {
  const target = path.join(temporaryRoot, name);
  fs.writeFileSync(target, content, { flag: 'wx' });
  ownedFiles.push(target);
  return target;
}

after(() => {
  // Remove only files created by this test, then the now-empty exact directory.
  for (const target of ownedFiles) fs.unlinkSync(target);
  fs.rmdirSync(temporaryRoot);
});

function cli(args) {
  const environment = { ...process.env };
  delete environment.NODE_OPTIONS;
  const result = spawnSync(process.execPath, [CLI_PATH, ...args], {
    cwd: temporaryRoot, env: environment, encoding: 'utf8',
    timeout: 5000, maxBuffer: 8192, windowsHide: true,
  });
  assert.ifError(result.error);
  assert.equal(result.signal, null);
  assert.equal(result.stderr, '');
  assert.ok(Buffer.byteLength(result.stdout) < 4096);
  assert.equal(result.stdout.trim().split('\n').length, 1);
  assert.equal(result.stdout.includes(temporaryRoot), false);
  return { status: result.status, report: JSON.parse(result.stdout) };
}

function assertError(args, code) {
  const result = cli(args);
  assert.equal(result.status, 2);
  assert.deepEqual(result.report, {
    schema: 'elon.esk.early_support_policy_error.v1', error: { code },
  });
}

test('a real child process reads an unresolved draft without changing input files', () => {
  const content = encode(fixture());
  const inputPath = createFile('unresolved.json', content);
  const result = cli(['--input', inputPath]);
  assert.equal(result.status, 0);
  assertDraftOnly(result.report);
  assert.equal(result.report.review_status, 'needs_decisions');
  assert.deepEqual(result.report, evaluate(fixture()));
  assert.deepEqual(fs.readFileSync(inputPath), content);
});

test('complete and contradictory drafts never become executable authorizations in CLI output', () => {
  const input = completeDraft();
  let inputPath = createFile('complete-private-marker-3291.json', encode(input));
  let result = cli(['--input', inputPath]);
  assert.equal(result.status, 0);
  assertDraftOnly(result.report);
  assert.equal(result.report.review_status, 'ready_for_policy_review');
  assert.equal(JSON.stringify(result.report).includes('3291'), false);
  input.decisions.minimum_return_terms = 'UNAPPROVED_PRIVATE_MINIMUM_RETURN_91824';
  inputPath = createFile('contradictory.json', encode(input));
  result = cli(['--input', inputPath]);
  assert.equal(result.status, 0);
  assertDraftOnly(result.report);
  assert.equal(result.report.review_status, 'needs_correction');
  assert.equal(JSON.stringify(result.report).includes('91824'), false);
});

test('invalid JSON and invalid UTF-8 produce bounded errors without file contents', () => {
  const invalid = createFile('private-invalid.json', '{"PRIVATE_CONTENT_93891":');
  assertError(['--input', invalid], 'INVALID_JSON');
  const invalidUtf8 = createFile('invalid-utf8.json', Buffer.from([0xc0, 0xaf]));
  assertError(['--input', invalidUtf8], 'INVALID_UTF8');
});

test('missing files, directories and over-limit files are distinct bounded failures', () => {
  assertError(['--input', path.join(temporaryRoot, 'PRIVATE_ABSENT_NAME_12893.json')], 'INPUT_READ_FAILED');
  assertError(['--input', temporaryRoot], 'INPUT_NOT_REGULAR_FILE');
  const huge = createFile('too-large.json', Buffer.alloc(MAX_INPUT_BYTES + 1, 0x20));
  assertError(['--input', huge], 'INPUT_TOO_LARGE');
});

test('exact byte-limit files can be read without truncating valid input', () => {
  const content = encode(fixture());
  const padded = Buffer.concat([content, Buffer.alloc(MAX_INPUT_BYTES - content.length, 0x20)]);
  const inputPath = createFile('exact-limit.json', padded);
  const result = cli(['--input', inputPath]);
  assert.equal(result.status, 0);
  assert.deepEqual(result.report, evaluate(fixture()));
});

test('unrecognized arguments cannot enable publication or read from standard input', () => {
  for (const args of [[], ['--input'], ['--publish'], ['--input', '-'],
    ['--help', 'extra'], ['--input', '--broadcast'], ['--input', 'x', '--approve']]) {
    assertError(args, 'INVALID_ARGUMENTS');
  }
});

test('help is stable offline information, with no environment or file path disclosure', () => {
  const result = cli(['--help']);
  assert.equal(result.status, 0);
  assert.deepEqual(result.report, {
    schema: 'elon.esk.early_support_policy_cli_help.v1',
    usage: 'node scripts/esk-early-support-policy/cli.js --input <file>',
    max_input_bytes: MAX_INPUT_BYTES, offline: true,
  });
});
