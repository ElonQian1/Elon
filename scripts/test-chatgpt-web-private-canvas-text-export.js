'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path');
const text = require('../android/app/src/main/assets/chatgpt_web_private_canvas_text_export.js');
const policy = require('../android/app/src/main/assets/chatgpt_web_private_canvas_document_policy.js');
const actions = require('../android/app/src/main/assets/chatgpt_web_canvas_document_actions.js');
const { setup, PATH, ID } = require('./fixtures/chatgpt-web-private-canvas-export.cjs');

function runtime(options = {}) {
  const calls = [], loaded = [], inits = [];
  const plugins = { hive() {}, strip() {}, remark: [() => {}] };
  const pipeline = { use(...args) { calls.push(args); return this; }, process: async content => {
    await options.process?.(); return { toString: () => options.result ?? '  converted\n' };
  } };
  const modules = [
    { IAn: () => inits.push('unified'), LAn: { unified: () => pipeline } },
    { n: () => inits.push('remark'), r: { CANVAS_REMARK_PLUGINS: plugins.remark } },
    { t: () => inits.push('strip'), r: { stripDirectivePlugin: plugins.strip } },
    { i: () => inits.push('hive'), r: { hiveLogDirectivePlugin: plugins.hive } }
  ];
  const loadRuntime = async url => { const i = loaded.length % 4; loaded.push(url); await options.load?.(); return modules[i]; };
  const factory = { ...text, create: page => text.create(page, { loadRuntime, timeoutMs: options.timeoutMs }) };
  return { factory, calls, loaded, inits, plugins, modules };
}

for (const [type, info] of Object.entries(text.sourceTypes)) test('exact source bytes and official format: ' + type, async () => {
  const f = setup({ format: 'source' });
  const content = '\ufeff\t <html>\r\n  synthetic \u4e2d\u6587 \ud83d\ude03 :contentReference[1]  \n';
  Object.assign(f.rows[0], { content, textdoc_type: type, title: 'X'.repeat(512) });
  const value = await f.prepare(); assert.equal(value.ok, true);
  assert.equal(value.exportFile.format, 'source'); assert.equal(value.exportFile.file.mediaType, info.mediaType);
  assert.ok(value.exportFile.file.name.endsWith('.' + info.extension)); assert.ok(value.exportFile.file.name.length <= 150);
  assert.equal(text.validFile(value.exportFile.file), true);
  await f.runDownload(value.exportFile);
  assert.equal(f.saved(), true); assert.deepEqual(Buffer.concat(f.stored), Buffer.from(content));
  assert.equal(f.packets.filter(row => row.byteOperation === 'commit').length, 1);
  assert.ok(f.requests.every(row => row.init.method === 'GET')); assert.equal(f.exports.length, 0);
  assert.equal(f.snapshot.composerReady, false);
});

for (const content of ['', '\n# Title\r\n\n**bold** _italic_ `code`\n\n', 'a'.repeat(100000) + '\u4e2d\ud83d\ude03']) {
  test('plain Markdown preserves exact whitespace and UTF-8 without runtime imports: ' + content.length, async () => {
    const r = runtime({ load: () => assert.fail('plain text needs no plugins') }), f = setup({ format: 'md', text: r.factory });
    f.rows[0].content = content;
    const value = await f.prepare(); await f.runDownload(value.exportFile);
    assert.equal(f.saved(), true); assert.deepEqual(Buffer.concat(f.stored), Buffer.from(content));
    assert.equal(value.exportFile.file.mediaType, 'text/markdown'); assert.equal(f.exports.length, 0);
    assert.equal(r.loaded.length, 0);
  });
}

test('known Markdown directives use the exact ordered official pipeline, cached modules, no POST', async () => {
  const r = runtime(), f = setup({ format: 'md', text: r.factory }); f.rows[0].content = 'x :contentReference[1]';
  for (let i = 0; i < 2; i++) {
    const result = await f.prepare(); await f.runDownload(result.exportFile); assert.equal(f.saved(), true);
  }
  assert.equal(r.loaded.length, 4); assert.equal(r.inits.length, 4);
  assert.ok(r.loaded.every(url => url.startsWith('https://chatgpt.com/cdn/assets/')));
  assert.deepEqual(r.calls, [...Array(2)].flatMap(() => [[r.plugins.hive], [r.plugins.strip, { preserve: undefined }], [r.plugins.remark]]));
  assert.deepEqual(Buffer.concat(f.stored), Buffer.from('convertedconverted')); assert.equal(f.exports.length, 0);
});

