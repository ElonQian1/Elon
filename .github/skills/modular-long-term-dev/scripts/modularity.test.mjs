import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { git } from './git-snapshot.mjs';
import { classify, glob, measure, parseConfig } from './policy.mjs';

const cli = fileURLToPath(new URL('./modularity.mjs', import.meta.url));
function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'modularity-test-'));
  const parent = fs.realpathSync(os.tmpdir());
  t.after(() => {
    const resolved = fs.realpathSync(root);
    assert.equal(path.dirname(resolved), parent);
    assert.ok(path.basename(resolved).startsWith('modularity-test-'));
    fs.rmSync(resolved, { recursive: true, force: true });
  });
  git(root, ['init', '--quiet']);
  git(root, ['config', 'user.email', 'test@example.invalid']);
  git(root, ['config', 'user.name', 'Modularity test']);
  git(root, ['config', 'core.autocrlf', 'false']);
  // Fixtures must not execute a machine's custom hook configuration.
  git(root, ['config', 'core.hooksPath', '.no-test-hooks']);
  const write = (file, value) => {
    fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
    fs.writeFileSync(path.join(root, file), value);
  };
  const run = (...args) => {
    const out = spawnSync(process.execPath, [cli, ...args, '--root', root], { encoding: 'utf8', windowsHide: true });
    if (out.error) throw out.error;
    return { status: out.status, output: out.stdout + out.stderr };
  };
  const commit = () => {
    git(root, ['add', '--all']);
    git(root, ['commit', '--quiet', '-m', 'fixture']);
    return git(root, ['rev-parse', 'HEAD']).trim();
  };
  return { root, write, run, commit };
}
function status(result, expected) { assert.equal(result.status, expected, result.output); }
const lines = count => 'x\n'.repeat(count);

test('untracked Unicode files are checked; empty repo and exact threshold pass', t => {
  const f = fixture(t);
  status(f.run('check'), 0);
  f.write('src/中文 空格.ts', lines(800));
  status(f.run('check'), 0);
  f.write('src/中文 空格.ts', lines(801));
  status(f.run('check'), 1);
});

test('staged content cannot be hidden by smaller unstaged content', t => {
  const f = fixture(t);
  f.write('src/feature.ts', lines(801));
  git(f.root, ['add', '--all']);
  f.write('src/feature.ts', lines(2));
  status(f.run('check'), 0);
  status(f.run('check', '--staged'), 1);
  git(f.root, ['add', '--all']);
  status(f.run('check', '--staged'), 0);
});

test('historical giants are adopted once, cannot grow, and ratchet only down', t => {
  const f = fixture(t);
  f.write('src/legacy.rs', lines(900));
  status(f.run('baseline', '--reason', 'Existing historical debt'), 0);
  status(f.run('baseline', '--reason', 'Overwrite'), 2);
  const base = f.commit();
  status(f.run('check', '--base', base), 0);
  f.write('src/legacy.rs', lines(901));
  status(f.run('check'), 1);
  status(f.run('ratchet'), 2);
  f.write('src/legacy.rs', lines(850));
  status(f.run('ratchet'), 0);
  f.commit();
  f.write('src/legacy.rs', lines(851));
  status(f.run('check'), 1);
  f.write('src/legacy.rs', lines(800));
  status(f.run('ratchet'), 0);
  const debt = JSON.parse(fs.readFileSync(path.join(f.root, '.modularity-baseline.json')));
  assert.deepEqual(debt.files, {});
});

test('base ref tightens actual prior size and prevents baseline inflation', t => {
  const f = fixture(t);
  f.write('src/legacy.rs', lines(900));
  status(f.run('baseline', '--reason', 'Historical'), 0);
  f.commit();
  f.write('src/legacy.rs', lines(850));
  const smaller = f.commit();
  f.write('src/legacy.rs', lines(875));
  status(f.run('check', '--base', smaller), 1);
  const file = path.join(f.root, '.modularity-baseline.json');
  const debt = JSON.parse(fs.readFileSync(file));
  debt.files['src/legacy.rs'].lines = 999;
  fs.writeFileSync(file, JSON.stringify(debt));
  const result = f.run('check', '--base', smaller);
  status(result, 1);
  assert.match(result.output, /baseline-expanded/);
});

