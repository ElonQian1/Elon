'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const root = path.resolve(__dirname, '..');
const source = fs.readFileSync(path.join(root, 'android/app/src/main/assets/chatgpt_web_command_delivery.js'), 'utf8');
const token = 'doc_delivery_fixture', href = 'https://chatgpt.com/c/fixture';
function fixture() {
  const page = { __elonChatGptDocumentToken: token, location: { href } };
  const call = vm.runInNewContext(source, { window: page });
  const commands = [];
  page.__elonChatGptBridge = { command(value) { commands.push(value); } };
  return { page, commands, run: () => call('{"action":"send_prompt"}', token, href) };
}
test('warm command enters exactly once without replacing the bridge', () => {
  const f = fixture(), bridge = f.page.__elonChatGptBridge;
  assert.equal(f.run(), 'entered');
  assert.equal(f.commands.length, 1);
  assert.equal(f.page.__elonChatGptBridge, bridge);
});
for (const value of [undefined, null, {}, { command: true }]) {
  test('missing callable bridge proves no invocation: ' + String(value), () => {
    const f = fixture(); f.page.__elonChatGptBridge = value;
    assert.equal(f.run(), 'missing'); assert.equal(f.commands.length, 0);
  });
}
for (const field of ['token', 'route']) {
  test(field + ' change refuses even an available bridge', () => {
    const f = fixture();
    if (field === 'token') f.page.__elonChatGptDocumentToken = 'doc_other';
    else f.page.location.href += '?temporary-chat=true';
    assert.equal(f.run(), 'stale'); assert.equal(f.commands.length, 0);
  });
}
test('a throw after entering a write is unknown, never missing', () => {
  const f = fixture(); let writes = 0;
  f.page.__elonChatGptBridge.command = () => { writes++; throw Error('synthetic'); };
  assert.equal(f.run(), 'unknown'); assert.equal(writes, 1);
});
test('a throwing bridge accessor cannot create a retryable proof', () => {
  const f = fixture();
  Object.defineProperty(f.page, '__elonChatGptBridge', { get() { throw Error('synthetic'); } });
  assert.equal(f.run(), 'unknown');
});
test('missing then explicit repair dispatches the original command once', () => {
  const f = fixture(), bridge = f.page.__elonChatGptBridge;
  delete f.page.__elonChatGptBridge;
  assert.equal(f.run(), 'missing'); assert.equal(f.commands.length, 0);
  f.page.__elonChatGptBridge = bridge;
  assert.equal(f.run(), 'entered'); assert.equal(f.commands.length, 1);
});
test('private text and read commands use the same guarded native boundary', () => {
  const adapter = fs.readFileSync(path.join(root, 'android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  assert.match(adapter, /commandDelivery\.send\(command\)/);
  assert.match(adapter, /fun onPageStarted\(url: String\) \{\s*commandDelivery\.invalidate\(\)/);
  assert.match(adapter, /fun onHostPaused\(\) \{\s*commandDelivery\.invalidate\(\)/);
  assert.doesNotMatch(adapter, /window\.__elonChatGptBridge && window\.__elonChatGptBridge\.command/);
  assert.match(adapter, /if \(!allowed\(\)\) result\(false\)/);
  assert.match(adapter, /JSONObject\.quote\(owner\.href\)/);
});
