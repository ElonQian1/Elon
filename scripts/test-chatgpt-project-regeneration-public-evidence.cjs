'use strict';

// Parse reviewed public assets, never import website code or read a user session.
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { parseSource } = require('./analyze-chatgpt-runtime-contracts.cjs');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;
const assets = {
  conversation: ['conversation-small-newrvr7nrx5tnmp4.js',
    '039efa3e391652942a9ae5de7cc057eb1bc05c3afad32d851e14ac47b554470d'],
  composer: ['8b34dbc2-duz5rkq42xhpbotn.js',
    '5693a37d5f6eecdecd6bb9e257380cc7594803aeafb9df87b5dbafdf4d03f605']
};

test('reviewed project retry carries its mode through fresh prepare and variant dispatch', {
  skip: !directory && 'Set CHATGPT_PUBLIC_RUNTIME_DIR to the second Sep 15 public assets.'
}, () => {
  const modules = {};
  for (const [role, [file, hash]] of Object.entries(assets)) {
    const bytes = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), hash);
    modules[role] = parseSource(bytes.toString('utf8'));
  }
  function one(role, predicate) {
    const m = modules[role], candidates = [];
    for (const nodes of m.definitions.values()) for (const node of nodes) {
      if (node.type !== 'FunctionDeclaration') continue;
      const text = m.text.slice(node.start, node.end);
      if (predicate(text)) candidates.push(text);
    }
    assert.equal(candidates.length, 1, role + ' contract must be unique');
    return candidates[0];
  }
  const retry = one('conversation', s => s.includes('Regeneration must have parent prompt'));
  assert.match(retry, /conversationMode:r/);
  const mode = retry.match(/\b([\w$]+)=r\?\?[\w$]+\(u\)\?\.mode/)?.[1];
  assert.ok(mode, 'retry uses the selected override or the conversation tree mode');
  assert.match(retry, /completionType:[\w$]+\.Variant/);
  assert.ok(retry.includes('completionMetadata:{variantPurpose:h?`comparison_implicit`:`none`,conversationMode:' + mode + ','));
  assert.match(retry, /parentMessageId:p.id/);
  assert.match(retry, /getParentPromptNode\(d,t\)/);
  const prepare = one('composer', s => s.includes('conduit_f_conversation_prepare_api_error'));
  const parent = prepare.match(/else if\(i.isRegen\)([\w$]+)=[\w$]+.getParentPromptNode\(s\)\?\.id/)?.[1];
  assert.ok(parent, 'prepare selects the original user parent for retry');
  assert.ok(prepare.includes('parentMessageId:' + parent + ','));
  assert.match(prepare, /conversationModeOverride\?\?s\?\.mode/);
  assert.match(prepare, /C=i.conversationModeOverride\?\?s\?\.mode/);
  assert.match(prepare, /w=y&&[\w$]+\([\w$]+\(C\)\)\?\{kind:[\w$]+.PrimaryAssistant\}:C/);
  assert.match(prepare, /completionType:[\w$]+\.Next,completionMetadata:\{systemHints:n,conversationMode:w\}/);
  assert.match(prepare, /messages:\[\]/);
  const serializer = one('composer', s => s.includes('variant_purpose:e.completionMetadata?.variantPurpose'));
  assert.match(serializer, /action:e.completionType/);
  assert.match(serializer, /conversation_mode:[\w$]+\(e.completionMetadata\?\.conversationMode\)/);
  assert.match(serializer, /e.messages.length>0\?e[\s\S]*:void 0/);
  assert.match(serializer, /parent_message_id:e.parentMessageId/);
});
