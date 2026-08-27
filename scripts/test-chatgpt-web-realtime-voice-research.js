'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.join(__dirname, '..');
const source = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_realtime_voice_research.js'
), 'utf8');
const adapter = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'kotlin', 'com', 'elon', 'app',
  'chatgptweb', 'ChatGptWebPageAdapter.kt'
), 'utf8');
const recorder = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'kotlin', 'com', 'elon', 'app',
  'chatgptweb', 'ChatGptWebPrivateResearchEventRecorder.kt'
), 'utf8');

assert.match(adapter, /PRIVATE_REALTIME_VOICE_RESEARCH_ASSET/);
assert.match(adapter, /addDocumentStartJavaScript\([\s\S]*?privateRealtimeVoiceResearchScript/);
assert.match(adapter, /"chatgpt_web_realtime_voice_research\.js"/);
assert.match(recorder, /ChatGptWebRealtimeVoiceResearchObservation\.parse\(event\.detail\)/);
assert.match(recorder, /chatgpt_private_voice_research_observation/);

class FakePeerConnection {
  constructor() {
    this.listeners = new Map();
    this.connectionState = 'new';
    this.iceConnectionState = 'new';
    this.signalingState = 'stable';
  }
  addEventListener(name, callback) { this.listeners.set(name, callback); }
  dispatch(name, event = {}) { const callback = this.listeners.get(name); if (callback) callback(event); }
  createOffer() { return Promise.resolve({ type: 'offer', sdp: 'must-not-cross-bridge' }); }
  createAnswer() { return Promise.resolve({ type: 'answer', sdp: 'must-not-cross-bridge' }); }
  setLocalDescription() { return Promise.resolve(); }
  setRemoteDescription() { return Promise.resolve(); }
  createDataChannel() { return { readyState: 'open' }; }
}

class FakeXhr {
  constructor() { this.listeners = new Map(); this.status = 200; }
  addEventListener(name, callback) { this.listeners.set(name, callback); }
  open() {}
  send() { const callback = this.listeners.get('loadend'); if (callback) callback(); }
  getResponseHeader(name) { return name === 'content-type' ? 'application/json' : null; }
}

class FakeSocket {
  constructor() { this.listeners = new Map(); }
  addEventListener(name, callback) { this.listeners.set(name, callback); }
  dispatch(name) { const callback = this.listeners.get(name); if (callback) callback(); }
}

function fakeResponse() {
  return {
    status: 200,
    headers: { get: (name) => name === 'content-type' ? 'application/json' : null },
    clone: () => ({
      json: async () => ({
        client_secret: 'must-not-cross-bridge',
        expires_at: 123456,
        session: { id: 'must-not-cross-bridge' }
      })
    })
  };
}

async function run(enabled) {
  const events = [];
  const mediaDevices = {
    getUserMedia: async () => ({
      getTracks: () => [{ kind: 'audio', id: 'must-not-cross-bridge' }]
    })
  };
  const window = {
    __elonChatGptPrivateResearchEnabled: enabled,
    __elonChatGptAdapterTargetVersion: 189,
    __elonChatGptDocumentToken: 'doc_voice_research',
    elonChatGptNative: { postMessage: (payload) => events.push(JSON.parse(payload)) },
    fetch: async () => fakeResponse(),
    XMLHttpRequest: FakeXhr,
    WebSocket: FakeSocket,
    RTCPeerConnection: FakePeerConnection,
    navigator: { mediaDevices }
  };
  window.window = window;
  const context = {
    window,
    navigator: window.navigator,
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' },
    URL,
    Object,
    Array,
    Promise,
    Date,
    Math,
    Number,
    String,
    JSON,
    WeakMap
  };
  vm.runInNewContext(source, context, { filename: 'chatgpt_web_realtime_voice_research.js' });
  if (!enabled) return { events, window };

  await window.fetch('https://chatgpt.com/backend-api/realtime/bootstrap?token=must-not-cross-bridge', {
    method: 'POST',
    headers: { Authorization: 'must-not-cross-bridge' },
    body: 'must-not-cross-bridge'
  });
  await new Promise((resolve) => setImmediate(resolve));
  window.__elonChatGptRealtimeVoiceResearch.activate();
  const xhr = new window.XMLHttpRequest();
  xhr.open('POST', 'https://chatgpt.com/backend-api/session/bootstrap?secret=ignored');
  xhr.send('must-not-cross-bridge');
  const socket = new window.WebSocket('wss://ws.chatgpt.com/backend-api/session?token=ignored');
  socket.dispatch('open');
  socket.dispatch('close');
  await window.navigator.mediaDevices.getUserMedia({ audio: true });
  const peer = new window.RTCPeerConnection({ iceServers: [{ urls: 'must-not-cross-bridge' }] });
  await peer.createOffer();
  await peer.setLocalDescription({ type: 'offer', sdp: 'must-not-cross-bridge' });
  await peer.setRemoteDescription({ type: 'answer', sdp: 'must-not-cross-bridge' });
  peer.createDataChannel('must-not-cross-bridge');
  peer.connectionState = 'connected';
  peer.dispatch('connectionstatechange');
  peer.iceConnectionState = 'connected';
  peer.dispatch('iceconnectionstatechange');
  peer.dispatch('track', { track: { kind: 'audio', id: 'must-not-cross-bridge' } });
  return { events, window };
}

(async () => {
  const disabled = await run(false);
  assert.equal(disabled.events.length, 0);
  assert.equal(disabled.window.__elonChatGptRealtimeVoiceResearch, undefined);

  const enabled = await run(true);
  const details = enabled.events.map((event) => event.detail);
  assert.ok(details.includes('v1|observer-ready'));
  assert.ok(details.some((value) => value.startsWith(
    'v1|network-start|post|chatgpt_origin|/backend-api/realtime/bootstrap'
  )));
  assert.ok(details.some((value) => value.includes('|network-shape|')));
  assert.ok(details.some((value) => value.startsWith(
    'v1|network-start|post|chatgpt_origin|/backend-api/session/bootstrap'
  )));
  assert.ok(details.includes('v1|socket-start|chatgpt_subdomain|/backend-api/session'));
  assert.ok(details.includes('v1|socket-open|chatgpt_subdomain|/backend-api/session'));
  assert.ok(details.includes('v1|socket-close|chatgpt_subdomain|/backend-api/session'));
  assert.ok(details.includes('v1|media-request|audio'));
  assert.ok(details.includes('v1|media-granted|a1v0'));
  assert.ok(details.includes('v1|peer-created'));
  assert.ok(details.includes('v1|peer-create-offer'));
  assert.ok(details.some((value) => value.startsWith('v1|peer-local-description|offer|')));
  assert.ok(details.some((value) => value.startsWith('v1|peer-remote-description|answer|')));
  assert.ok(details.includes('v1|peer-connection|connected'));
  assert.ok(details.includes('v1|peer-ice|connected'));
  assert.ok(details.includes('v1|peer-track|audio'));
  assert.ok(enabled.events.every((event) => event.action === 'research_voice_observation'));

  const emitted = JSON.stringify(enabled.events).toLowerCase();
  assert.doesNotMatch(emitted, /must-not-cross-bridge|client_secret|authorization|cookie/);
  assert.doesNotMatch(emitted, /\btoken\b|\bsdp\b|\bcandidate\b/);
  assert.ok(enabled.events.every((event) => event.detail.length <= 160));
  console.log('CHATGPT_WEB_REALTIME_VOICE_RESEARCH_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
