'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { parseSource, roleFiles } = require('./analyze-chatgpt-runtime-contracts.cjs');
const profile = require('./fixtures/chatgpt-runtime-bindings-sep12');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;
const hashes = {
  anchor: '626dec60a232340efeac2f154db07232740de506b221abf5fafe0e71d4ab03a2',
  shared: '6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e',
  conversation: 'da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e',
  composer: '1d0b132fe9b13120370395bbfbe4bf3324c4dc1213b80e591cd3c6a6db7d16a1'
};

test('reviewed public modules contain actual exports, not coincidentally named imports', {
  skip: !directory && 'Set CHATGPT_PUBLIC_RUNTIME_DIR to the reviewed public assets; no network in tests.'
}, () => {
  const parsed = {};
  for (const [role, file] of Object.entries({ anchor: profile.anchor, ...profile.files })) {
    const source = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(source).digest('hex'), hashes[role]);
    // Parse only. Importing public modules here could initialize unrelated website code.
    parsed[role] = parseSource(source.toString('utf8'));
  }
  assert.deepEqual(roleFiles(parsed.anchor.sources), profile.files);
  for (const role of Object.keys(profile.files)) {
    for (const [canonical, exported] of Object.entries({ ...profile.expectedExports[role], ...profile.extraExports[role] })) {
      assert.ok(parsed[role].exported.has(exported), role + '.' + canonical + ' must be exported');
    }
  }
  const conversation = parsed.conversation;
  assert.equal(conversation.exported.has('one'), false);
  assert.equal(conversation.imports.get('one').file, './' + profile.files.shared);
  assert.equal(conversation.exported.has('dir'), false);
  assert.equal(profile.extraExports.conversation.canvasEdits, undefined);
  const stop = conversation.definitions.get(conversation.exported.get(profile.expectedExports.conversation.FVt))[0];
  const body = conversation.text.slice(stop.start, stop.end);
  assert.match(body, /Uo\(c\)/);
  assert.deepEqual(conversation.imports.get('Uo'), { file: './' + profile.files.shared, name: 'aC' });
  assert.deepEqual(conversation.imports.get('yn'), { file: './' + profile.files.shared, name: 'GJ' });
  assert.deepEqual(conversation.imports.get('au'), { file: './' + profile.files.shared, name: 'rtt' });
  assert.match(body, /safePost\(`\/stop_conversation`/);
  assert.ok(parsed.composer.definitions.has(profile.toolOwner));
  const temporary = parsed.composer.definitions.get(profile.temporary.owner)[0];
  assert.ok(parsed.composer.text.slice(temporary.start, temporary.end).includes(profile.temporary.action));
});
