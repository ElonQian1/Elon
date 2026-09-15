'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { parseSource, roleFiles, compareSymbol } = require('./analyze-chatgpt-runtime-contracts.cjs');
const prior = require('./fixtures/chatgpt-runtime-bindings-sep15.cjs');
const profile = require('./fixtures/chatgpt-runtime-bindings-sep15b.cjs');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;
const previous = process.env.CHATGPT_PRIOR_RUNTIME_DIR;
const hashes = {
  anchor: '13090b4a8f5cf432ec3694a4ab7ed214ece5498171bddf58e4e9708660d3f96e',
  shared: '318a749ff4dfcc37b182d7895b87072387f9d7c9fa0d450cc6bad9e3862bee94',
  conversation: '039efa3e391652942a9ae5de7cc057eb1bc05c3afad32d851e14ac47b554470d',
  composer: '5693a37d5f6eecdecd6bb9e257380cc7594803aeafb9df87b5dbafdf4d03f605',
  react: 'a0e3af9dc19a43aff7b55fe57a6ebca9f8ce108eb599880091abbec880551e9d'
};

test('second Sep 15 rollout resolves all canonical exports and rejects an unknown anchor', async () => {
  const binding = require('../android/app/src/main/assets/chatgpt_web_private_runtime_bindings');
  const cdn = 'https://chatgpt.com/cdn/assets/';
  let anchor = profile.anchor;
  const page = { location: { origin: 'https://chatgpt.com' }, document: {}, setTimeout, clearTimeout,
    __elonChatGptDocumentToken: 'doc_fixture', performance: { getEntriesByName: url => url === cdn + anchor ? [{}] : [] } };
  const namespaces = Object.fromEntries(Object.entries(profile.expectedExports).map(([role, map]) =>
    [role, Object.fromEntries(Object.entries(map).map(([name, exported]) => [exported, { name }]))]));
  const runtime = binding.create(page, { loadRuntime: async url => namespaces[
    Object.keys(profile.files).find(role => url === cdn + profile.files[role])] });
  let count = 0;
  for (const [role, mappings] of Object.entries(profile.expectedExports)) {
    const exposed = await runtime.load(role);
    assert.deepEqual(Object.keys(exposed).sort(), Object.keys(mappings).sort());
    for (const [name, exported] of Object.entries(mappings)) { count++; assert.equal(exposed[name], namespaces[role][exported]); }
  }
  assert.equal(count, 100);
  assert.equal(runtime.state().profile_id, profile.id);
  namespaces.react.kn = { ready: true };
  assert.equal(runtime.peek('react').intlProvider, namespaces.react.kn);
  anchor = 'c2675c8c-unknown.js'; page.__elonChatGptDocumentToken = 'doc_next';
  await assert.rejects(runtime.load('shared'), /runtime_not_observed/);
});

