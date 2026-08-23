'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_socket_tap.js'
), 'utf8');

assert.doesNotMatch(source, /document\.cookie|localStorage|sessionStorage|authorization/i);

class FakeWebSocket {
  constructor(url, protocols) {
    this.url = url;
    this.protocols = protocols;
    this.listeners = new Map();
    FakeWebSocket.created.push(this);
  }

  addEventListener(type, listener) {
    const values = this.listeners.get(type) || [];
    values.push(listener);
    this.listeners.set(type, values);
  }

  emit(type, data) {
    (this.listeners.get(type) || []).forEach((listener) => listener({ data }));
  }

  send(value) { this.sent = value; }
}
FakeWebSocket.created = [];
FakeWebSocket.CONNECTING = 0;
FakeWebSocket.OPEN = 1;
FakeWebSocket.CLOSING = 2;
FakeWebSocket.CLOSED = 3;

const window = {
  __elonChatGptPrivateStreamObserverEnabled: true,
  WebSocket: FakeWebSocket
};
window.window = window;
const sandbox = {
  window,
  location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' },
  URL,
  Set,
  Object,
  String,
  ArrayBuffer,
  TextDecoder,
  Blob
};
vm.runInNewContext(source, sandbox, { filename: 'chatgpt_web_private_socket_tap.js' });

assert.equal(window.__elonChatGptPrivateSocketTap.version, 1);
assert.notEqual(window.WebSocket, FakeWebSocket);
const observed = [];
const unsubscribe = window.__elonChatGptPrivateSocketTap.subscribe((value) => observed.push(value));
const official = new window.WebSocket('wss://ws.chatgpt.com/backend-api/ws', ['json']);
assert.equal(FakeWebSocket.created.length, 1, 'the page still owns one official socket');
official.send('official-outbound');
assert.equal(official.sent, 'official-outbound', 'outbound data remains untouched');
official.emit('message', '{"type":"assistant"}');
official.emit('message', new TextEncoder().encode('{"type":"binary"}').buffer);

const unrelated = new window.WebSocket('wss://example.com/socket');
unrelated.emit('message', '{"type":"ignored"}');
assert.deepEqual(observed, ['{"type":"assistant"}', '{"type":"binary"}']);

for (let index = 0; index < 40; index += 1) {
  official.emit('message', JSON.stringify({ index }));
}
assert.equal(window.__elonChatGptPrivateSocketTap.bufferedCount(), 24);
unsubscribe();
official.emit('message', '{"type":"after-unsubscribe"}');
assert.equal(observed.length, 42);

window.__elonChatGptPrivateSocketTap.dispose();
assert.equal(window.WebSocket, FakeWebSocket);
assert.equal(window.__elonChatGptPrivateSocketTap.bufferedCount(), 0);

console.log('CHATGPT_WEB_PRIVATE_SOCKET_TAP_TESTS=passed');
