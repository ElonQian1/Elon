'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { parseSource, roleFiles, compareSymbol } = require('./analyze-chatgpt-runtime-contracts.cjs');
const prior = require('./fixtures/chatgpt-runtime-bindings-sep21.cjs');
const profile = process.env.CHATGPT_RUNTIME_ROLLOUT === 'sep22b'
  ? require('./fixtures/chatgpt-runtime-bindings-sep22b.cjs')
  : require('./fixtures/chatgpt-runtime-bindings-sep22.cjs');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR, previous = process.env.CHATGPT_PRIOR_RUNTIME_DIR;

test(profile.id + ' exposes all reviewed live exports and rejects unknown documents', async () => {
  const bindings = require('../android/app/src/main/assets/chatgpt_web_private_runtime_bindings');
  const cdn = 'https://chatgpt.com/cdn/assets/';
  let observed = profile.anchor;
  const page = { location: { origin: 'https://chatgpt.com' }, document: {}, setTimeout, clearTimeout,
    __elonChatGptDocumentToken: 'doc_fixture', performance: { getEntriesByName: u => u === cdn + observed ? [{}] : [] } };
  const namespaces = Object.fromEntries(Object.entries(profile.expectedExports).map(([role, map]) =>
    [role, Object.fromEntries(Object.entries(map).map(([name, exported]) => [exported, { name }]))]));
  const runtime = bindings.create(page, { loadRuntime: async url => namespaces[
    Object.keys(profile.files).find(role => cdn + profile.files[role] === url)] });
  let count = 0;
  for (const [role, map] of Object.entries(profile.expectedExports)) {
    const actual = await runtime.load(role);
    assert.deepEqual(Object.keys(actual).sort(), Object.keys(map).sort());
    for (const [name, exported] of Object.entries(map)) { assert.equal(actual[name], namespaces[role][exported]); count++; }
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

test(profile.id + ' public-source contracts and ambiguous dependencies are pinned, not guessed', {
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
  const body = (role, name, local = false) => {
    const m = current[role], nodes = m.definitions.get(local ? name : m.exported.get(name));
    assert.ok(nodes?.length, role + '.' + name);
    return m.text.slice(nodes[0].start, nodes[0].end);
  };
  for (const [role, mappings] of Object.entries(profile.expectedExports)) {
    for (const [name, exported] of Object.entries(mappings)) {
      assert.ok(current[role].exported.has(exported), role + '.' + name);
      if (role === 'react') assert.equal(profile.hashes.react, prior.hashes.react);
      else if (role + '.' + name !== 'composer.fh') assert.ok(
        compareSymbol(old[role], current[role], prior.expectedExports[role][name])
          .candidates.some(candidate => candidate.exports.includes(exported)), role + '.' + name);
    }
  }
  assert.equal(body('shared', 'C0'), 'function pb(e){return cb().batch(e)}');
  assert.match(body('shared', 'cb', true), /concurrent/);
  assert.deepEqual(current.conversation.imports.get('rl'), { file: './' + profile.files.shared, name: 'kb' });
  assert.match(body('conversation', 'YJt'), /rl\(c\)/);
  assert.equal(body('shared', 'zW'), 'function kD(){return J(OD)}');
  assert.match(body('shared', 'OD', true), /accountKey===DD\(\)\?e.pin:null/);
  assert.equal(body('shared', 'MJ'), 'function Xw(e){return e.startsWith(Fp)}');
  assert.equal(body('shared', 'Fp', true), '`WEB:`');
  assert.equal(body('shared', 'QW'), 'function rD(){return J(xD)}');
  assert.equal(body('shared', 'xD', true), 'X(()=>nD(bD().data))');
  assert.match(body('shared', 'nD', true), /accountItems.find\(t=>t.id===e.currentAccountId\)/);
  assert.equal(body('conversation', 'kjt'), 'Sd(()=>Q0n())');
  assert.match(body('conversation', 'Q0n', true), /hydrateSessionFromLibrary/);
  const attachments = body('composer', 'tg');
  assert.match(attachments, /reset:e=>/);
  assert.match(attachments, /e.files\$\.set\(\[\]\)/);
  assert.match(attachments, /e.skipChatAttachmentLimits/);
  assert.ok(body('composer', profile.temporary.owner, true).includes(profile.temporary.action));
  assert.match(body('composer', profile.toolOwner, true), /conversation:n,isNewThread:r,composerController:i/);
});