test('hash-pinned source resolves ambiguous dependencies and changed stop ownership', {
  skip: !(directory && previous) && 'Set both current and prior public runtime directories.'
}, () => {
  const modules = {}, old = {};
  for (const [role, file] of Object.entries({ anchor: profile.anchor, ...profile.files })) {
    const bytes = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), hashes[role]);
    modules[role] = parseSource(bytes.toString('utf8'));
    if (prior.files[role]) old[role] = parseSource(fs.readFileSync(path.join(previous, prior.files[role]), 'utf8'));
  }
  assert.deepEqual(roleFiles(modules.anchor.sources, true), profile.files);
  const definition = (role, name, local = false) => {
    const m = modules[role], nodes = m.definitions.get(local ? name : m.exported.get(name));
    assert.equal(nodes?.length, 1, role + '.' + name);
    return m.text.slice(nodes[0].start, nodes[0].end);
  };
  // Same normalized bodies are only the first check; ambiguous getter targets
  // and the changed stop function are independently verified below.
  for (const [role, mappings] of Object.entries(profile.expectedExports)) {
    for (const [name, exported] of Object.entries(mappings)) {
      assert.ok(modules[role].exported.has(exported), role + '.' + name);
      if (role === 'react' || role === 'conversation' && name === 'FVt') continue;
      assert.ok(compareSymbol(old[role], modules[role], prior.expectedExports[role][name])
        .candidates.some(candidate => candidate.exports.includes(exported)), role + '.' + name);
    }
  }
  assert.equal(fs.readFileSync(path.join(previous, prior.files.react), 'utf8'), modules.react.text);
  const stop = definition('conversation', 'HWt');
  assert.match(stop, /clientInitiated:n=!1,clientStopReason:r,isLastTurnAgent:i=void 0,hasAgentSystemHint:a=void 0/);
  assert.match(stop, /stopConduitToken/);
  assert.match(stop, /i\[`x-conduit-token`\]=_/);
  assert.match(stop, /safePost\(`\/stop_conversation`,\{\.\.\.l\}\)/);
  assert.match(stop, /stopConduitToken===_\?\{\.\.\.e,stopConduitToken:null\}:e/);
  assert.match(stop, /uie.abort\(t\)/);
  assert.deepEqual(modules.conversation.imports.get('_t'), { file: './' + profile.files.shared, name: 'CC' });
  assert.equal(definition('shared', 'zK'), 'function kw(){return mg(Ow)}');
  assert.match(definition('shared', 'Ow', true), /accountKey===Dw\(\)\?e.pin:null/);
  assert.match(definition('shared', 'pX'), /startsWith\(Rl\)/);
  assert.equal(definition('shared', 'Rl', true), '`WEB:`');
  assert.equal(definition('shared', 'QK'), 'function ow(){return mg(xw)}');
  assert.equal(definition('shared', 'xw', true), 'J(()=>aw(bw().data))');
  assert.match(definition('shared', 'aw', true), /accountItems.find\(t=>t.id===e.currentAccountId\)/);
  assert.equal(definition('conversation', 'YDt'), 'qr(()=>F1n())');
  assert.match(definition('conversation', 'F1n', true), /hydrateSessionFromLibrary/);
  assert.ok(definition('composer', profile.temporary.owner, true).includes(profile.temporary.action));
  assert.ok(modules.composer.definitions.has(profile.toolOwner));
  const history = definition('conversation', profile.expectedExports.conversation.textHydrateHistory);
  assert.match(history, /shouldApplyResponse:/);
  assert.match(history, /pendingDrafts.length>0/);
  const attachments = definition('conversation', profile.expectedExports.conversation.textSerializeAttachments);
  assert.match(attachments, /mountedLibraryFileId/);
  assert.match(attachments, /mounted_library_mime_type/);
});

test('second rollout Canvas assets preserve export and original restore contracts', {
  skip: !directory && 'Set CHATGPT_PUBLIC_RUNTIME_DIR to reviewed public assets.'
}, () => {
  const files = {
    'bc86e6a9-pb4piezqygn8qfrm.js': 'aa5b13f61464b06b6907fca0fb8194b523edcfc10b95e521e48449816609dc66',
    'e5d54aa7-pdcracz6gqmo2bwj.js': '2ddbea639a6c3402c7467d6896b7b2e21540fd0a4a199751238c579e971eef1f',
    '1c4de3ec-e4l31n3azu0cqixl.js': '6aae7c8d2a761f18db32e983874fe15e0ebd5535f669df982e2f3e18c61b22b1',
    '6afb0137-ol6cc2ldzctuty5j.js': '07194ed3ed02057920ca1d58295389704e70144c47879e9b16206480d86dfbad',
    'd3304073-i5jaxzkvtv1ju9nn.js': 'b74ab162699fbc867432c5223fb4621dad20b630301b9ab4f61fa1873f9b49a9',
    [profile.files.conversation]: hashes.conversation
  }, modules = {};
  for (const [file, hash] of Object.entries(files)) {
    const bytes = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), hash);
    modules[file] = parseSource(bytes.toString('utf8'));
  }
  const exporter = modules['bc86e6a9-pb4piezqygn8qfrm.js'].text;
  assert.match(exporter, /e.iNn\(\),e.aNn/);
  for (const key of ['CANVAS_REMARK_PLUGINS', 'stripDirectivePlugin', 'hiveLogDirectivePlugin']) assert.ok(exporter.includes(key));
  for (const [file, exports] of Object.entries({
    [profile.files.conversation]: ['iNn', 'aNn'], 'e5d54aa7-pdcracz6gqmo2bwj.js': ['n', 'r'],
    '1c4de3ec-e4l31n3azu0cqixl.js': ['t', 'r'], '6afb0137-ol6cc2ldzctuty5j.js': ['i', 'r'],
    'd3304073-i5jaxzkvtv1ju9nn.js': ['i', 'a']
  })) for (const name of exports) assert.ok(modules[file].exported.has(name), file + '.' + name);
  const m = modules[profile.files.conversation];
  const restores = [...m.definitions.values()].filter(nodes => nodes.length === 1).map(([n]) => m.text.slice(n.start, n.end))
    .filter(body => body.includes('safePost(`/textdoc/{textdoc_id}/restore`'));
  assert.equal(restores.length, 1);
  assert.match(restores[0], /textdocId:e,versionInt:t,restoreFromVersionInt:n/);
  assert.match(restores[0], /requestBody:\{version:t,restore_from_version:n\}/);
});
