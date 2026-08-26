const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.resolve(__dirname, '..');
const source = fs.readFileSync(path.join(
  root,
  'desktop-shell/src-tauri/src/local_ai_browser/google_win_private_reply_state.js'
), 'utf8');

let reply = null;
let observedPrompt = '';
let listener = null;
const emitted = [];
const baseObserver = {
  version: 5,
  observePrompt(value) { observedPrompt = String(value); reply = null; },
  snapshot() { return reply; },
  diagnostics() { return 'base-diagnostics'; },
  setListener(value) { listener = value; },
};
const baseNative = {
  postMessage(raw) { emitted.push(JSON.parse(String(raw))); },
};
const window = {
  __elonGoogleWebPrivateReplyObserver: baseObserver,
  elonGoogleWebNative: baseNative,
};

vm.runInNewContext(source, { window, JSON, String, Number, Object });

assert.equal(window.__elonWinGooglePrivateReplyStateVersion, 2);
assert.notEqual(window.__elonGoogleWebPrivateReplyObserver, baseObserver);
assert.notEqual(window.elonGoogleWebNative, baseNative);
const installedObserver = window.__elonGoogleWebPrivateReplyObserver;
const installedNative = window.elonGoogleWebNative;

window.__elonGoogleWebPrivateReplyObserver.observePrompt('BTC 走势');
assert.equal(observedPrompt, 'BTC 走势');
assert.equal(window.__elonWinGooglePrivateReplyState.snapshot().state, 'idle');

reply = { prompt: 'BTC 走势', text: '正在回答', streaming: true };
window.elonGoogleWebNative.postMessage(envelope(true));
let event = emitted.pop().event;
assert.equal(event.privateStreamObserved, true);
assert.equal(event.privateStreamRevision, 1);
assert.equal(event.privateStreamState, 'streaming');
assert.equal(event.streaming, true);

window.elonGoogleWebNative.postMessage(envelope(true));
event = emitted.pop().event;
assert.equal(event.privateStreamRevision, 1, 'unchanged private text must not advance revision');

reply = { prompt: 'BTC 走势', text: '回答完成', streaming: false };
window.elonGoogleWebNative.postMessage(envelope(true));
event = emitted.pop().event;
assert.equal(event.privateStreamRevision, 2);
assert.equal(event.privateStreamState, 'completed');
assert.equal(event.streaming, false, 'private completion must override stale DOM streaming');

window.__elonWinGooglePrivateReplyState.reset();
window.elonGoogleWebNative.postMessage(envelope(false));
event = emitted.pop().event;
assert.equal(event.privateStreamObserved, false);
assert.equal(event.privateStreamState, 'idle');
assert.equal(event.privateStreamRevision, 2);

window.__elonGoogleWebPrivateReplyObserver.setListener(() => {});
assert.equal(typeof listener, 'function');
assert.equal(window.__elonGoogleWebPrivateReplyObserver.diagnostics(), 'base-diagnostics');

let reboundPrompt = '';
let reboundReply = null;
let reboundListener = null;
const reboundEmitted = [];
const reboundObserver = {
  version: 6,
  observePrompt(value) { reboundPrompt = String(value); reboundReply = null; },
  snapshot() { return reboundReply; },
  diagnostics() { return 'rebound-diagnostics'; },
  setListener(value) { reboundListener = value; },
};
const reboundNative = {
  postMessage(raw) { reboundEmitted.push(JSON.parse(String(raw))); },
};
window.__elonGoogleWebPrivateReplyObserver = reboundObserver;
window.elonGoogleWebNative = reboundNative;
vm.runInNewContext(source, { window, JSON, String, Number, Object });

assert.equal(window.__elonGoogleWebPrivateReplyObserver, installedObserver);
assert.equal(window.elonGoogleWebNative, installedNative);
assert.equal(window.__elonGoogleWebPrivateReplyObserver.version, 6);
assert.equal(reboundListener, listener, 'adapter listener must follow a replacement observer');
assert.equal(
  window.__elonWinGooglePrivateReplyState.diagnostics(),
  'v2|bindings=2|state=idle',
);
window.__elonGoogleWebPrivateReplyObserver.observePrompt('ETH 走势');
assert.equal(reboundPrompt, 'ETH 走势');
reboundReply = { prompt: 'ETH 走势', text: '新观察器回答', streaming: false };
window.elonGoogleWebNative.postMessage(envelope(true));
event = reboundEmitted.pop().event;
assert.equal(event.privateStreamObserved, true);
assert.equal(event.privateStreamState, 'completed');
assert.equal(event.streaming, false);
assert.equal(emitted.length, 0, 'rebound events must not leak to the stale native bridge');
assert.doesNotMatch(source, /document\.cookie|authorization|access[_-]?token|fetch\s*\(/i);

function envelope(streaming) {
  return JSON.stringify({
    schema: 'yilong.ai.ui.v1',
    providerId: 'google_web',
    event: { type: 'message_snapshot', streaming },
  });
}

process.stdout.write('PASS Google Win private reply state\n');
