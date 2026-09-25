'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const script = path.join(__dirname, 'research-chatgpt-rspack.cjs');

function run(t, files, args) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rspack-source-test-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  for (const [name, body] of Object.entries(files)) fs.writeFileSync(path.join(dir, name), body);
  return spawnSync(process.execPath, [script, dir, ...args], { encoding: 'utf8', timeout: 10000 });
}

test('reports malformed filtered sources without hiding valid module results', t => {
  const r = run(t, { '1.aa.js': 'export const __webpack_modules__ = { Sample: function() { return 1; } };',
    '2.bb.js': 'const x = ;' }, ['index', 'module:Sample']);
  assert.equal(r.status, 0, r.stderr);
  const result = JSON.parse(r.stdout);
  assert.equal(result.matched, 1);
  assert.equal(result.items[0].module, 'Sample');
  assert.equal(result.parseGaps.length, 1);
  assert.equal(result.parseGaps[0].file, '2.bb.js');
});

test('accepts an explicitly observed manifest without fetching existing assets', t => {
  const entry = { module: '/cdn/assets/1.aa.js', imports: [] };
  const manifest = { entry, routes: Object.fromEntries(['root', 'home.route',
    'conversation-layout.route', 'conversation.route'].map(name => [name, entry])) };
  const r = run(t, { 'manifest-abcdef.js': 'window.manifest=' + JSON.stringify(manifest),
    '1.aa.js': 'throw Error("source must never execute");' }, ['download', 'manifest-abcdef.js']);
  assert.equal(r.status, 0, r.stderr);
  assert.equal(JSON.parse(r.stdout).publicAssets, 1);
});

test('rejects manifest paths outside the evidence directory', t => {
  const r = run(t, {}, ['download', '../manifest-abcdef.js']);
  assert.equal(r.status, 1);
  assert.match(r.stderr, /observed manifest basename required/);
});