for (const mode of ['profile', 'missing', 'timeout', 'identity', 'page', 'version', 'cancel', 'invalid_result']) {
  test('special Markdown fails closed before native save: ' + mode, async () => {
    let f;
    const r = runtime({ timeoutMs: 10,
      load: mode === 'timeout' ? () => new Promise(() => {}) : undefined,
      result: mode === 'invalid_result' ? '\ud800' : undefined,
      process: () => {
        if (mode === 'identity') f.headers.Authorization = 'Bearer changed';
        if (mode === 'page') f.navigate('/');
        if (mode === 'version') f.rows[0].version++;
        if (mode === 'cancel') f.download.cancel();
      } });
    f = setup({ format: 'md', text: r.factory }); f.rows[0].content = '::hiveTranscript{timestamp="1"}';
    if (mode === 'profile') f.page.__elonChatGptPrivateRuntimeBindings.state = () => ({ profile_id: 'unknown' });
    if (mode === 'missing') r.modules[2].t = null;
    await f.runDownload((await f.prepare()).exportFile);
    assert.equal(f.saved(), false); assert.ok(!f.packets.some(row => row.byteOperation === 'commit'));
    assert.equal(f.exports.length, 0);
  });
}

test('failed plugin import is recoverable only by a new explicit export, not a hidden retry', async () => {
  let reject = true;
  const r = runtime({ load: () => { if (reject) throw Error('offline'); } });
  const page = { setTimeout, clearTimeout, __elonChatGptPrivateCanvasDocumentPolicy: policy,
    __elonChatGptPrivateRuntimeBindings: { state: () => ({ profile_id: 'web_20260912' }) } };
  const serializer = r.factory.create(page), document = { documentType: 'document', content: 'contentReference' };
  await assert.rejects(serializer.serialize(document, 'md', () => {}), /offline/);
  assert.equal(r.loaded.length, 4); reject = false;
  assert.equal(await serializer.serialize(document, 'md', () => {}), 'converted'); assert.equal(r.loaded.length, 8);
});

test('a stalled serializer releases original-document ownership, and late completion cannot save a file', async () => {
  let release;
  const r = runtime({ timeoutMs: 10, process: () => new Promise(resolve => { release = resolve; }) });
  const f = setup({ format: 'md', text: r.factory }); f.rows[0].content = 'contentReference';
  const result = await f.prepare();
  await f.runDownload(result.exportFile);
  assert.equal(f.saved(), false); assert.equal(f.receipts.at(-1)[2], 'download_transfer_timeout');
  assert.equal(typeof release, 'function');
  const saved = await f.save((await f.list()).ticket);
  assert.equal(saved.ok, true); // Not stuck as canvas_busy after the local converter deadline.
  release(); await new Promise(resolve => setTimeout(resolve, 5));
  assert.ok(!f.packets.some(row => row.byteOperation === 'commit'));
});

test('unknown language, wrong formats, forged file extension and MIME are rejected', async () => {
  for (const type of ['code/unknown', 'webview', 'document']) {
    const f = setup({ format: 'source' }); f.rows[0].textdoc_type = type;
    assert.equal((await f.prepare()).code, 'canvas_export_unsupported');
  }
  const file = { id: 'canvas-export-original-source', name: 'original.py', mediaType: 'text/x-python' };
  assert.equal(text.validFile(file), true);
  for (const patch of [{ name: 'original.html' }, { mediaType: 'text/html' }, { name: '../original.py' },
    { id: 'canvas-export-original-md' }, { name: 'original\u202e.py' }, { name: 'original\u0085.py' }]) {
    assert.equal(text.validFile({ ...file, ...patch }), false);
    const f = setup();
    assert.equal(f.download.registerCanvasExport(PATH, { ...file, ...patch }, () => {}), '');
  }
});

test('native Canvas export action accepts only the two additional typed selections', async () => {
  for (const format of ['md', 'source']) {
    const f = setup({ format }); if (format === 'source') f.rows[0].textdoc_type = 'code/python';
    const list = await f.list(), events = [], api = actions.create(f.page);
    const result = await new Promise(resolve => api.handle('canvas_document', { requestId: 'mcp_text', value: JSON.stringify({
      operation: 'prepare_export', path: PATH, scope: list.scope, ticket: list.ticket, id: ID, format
    }) }, (_, ok, detail) => resolve({ ok, detail }), () => f.snapshot, e => events.push(e)));
    assert.equal(result.ok, true); assert.equal(events[0].exportFile.format, format);
  }
});

test('Android independently validates the same source metadata and production assets load before download ownership', () => {
  const root = path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/');
  const native = fs.readFileSync(path.join(root, 'chatgptweb/ChatGptWebCanvasExportFormats.kt'), 'utf8');
  const matches = [...native.matchAll(/"([a-z]+)" to \("([A-Za-z0-9]+)" to "([a-z+./-]+)"\)/g)];
  assert.deepEqual(Object.fromEntries(matches.map(([, type, extension, mediaType]) => ['code/' + type, { extension, mediaType }])), text.sourceTypes);
  const assets = fs.readFileSync(path.join(root, 'chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  assert.ok(assets.indexOf('chatgpt_web_private_canvas_text_export.js') < assets.indexOf('chatgpt_web_private_file_download.js'));
  for (const file of ['WebChatCanvasEditorView.kt', 'WebChatCanvasManagementCoordinator.kt']) {
    assert.ok(fs.readFileSync(path.join(root, file), 'utf8').includes('ChatGptWebCanvasExportFormats.options(draft.base.documentType)'));
  }
});
