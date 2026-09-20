'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { parseSource, roleFiles, compareSymbol } = require('./analyze-chatgpt-runtime-contracts.cjs');
const prior = require('./fixtures/chatgpt-runtime-bindings-sep15b.cjs');
const profile = require('./fixtures/chatgpt-runtime-bindings-sep21.cjs');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;
const previous = process.env.CHATGPT_PRIOR_RUNTIME_DIR;

test('Sep 21 exposes the reviewed exports, temporary owner and tools, never an unknown revision', async () => {
  const bindings = require('../android/app/src/main/assets/chatgpt_web_private_runtime_bindings');
  const cdn = 'https://chatgpt.com/cdn/assets/';
  let observed = profile.anchor;
  const page = { location: { origin: 'https://chatgpt.com' }, document: {}, setTimeout, clearTimeout,
    __elonChatGptDocumentToken: 'doc_fixture', performance: { getEntriesByName: url => url === cdn + observed ? [{}] : [] } };
  const namespaces = Object.fromEntries(Object.entries(profile.expectedExports).map(([role, map]) =>
    [role, Object.fromEntries(Object.entries(map).map(([name, exported]) => [exported, { name }]))]));
  const runtime = bindings.create(page, { loadRuntime: async url => namespaces[
    Object.keys(profile.files).find(role => cdn + profile.files[role] === url)] });
  let count = 0;
  for (const [role, map] of Object.entries(profile.expectedExports)) {
    const actual = await runtime.load(role);
    assert.deepEqual(Object.keys(actual).sort(), Object.keys(map).sort());
    for (const [name, exported] of Object.entries(map)) {
      assert.equal(actual[name], namespaces[role][exported]); count++;
    }
  }
  assert.equal(count, 100);
  assert.equal(runtime.state().profile_id, profile.id);
  assert.deepEqual(runtime.temporary(), profile.temporary);
  assert.deepEqual(runtime.tools(), { owner: profile.toolOwner });
  namespaces.react.Mn = { initialized: true };
  assert.equal(runtime.peek('react').intlProvider, namespaces.react.Mn);
  observed = 'c2675c8c-unknown.js'; page.__elonChatGptDocumentToken = 'doc_next';
  await assert.rejects(runtime.load('shared'), /runtime_not_observed/);
});

test('hash-pinned Sep 21 sources retain send/security contracts and resolve ambiguous aliases by dependencies', {
  skip: !(directory && previous) && 'Set both reviewed public runtime directories.'
}, () => {
  const current = {}, old = {};
  for (const [role, file] of Object.entries({ anchor: profile.anchor, ...profile.files })) {
    const bytes = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), profile.hashes[role]);
    current[role] = parseSource(bytes.toString('utf8'));
    if (prior.files[role]) old[role] = parseSource(fs.readFileSync(path.join(previous, prior.files[role]), 'utf8'));
  }
  assert.deepEqual(roleFiles(current.anchor.sources, true), profile.files);
  const body = (modules, role, name, local = false) => {
    const m = modules[role], nodes = m.definitions.get(local ? name : m.exported.get(name));
    assert.ok(nodes?.length, role + '.' + name);
    return m.text.slice(nodes[0].start, nodes[0].end);
  };
  const definition = (role, name, local = false) => body(current, role, name, local);
  const changed = new Set(['shared.textBusinessContext', 'conversation.J5t', 'conversation.p8t',
    'conversation.textHydrateHistory', 'conversation.textSerializeAttachments', 'composer.fh']);
  for (const [role, mappings] of Object.entries(profile.expectedExports)) {
    for (const [name, exported] of Object.entries(mappings)) {
      assert.ok(current[role].exported.has(exported), role + '.' + name);
      if (role === 'react') {
        // The parser also indexes nested assignments to short identifiers.
        // Their first definition is the actual top-level/lazy module binding.
        assert.equal(definition(role, exported), body(old, role, prior.expectedExports[role][name]));
      } else if (!changed.has(role + '.' + name)) {
        assert.ok(compareSymbol(old[role], current[role], prior.expectedExports[role][name])
          .candidates.some(candidate => candidate.exports.includes(exported)), role + '.' + name);
      }
    }
  }
  assert.match(definition('shared', 'n4'), /return Ib\(\).batch\(e\)/);
  assert.deepEqual(current.conversation.imports.get('fd'), { file: './' + profile.files.shared, name: 'vS' });
  assert.match(definition('conversation', 'OJt'), /fd\(c\)/);
  assert.equal(definition('shared', 'yK'), 'function Gk(){return J(Wk)}');
  assert.match(definition('shared', 'Wk', true), /accountKey===Uk\(\)\?e.pin:null/);
  assert.equal(definition('shared', 'dX'), 'function QE(e){return e.startsWith(qm)}');
  assert.equal(definition('shared', 'qm', true), '`WEB:`');
  assert.equal(definition('shared', 'MK'), 'function bk(){return J(Lk)}');
  assert.equal(definition('shared', 'Lk', true), 'X(()=>yk(Ik().data))');
  assert.match(definition('shared', 'yk', true), /accountItems.find\(t=>t.id===e.currentAccountId\)/);
  assert.equal(definition('conversation', 'hjt'), 'Ri(()=>z1n())');
  assert.match(definition('conversation', 'z1n', true), /hydrateSessionFromLibrary/);
  // Changed functions preserve the consumed boundary; additions are provider-owned.
  assert.match(definition('shared', 'LR'), /persistedContext:e,messages:t,turns:n,gizmoId:r,conversationId:i,gizmoDisplay:a/);
  assert.match(definition('shared', 'LR'), /return r\?\.trim\(\)\?bF\(r,i\):null/);
  assert.match(definition('conversation', 'kcn'), /dweb_conversation_share_sheet_variant/);
  assert.match(definition('conversation', 'Kon'), /status:`rate_limited`,resetsAfter:i.resetsAfter/);
  const history = definition('conversation', 'bPn');
  assert.match(history, /shouldApplyResponse:l/);
  assert.match(history, /if\(l\?\.\(\)===!1\)return C\?\.finished\(`aborted`\)/);
  assert.match(history, /pendingDrafts.length>0/);
  const attachments = definition('conversation', 'w_t');
  assert.match(attachments, /mounted_library_mime_type/);
  assert.match(attachments, /duration_seconds:n.audioDurationSeconds/);
  assert.match(definition('composer', 'rg'), /reset:e=>/);
  assert.match(definition('composer', 'rg'), /e.files\$\.set\(\[\]\)/);
  assert.ok(definition('composer', profile.temporary.owner, true).includes(profile.temporary.action));
  assert.match(definition('composer', profile.toolOwner, true), /conversation:n,isNewThread:r,composerController:i/);
  assert.match(definition('composer', profile.toolOwner, true), /availableSystemHints:_,activeSystemHintType:v/);
});
