'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const read = relative => fs.readFileSync(path.join(root, relative), 'utf8');
const kotlin = 'android/app/src/main/kotlin/com/elon/app/';

test('production snapshot triggers the registered read session independently of private input readiness', () => {
  const manifest = read(kotlin + 'chatgptweb/ChatGptWebAdapterAssets.kt');
  const modules = ['chatgpt_web_fresh_text_recovery_context.js', 'chatgpt_web_fresh_text_recovery_session.js',
    'chatgpt_web_fresh_text_transaction.js'];
  const indices = modules.map(name => manifest.indexOf('"' + name + '"'));
  assert.ok(indices.every(index => index > 0)); assert.ok(indices[0] < indices[1] && indices[1] < indices[2]);
  const adapter = read('android/app/src/main/assets/chatgpt_web_adapter.js');
  assert.match(adapter, /addRecoverySnapshotFields\?\.\(event, \(\) => scheduleSnapshot\(true\)\)/);
  assert.ok(adapter.indexOf('addRecoverySnapshotFields') < adapter.indexOf('const fingerprint = JSON.stringify(event)'));
  assert.match(read('android/app/src/main/assets/chatgpt_web_fresh_text_transaction.js'),
    /event\.privateSendRecoveryState = pendingRecoverySnapshot\(onChange\)/);
});

test('canonical snapshot state reaches the existing production status banner without a fake message', () => {
  assert.match(read(kotlin + 'chatgptweb/ChatGptWebProtocol.kt'), /privateSendRecoveryState = ChatGptWebPendingWriteRecoveryState\.parse/);
  assert.match(read(kotlin + 'ChatGptSocialChatController.kt'), /override fun pendingWriteRecoveryState\(\): String =\s*session\.currentSnapshot\(\)\?\.privateSendRecoveryState/);
  assert.match(read(kotlin + 'WebChatConsumerStatusBanner.kt'), /pendingWriteRecovery = pendingWriteRecoveryState\(\)/);
  assert.match(read(kotlin + 'MainSocialAiChatFeature.kt'), /val recovery = controller\.consumerRecoveryState\(provider\)/);
  assert.doesNotMatch(read(kotlin + 'WebChatPendingWriteRecoveryPresentation.kt'), /ChatMessage|Toast|Dialog|reload/);
});

test('wire-only recovery status is not persisted as a connection or history cache fact', () => {
  for (const name of ['WebChatSnapshotStore.kt', 'ChatGptConversationSnapshotStore.kt']) {
    assert.doesNotMatch(read(kotlin + 'chatgptweb/' + name), /privateSendRecoveryState/);
  }
});