test('first adoption explicitly permits current uncommitted historical debt', t => {
  const f = fixture(t);
  f.write('src/legacy.rs', lines(700));
  const base = f.commit();
  f.write('src/legacy.rs', lines(900));
  status(f.run('baseline', '--reason', 'Adopt work in progress'), 0);
  status(f.run('check', '--base', base), 0);
  f.commit();
  f.write('src/new.rs', lines(801));
  status(f.run('check'), 1);
});

test('renaming an over-limit file does not create a new exception', t => {
  const f = fixture(t);
  f.write('src/legacy.rs', lines(900));
  status(f.run('baseline', '--reason', 'Historical'), 0);
  const base = f.commit();
  git(f.root, ['mv', 'src/legacy.rs', 'src/renamed.rs']);
  status(f.run('check', '--staged', '--base', base), 1);
  fs.unlinkSync(path.join(f.root, 'src/renamed.rs'));
  status(f.run('check'), 0);
});

test('ignores generated and Git-ignored content while tracking source', t => {
  const f = fixture(t);
  f.write('.gitignore', 'local/\n');
  f.write('node_modules/dependency.js', lines(900));
  f.write('web/generated/bindings.ts', lines(900));
  f.write('local/scratch.ts', lines(900));
  f.write('.modularity.json', JSON.stringify({ version: 1, exclude: [{ pattern: 'export/**', reason: 'Generated export' }] }));
  f.write('export/script.ts', lines(900));
  status(f.run('check'), 0);
  f.write('web/feature.ts', lines(801));
  status(f.run('check'), 1);
});

test('document and source byte limits catch giant single lines', t => {
  const f = fixture(t);
  f.write('docs/topic.md', '中'.repeat(16667));
  status(f.run('check'), 1);
  f.write('docs/topic.md', 'small');
  f.write('web/feature.ts', 'x'.repeat(128001));
  status(f.run('check'), 1);
});

test('invalid refs, configuration, and UTF-8 fail explicitly', t => {
  const f = fixture(t);
  f.write('src/file.ts', 'small');
  f.commit();
  status(f.run('check', '--base', 'does-not-exist'), 2);
  f.write('.modularity.json', '{"version":1,"excldue":[]}');
  status(f.run('check'), 2);
  f.write('.modularity.json', '{"version":1}');
  f.write('src/file.ts', Buffer.from([0xff]));
  status(f.run('check'), 2);
});

test('staged policy and baseline are read from index', t => {
  const f = fixture(t);
  f.write('src/feature.ts', lines(801));
  git(f.root, ['add', '--all']);
  status(f.run('baseline', '--reason', 'Unstaged baseline'), 0);
  status(f.run('check'), 0);
  status(f.run('check', '--staged'), 1);
  git(f.root, ['add', '.modularity-baseline.json']);
  status(f.run('check', '--staged'), 0);
});

test('role budgets and portable line counting', () => {
  const config = parseConfig(null);
  assert.equal(classify('web/main.ts', config).maxLines, 500);
  assert.equal(classify('src/module/tests.rs', config).maxLines, 1000);
  assert.equal(classify('scripts/test-world.mjs', config).role, 'test');
  assert.equal(classify('src/network_types.rs', config).role, 'schema');
  assert.deepEqual(measure(Buffer.from('')), { lines: 0, bytes: 0 });
  assert.deepEqual(measure(Buffer.from('a\r\nb\r\n')), measure(Buffer.from('a\nb\n')));
  assert.equal(measure(Buffer.from('a\nb')).lines, 2);
  assert.ok(glob('**/test*.ts').test('test.ts'));
  assert.ok(glob('**/test*.ts').test('src/tests.ts'));
  assert.ok(!glob('src/*.ts').test('src/nested/a.ts'));
});

test('symbolic links are never followed', t => {
  const f = fixture(t);
  f.write('outside-data.txt', lines(900));
  try { fs.symlinkSync(path.join(f.root, 'outside-data.txt'), path.join(f.root, 'link.ts'), 'file'); }
  catch (error) {
    if (error.code === 'EPERM') { t.skip('Symlink privilege unavailable'); return; }
    throw error;
  }
  status(f.run('check'), 0);
});
