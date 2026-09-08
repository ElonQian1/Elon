'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { fixture, flush } = require('./fixtures/chatgpt-runtime-stop');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const adapter = fs.readFileSync(path.join(assets, 'chatgpt_web_adapter.js'), 'utf8');
const findSource = adapter.match(/  function findComposer\(\) \{[\s\S]*?(?=\n  function composerValue)/)?.[0];
assert.ok(findSource);

test('the production stop path binds an ID-only rich composer while editing is disabled', async () => {
  const f = fixture();
  const find = vm.runInNewContext('(' + findSource.trim() + ')', {
    document: { querySelectorAll: selector => selector === '#prompt-textarea' ? [f.node] : [] },
    isVisible: node => node === f.node
  });
  const composer = find();
  assert.equal(composer, f.node, 'a disabled composer still owns the active official request');
  const transaction = f.api.stop({ ...f.command, composer });
  assert.equal(transaction.handled, true); await flush();
  assert.equal(f.calls.length, 1);
  f.setState(false, null); f.settle();
  assert.equal((await transaction.completion).code, 'stop_observed');
});

test('fixed ID does not select a hidden composer over the current visible editor', () => {
  const hidden = {}, visible = {};
  const find = vm.runInNewContext('(' + findSource.trim() + ')', {
    document: { querySelectorAll: selector => selector === '#prompt-textarea' ? [hidden] :
      selector === 'form [contenteditable="true"]' ? [visible] : [] },
    isVisible: node => node === visible
  });
  assert.equal(find(), visible);
});

for (const [code, change] of Object.entries({
  disabled: f => { f.page.__elonChatGptPrivateTextTransactionsEnabled = false; },
  invalid_command: f => { f.command.requestId = 'not a command'; },
  composer_unavailable: f => { f.command.composer = null; },
  context_unavailable: f => { f.top.stateNode.current = {}; },
  request_unavailable: f => { f.props.currentRequestId = null; },
  runtime_not_observed: f => { f.page.performance.getEntriesByName = () => []; }
})) test('a declined stop retains only its bounded diagnostic stage: ' + code, () => {
  const f = fixture();
  // Keep composer-runtime evidence while hiding only stop-specific resources.
  if (code === 'runtime_not_observed') {
    f.page.__elonChatGptPrivateRuntimeBindings = {
      observed: url => url.includes('/8b34dbc2-'), peek: () => null
    };
  } else change(f);
  assert.equal(f.api.stop(f.command).handled, false);
  assert.equal(f.api.state().code, code);
  assert.deepEqual(Object.keys(f.api.state()).sort(), ['code', 'pending']);
  assert.equal(f.calls.length, 0);
});

test('diagnostic status tracks an invoked stop through observation, not just HTTP settlement', async () => {
  const f = fixture(), transaction = f.api.stop(f.command); await flush();
  assert.equal(f.api.state().code, 'invoked');
  f.settle(); await flush(); assert.equal(f.api.state().pending, true);
  f.setState(false, null); f.publish(); await transaction.completion;
  assert.equal(f.api.state().code, 'stop_observed');
});

for (const version of [null, 13, 14, 15]) test('stop diagnostics are read-only on probe version ' + version, () => {
  let code = 'composer_unavailable', fetched = 0, delegated = 0;
  const fetch = () => { fetched++; throw Error('diagnostics cannot use the network'); };
  const page = { fetch, __elonChatGptDocumentToken: 'doc_diagnostic_test',
    __elonChatGptAdapterTargetVersion: 304,
    __elonChatGptPrivateStopRuntime: { state: () => ({ pending: false, code }) },
    elonChatGptNative: { postMessage: () => { throw Error('no ambient telemetry'); } } };
  const original = { version, handle: () => { delegated++; return false; },
    marker: {} };
  if (version && version < 15) page.__elonChatGptPrivateResearchProbe = original;
  const source = fs.readFileSync(path.join(assets, 'chatgpt_web_private_research_probe.js'), 'utf8');
  const sandbox = { window: page, location: { origin: 'https://chatgpt.com' }, URL };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_private_protocol_evidence.js'), 'utf8'), sandbox);
  vm.runInNewContext(source, sandbox);
  if (version === 15) vm.runInNewContext(source, sandbox);
  const probe = page.__elonChatGptPrivateResearchProbe, replies = [];
  const run = () => probe.handle('private_protocol_probe', { value: 'stop_runtime_context' },
    (...args) => replies.push(args));
  assert.equal(run(), true);
  assert.deepEqual(replies[0], ['private_protocol_probe', true, 'stop_runtime_context:composer_unavailable']);
  code = 'untrusted-private-value'; run();
  assert.equal(replies[1][2], 'stop_runtime_context:not_observed');
  assert.equal(fetched, 0);
  if (version && version < 15) {
    assert.equal(page.fetch, fetch, 'upgrades do not stack fetch observers');
    assert.equal(probe.marker, original.marker);
    probe.handle('unrelated', {}, () => {}); assert.equal(delegated, 1);
  }
});
